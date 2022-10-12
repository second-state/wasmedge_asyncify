use std::{
    pin::Pin,
    task::{Context, Poll, Waker},
};

use crate::{
    core::{
        executor::{Executor, InnerExecutor},
        instance::function::{FuncRef, Function},
        AsInnerInstance, AsInstance, AstModule, ImportModule, InnerInstance,
    },
    error::{CoreError, InstanceError},
    types::{ValType, WasmEdgeString, WasmVal},
    Memory,
};
use thiserror::Error;
use wasmedge_sys_ffi as ffi;

use super::store::Store;

#[derive(Error, Clone, Debug, PartialEq, Eq)]
pub enum CallError {
    #[error("{0}")]
    InstanceError(#[from] InstanceError),
    #[error("{0}")]
    RuntimeError(#[from] CoreError),
}

pub struct AsyncInstance<'import> {
    _store: std::marker::PhantomData<Store<'import>>,
    executor: Executor,
    inner: InnerInstance,
}

impl<'a> AsyncInstance<'a> {
    pub fn instance(
        executor: Executor,
        store: &'a mut Store<'a>,
        module: &AstModule,
    ) -> Result<AsyncInstance<'a>, CoreError> {
        let inner = executor.instantiate(&store.inner_store, module)?;
        Ok(AsyncInstance {
            _store: Default::default(),
            executor,
            inner,
        })
    }

    #[allow(dead_code)]
    fn asyncify_yield(&mut self) -> Result<(), CallError> {
        let f = self.inner.get_func("asyncify_start_unwind")?;
        f.call(&self.executor, &[])?;
        Ok(())
    }

    fn asyncify_normal(&mut self) -> Result<(), CallError> {
        let f = self.inner.get_func("asyncify_stop_unwind")?;
        self.executor.run_func_ref(&f, &[])?;
        Ok(())
    }

    fn asyncify_resume(&mut self) -> Result<(), CallError> {
        if !self.asyncify_done()? {
            let f = self.inner.get_func("asyncify_start_rewind")?;
            self.executor.run_func_ref(&f, &[])?;
        }
        Ok(())
    }

    pub(crate) fn asyncify_done(&mut self) -> Result<bool, CallError> {
        let f = self.inner.get_func("asyncify_get_state")?;
        let r = self.executor.run_func_ref(&f, &[])?;

        if let Some(WasmVal::I32(i)) = r.first() {
            return Ok(*i == 0);
        }
        return Ok(true);
    }

    pub fn call(
        &'a mut self,
        name: &str,
        args: Vec<WasmVal>,
    ) -> Result<CallFuture<'a>, InstanceError> {
        let fun_ref = self.inner.get_func(name)?;
        Ok(CallFuture {
            inst: self,
            fun_ref,
            args,
            fut_store: None,
        })
    }

    pub fn unpack(self) -> Executor {
        let Self { executor, .. } = self;
        executor
    }
}

pub struct AsyncInstanceRef {
    executor: Executor,
    inner: InnerInstance,
}

impl AsyncInstanceRef {
    fn asyncify_yield(&mut self) -> Result<(), CoreError> {
        let f = self
            .inner
            .get_func("asyncify_start_unwind")
            .or_else(|_| Err(CoreError::Asyncify))?;
        f.call(&self.executor, &[])?;
        Ok(())
    }

    fn asyncify_normal(&mut self) -> Result<(), CoreError> {
        let f = self
            .inner
            .get_func("asyncify_stop_unwind")
            .or_else(|_| Err(CoreError::Asyncify))?;
        self.executor.run_func_ref(&f, &[])?;
        Ok(())
    }

    #[allow(dead_code)]
    fn asyncify_resume(&mut self) -> Result<(), CoreError> {
        if !self.asyncify_is_normal()? {
            let f = self
                .inner
                .get_func("asyncify_start_rewind")
                .or_else(|_| Err(CoreError::Asyncify))?;
            self.executor.run_func_ref(&f, &[])?;
        }
        Ok(())
    }

    pub(crate) fn asyncify_is_normal(&mut self) -> Result<bool, CoreError> {
        let f = self
            .inner
            .get_func("asyncify_get_state")
            .or_else(|_| Err(CoreError::Asyncify))?;
        let r = self.executor.run_func_ref(&f, &[])?;

        if let Some(WasmVal::I32(i)) = r.first() {
            return Ok(*i == 0);
        }
        return Ok(true);
    }
}

impl AsInnerInstance for AsyncInstanceRef {
    unsafe fn get_mut_ptr(&self) -> *mut ffi::WasmEdge_ModuleInstanceContext {
        self.inner.0
    }
}

scoped_tls::scoped_thread_local!(static FUT_STORE_RAW_PTR:(Waker,*const c_void));

pub struct CallFuture<'a> {
    inst: &'a mut AsyncInstance<'a>,
    pub(crate) fun_ref: FuncRef,
    pub(crate) args: Vec<WasmVal>,
    fut_store: Option<Pin<ResultFuture<'a>>>,
}

