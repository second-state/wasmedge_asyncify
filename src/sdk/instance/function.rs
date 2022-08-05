use std::{
    ffi::c_void,
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

use wasmedge_sys::ffi;
use wasmedge_types::{
    error::{FuncError, WasmEdgeError},
    ValType, WasmEdgeResult,
};

use crate::{
    core::{
        instance::function::{FuncType, Function, InnerFunc},
        types::WasmVal,
    },
    sdk::linker::AsyncLinker,
};

pub use crate::core::instance::function::FuncRef;

pub type ResultFuture<'a> = Box<dyn Future<Output = WasmEdgeResult<Vec<WasmVal>>> + 'a>;

pub struct WasmEdgeResultFuture<'a> {
    pub(crate) linker: &'a mut AsyncLinker,
    pub(crate) name: String,
    pub(crate) args: Vec<WasmVal>,
}

impl Future for WasmEdgeResultFuture<'_> {
    type Output = WasmEdgeResult<Vec<WasmVal>>;

    fn poll(self: std::pin::Pin<&mut Self>, cx: &mut Context<'_>) -> std::task::Poll<Self::Output> {
        let WasmEdgeResultFuture { linker, name, args } = self.get_mut();
        linker.cx = cx.waker().clone();

        if let Err(e) = linker.asyncify_resume() {
            return Poll::Ready(Err(e));
        }

        match linker.real_call(name, args) {
            Ok(v) => match linker.asyncify_done() {
                Ok(true) => Poll::Ready(Ok(v)),
                Ok(false) => Poll::Pending,
                Err(e) => return Poll::Ready(Err(e)),
            },
            Err(e) => {
                if let Err(asyncify_normal_err) = linker.asyncify_normal() {
                    Poll::Ready(Err(asyncify_normal_err))
                } else {
                    let e = linker.vm_err.take().unwrap_or(e);
                    Poll::Ready(Err(e))
                }
            }
        }
    }
}

type FnWrapper = extern "C" fn(
    key_ptr: *mut c_void,
    data_ptr: *mut c_void,
    _mem_ctx: *mut ffi::WasmEdge_MemoryInstanceContext,
    params: *const ffi::WasmEdge_Value,
    param_len: u32,
    returns: *mut ffi::WasmEdge_Value,
    return_len: u32,
) -> ffi::WasmEdge_Result;

pub(crate) extern "C" fn wrapper_async_fn(
    key_ptr: *mut c_void,
    data_ptr: *mut c_void,
    _mem_ctx: *mut ffi::WasmEdge_MemoryInstanceContext,
    params: *const ffi::WasmEdge_Value,
    param_len: u32,
    returns: *mut ffi::WasmEdge_Value,
    return_len: u32,
) -> ffi::WasmEdge_Result {
    if let Some(data) = unsafe { (data_ptr as *mut AsyncLinker).as_mut() } {
        let mut cous = || -> WasmEdgeResult<ffi::WasmEdge_Result> {
            let linker = unsafe { (data_ptr as *mut AsyncLinker).as_mut().unwrap() };

            let cx = data.cx.clone();
            let mut cx = Context::from_waker(&cx);
            let fut_is_ready;
            let r = {
                let fut = if data.asyncify_done()? {
                    let real_fn: fn(&mut AsyncLinker, Vec<WasmVal>) -> ResultFuture =
                        unsafe { std::mem::transmute(key_ptr) };

                    let input = {
                        let raw_input =
                            unsafe { std::slice::from_raw_parts(params, param_len as usize) };
                        raw_input
                            .iter()
                            .map(|r| (*r).into())
                            .collect::<Vec<WasmVal>>()
                    };

                    Some(Pin::from(real_fn(linker, input)))
                } else {
                    linker.func_futures().pop_back()
                };

                if fut.is_none() {
                    return Ok(ffi::WasmEdge_Result { Code: 0x89 });
                }

                let mut fut = fut.unwrap();

                let return_len = return_len as usize;
                let raw_returns = unsafe { std::slice::from_raw_parts_mut(returns, return_len) };

                match Future::poll(fut.as_mut(), &mut cx) {
                    std::task::Poll::Ready(result) => {
                        fut_is_ready = true;
                        match result {
                            Ok(v) => {
                                assert!(v.len() == return_len);
                                for (idx, item) in v.into_iter().enumerate() {
                                    raw_returns[idx] = item.into();
                                }
                                ffi::WasmEdge_Result { Code: 0 }
                            }
                            Err(e) => {
                                let _ = data.vm_err.insert(e);
                                ffi::WasmEdge_Result { Code: 0x89 }
                            }
                        }
                    }
                    std::task::Poll::Pending => {
                        fut_is_ready = false;
                        data.func_futures().push_back(fut);
                        ffi::WasmEdge_Result { Code: 0 }
                    }
                }
            };

            if fut_is_ready {
                data.asyncify_normal()?;
            } else {
                data.asyncify_yield()?;
            };
            Ok(r)
        };
        match cous() {
            Ok(r) => r,
            Err(e) => {
                let _ = data.vm_err.insert(e);
                ffi::WasmEdge_Result { Code: 0x89 }
            }
        }
    } else {
        // unreachable
        ffi::WasmEdge_Result { Code: 0x89 }
    }
}

pub extern "C" fn wrapper_fn(
    key_ptr: *mut c_void,
    data: *mut c_void,
    _mem_ctx: *mut ffi::WasmEdge_MemoryInstanceContext,
    params: *const ffi::WasmEdge_Value,
    param_len: u32,
    returns: *mut ffi::WasmEdge_Value,
    return_len: u32,
) -> ffi::WasmEdge_Result {
    let real_fn: fn(&mut AsyncLinker, &[WasmVal]) -> WasmEdgeResult<Vec<WasmVal>> =
        unsafe { std::mem::transmute(key_ptr) };

    let input = {
        let raw_input = unsafe { std::slice::from_raw_parts(params, param_len as usize) };
        raw_input
            .iter()
            .map(|r| (*r).into())
            .collect::<Vec<WasmVal>>()
    };

    let return_len = return_len as usize;
    let raw_returns = unsafe { std::slice::from_raw_parts_mut(returns, return_len) };

    if let Some(data) = unsafe { (data as *mut AsyncLinker).as_mut() } {
        let result = real_fn(data, &input);

        match result {
            Ok(v) => {
                assert!(v.len() == return_len);
                for (idx, item) in v.into_iter().enumerate() {
                    raw_returns[idx] = item.into();
                }
                ffi::WasmEdge_Result { Code: 0 }
            }
            Err(e) => {
                let _ = data.vm_err.insert(e);
                ffi::WasmEdge_Result { Code: 0x89 }
            }
        }
    } else {
        ffi::WasmEdge_Result { Code: 0x89 }
    }
}

impl Function {
    pub(crate) fn custom_create<T: Sized>(
        ty: (Vec<ValType>, Vec<ValType>),
        wrapper_fn: FnWrapper,
        real_fn: *mut c_void,
        data: *mut T,
        cost: u64,
    ) -> WasmEdgeResult<Self> {
        unsafe {
            let ty = FuncType::create(ty.0, ty.1)?;
            let ctx = ffi::WasmEdge_FunctionInstanceCreateBinding(
                ty.inner.0,
                Some(wrapper_fn),
                real_fn,
                data.cast(),
                cost,
            );
            ty.delete();

            match ctx.is_null() {
                true => Err(WasmEdgeError::Func(FuncError::Create)),
                false => Ok(Self {
                    inner: InnerFunc(ctx),
                }),
            }
        }
    }
}