use std::future::Future;
impl Future for CallFuture<'_> {
    type Output = Result<Vec<WasmVal>, CoreError>;

    fn poll(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        let CallFuture {
            inst,
            fun_ref,
            args,
            fut_store,
        } = self.as_mut().get_mut();
        let waker = cx.waker();

        // if let Some(fut) = fut_store {
        //     if Future::poll(fut.as_mut(), cx).is_pending() {
        //         return Poll::Pending;
        //     }
        // }

        if let Err(_) = inst.asyncify_resume() {
            return Poll::Ready(Err(CoreError::Asyncify));
        }

        let s = (
            waker.clone(),
            (fut_store as *const Option<Pin<ResultFuture>>).cast(),
        );

        let r = FUT_STORE_RAW_PTR.set(&s, || inst.executor.run_func_ref(&fun_ref, args));
        match r {
            Ok(v) => match inst.asyncify_done() {
                Ok(true) => Poll::Ready(Ok(v)),
                Ok(false) => Poll::Pending,
                Err(_) => Poll::Ready(Err(CoreError::Asyncify)),
            },
            Err(e) => {
                if let Err(_) = inst.asyncify_normal() {
                    Poll::Ready(Err(CoreError::Asyncify))
                } else {
                    Poll::Ready(Err(e))
                }
            }
        }
    }
}

pub type AsyncFn<T> =
    for<'a> fn(&'a mut AsyncInstanceRef, &'a Memory, &'a mut T, Vec<WasmVal>) -> ResultFuture<'a>;
pub type ResultFuture<'a> = Box<dyn Future<Output = Result<Vec<WasmVal>, CoreError>> + 'a>;

#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum AddFuncError {
    #[error("Found an interior nul byte")]
    NameError(#[from] std::ffi::NulError),
    #[error("Fail to create Function instance")]
    FunctionCreate,
}

use std::ffi::c_void;
pub(crate) unsafe extern "C" fn wrapper_async_fn<T: Sized + Send>(
    key_ptr: *mut c_void,
    data_ptr: *mut c_void,
    calling_frame_ctx: *const ffi::WasmEdge_CallingFrameContext,
    params: *const ffi::WasmEdge_Value,
    param_len: u32,
    returns: *mut ffi::WasmEdge_Value,
    return_len: u32,
) -> ffi::WasmEdge_Result {
    if !FUT_STORE_RAW_PTR.is_set() {
        return ffi::WasmEdge_ResultGen(ffi::WasmEdge_ErrCategory_UserLevelError, 1);
    }

    let cous = || -> Result<(), CoreError> {
        let inst_ctx = ffi::WasmEdge_CallingFrameGetModuleInstance(calling_frame_ctx);
        let executor_ctx = ffi::WasmEdge_CallingFrameGetExecutor(calling_frame_ctx);
        let main_mem_ctx = ffi::WasmEdge_CallingFrameGetMemoryInstance(calling_frame_ctx, 0);

        let mut inst = std::mem::ManuallyDrop::new(AsyncInstanceRef {
            executor: Executor {
                inner: InnerExecutor(executor_ctx),
            },
            inner: InnerInstance(inst_ctx as *mut _),
        });

        let mut mem = Memory::from_raw(main_mem_ctx);

        let (w, ptr) = FUT_STORE_RAW_PTR.with(|(w, ptr)| {
            (
                w.clone(),
                (*ptr as *mut Option<Pin<ResultFuture>>).as_mut().unwrap(),
            )
        });
        let mut cx = Context::from_waker(&w);

        let data_ptr = data_ptr.cast::<T>().as_mut();
        debug_assert!(data_ptr.is_some());
        let data_ptr = data_ptr.unwrap();

        let fut_is_ready;
        let r = {
            let fut = if inst.asyncify_is_normal()? {
                let real_fn: for<'a> fn(
                    &'a mut AsyncInstanceRef,
                    &mut Memory,
                    &mut T,
                    Vec<WasmVal>,
                ) -> ResultFuture<'a> = std::mem::transmute(key_ptr);

                let input = {
                    let raw_input = std::slice::from_raw_parts(params, param_len as usize);
                    raw_input
                        .iter()
                        .map(|r| (*r).into())
                        .collect::<Vec<WasmVal>>()
                };

                Some(Pin::from(real_fn(&mut inst, &mut mem, data_ptr, input)))
            } else {
                ptr.take()
            };

            debug_assert!(fut.is_some());
            let mut fut = fut.unwrap();

            let return_len = return_len as usize;
            let raw_returns = std::slice::from_raw_parts_mut(returns, return_len);

            match Future::poll(fut.as_mut(), &mut cx) {
                std::task::Poll::Ready(result) => {
                    fut_is_ready = true;
                    match result {
                        Ok(v) => {
                            debug_assert!(v.len() == return_len);
                            for (idx, item) in v.into_iter().enumerate() {
                                raw_returns[idx] = item.into();
                            }
                            Ok(())
                        }
                        Err(e) => Err(e),
                    }
                }
                std::task::Poll::Pending => {
                    fut_is_ready = false;
                    let _ = ptr.insert(fut);
                    Ok(())
                }
            }
        };

        if fut_is_ready {
            inst.asyncify_normal()?;
        } else {
            inst.asyncify_yield()?;
        };
        r
    };
    match cous() {
        Ok(_) => ffi::WasmEdge_Result { Code: 0x0 },
        Err(e) => e.into(),
    }
}

impl<T: Send + Sized> ImportModule<T> {
    pub fn add_async_func(
        &mut self,
        name: &str,
        ty: (Vec<ValType>, Vec<ValType>),
        real_fn: AsyncFn<T>,
    ) -> Result<(), AddFuncError> {
        let func_name = WasmEdgeString::new(name)?;
        unsafe {
            let func = Function::custom_create(
                ty,
                wrapper_async_fn::<T>,
                real_fn as *mut _,
                &mut self.data,
            )
            .ok_or(AddFuncError::FunctionCreate)?;

            ffi::WasmEdge_ModuleInstanceAddFunction(
                self.inner.0,
                func_name.as_raw(),
                func.inner.0 as *mut _,
            );
            Ok(())
        }
    }
}
