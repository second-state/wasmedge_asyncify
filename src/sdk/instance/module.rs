use std::{
    fmt::Debug,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll, Waker},
};

use crate::{
    core::{
        executor::{Executor, InnerExecutor},
        instance::function::{FnWrapper, FuncRef, Function},
        AsInnerInstance, AsInstance, AstModule, Global, ImportModule, InnerInstance, MutGlobal,
    },
    error::{CoreCommonError, CoreError, InstanceError},
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
    asyncify_stack: i32,
}

pub struct InstanceSnapshot {
    pub globals: Vec<MutGlobal>,
    pub memories: Vec<(String, Arc<Vec<u8>>)>,
    pub asyncify_stack: i32,
}

impl Debug for InstanceSnapshot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "InstanceRunState{{ asyncify_stack:{}\nglobals:{:?}, ",
            self.asyncify_stack, self.globals
        )?;
        write!(f, "memories:{{ ")?;
        for mem in &self.memories {
            write!(f, "({},{}) ", &mem.0, mem.1.len())?;
        }
        write!(f, "}}")?;
        Ok(())
    }
}

impl<'a> AsyncInstance<'a> {
    pub fn instance(
        executor: Executor,
        store: &'a mut Store<'a>,
        module: &AstModule,
    ) -> Result<AsyncInstance<'a>, CoreError> {
        let inner = executor.instantiate(&store.inner_store, module)?;
        let mut async_inner = AsyncInstance {
            _store: Default::default(),
            executor,
            inner,
            asyncify_stack: 0,
        };
        async_inner
            .init_asyncify_stack()
            .map_err(|_| CoreError::Asyncify)?;
        Ok(async_inner)
    }

    fn init_asyncify_stack(&mut self) -> Result<(), CallError> {
        use wasmedge_async_wasi::snapshots::common::memory::{Memory, WasmPtr};
        log::trace!("init_asyncify_stack");
        if self.asyncify_stack != 0 {
            return Ok(());
        }
        let malloc_func = self.inner.get_func("malloc")?;
        let r = malloc_func.call(&self.executor, &[WasmVal::I32(32 * 1024)])?;
        if let Some(WasmVal::I32(i)) = r.first() {
            self.asyncify_stack = *i;
            let mut mem = self.inner.get_memory("memory")?;
            mem.write_data(WasmPtr::from(*i as usize), [*i + 8, *i + 32 * 1024 - 8])
                .map_err(|_| CoreError::Asyncify)?;
            Ok(())
        } else {
            Err(CallError::RuntimeError(CoreError::Asyncify))
        }
    }

    #[allow(dead_code)]
    fn asyncify_yield(&mut self) -> Result<(), CallError> {
        log::trace!("asyncify_yield");
        let f = self.inner.get_func("asyncify_start_unwind")?;
        f.call(&self.executor, &[WasmVal::I32(self.asyncify_stack)])?;
        Ok(())
    }

    #[allow(dead_code)]
    fn asyncify_normal(&mut self) -> Result<(), CallError> {
        log::trace!("asyncify_normal");

        let f = self.inner.get_func("asyncify_stop_unwind")?;
        self.executor.run_func_ref(&f, &[])?;
        Ok(())
    }

    fn asyncify_resume(&mut self) -> Result<(), CallError> {
        log::trace!("asyncify_resume");

        if !self.asyncify_is_normal()? {
            log::trace!("call asyncify_resume");

            let f = self.inner.get_func("asyncify_start_rewind")?;
            self.executor
                .run_func_ref(&f, &[WasmVal::I32(self.asyncify_stack)])?;
        }
        Ok(())
    }

    pub(crate) fn asyncify_is_normal(&mut self) -> Result<bool, CallError> {
        let f = self.inner.get_func("asyncify_get_state")?;
        let r = self.executor.run_func_ref(&f, &[])?;

        if let Some(WasmVal::I32(i)) = r.first() {
            return Ok(*i == 0);
        }
        return Ok(true);
    }

    pub fn snapshot(&self) -> InstanceSnapshot {
        let mut globals = vec![];
        for global in self.inner.get_all_exports_globals() {
            if let Global::Mut(g) = global {
                globals.push(g);
            }
        }
        let mut memories = vec![];
        for (name, mem) in self.inner.get_all_exports_memories() {
            let page_size = mem.page_size() as usize;
            let mem_end = page_size * (64 * 1024);
            let data = mem
                .data_pointer(0, mem_end)
                .map(|v| Arc::new(v.to_vec()))
                .unwrap_or_default();
            memories.push((name, data));
        }

        InstanceSnapshot {
            globals,
            memories,
            asyncify_stack: self.asyncify_stack,
        }
    }

    pub fn apply_snapshot(&mut self, snapshot: InstanceSnapshot) -> Result<(), InstanceError> {
        let InstanceSnapshot {
            globals,
            memories,
            asyncify_stack,
        } = snapshot;
        for g in globals {
            self.inner.set_global(g)?;
        }
        for (name, data) in memories {
            let mut mem = self.inner.get_memory(&name)?;
            mem.write_bytes(data.as_slice(), 0)
                .map_err(|_| InstanceError::WriteMem(name))?;
        }

        self.asyncify_stack = asyncify_stack;

        Ok(())
    }

    pub async fn call<'r>(
        &'r mut self,
        name: &str,
        args: Vec<WasmVal>,
    ) -> Result<Vec<WasmVal>, CoreError>
    where
        'a: 'r,
    {
        let fun_ref = self
            .inner
            .get_func(name)
            .map_err(|_| CoreError::Common(CoreCommonError::FuncNotFound))?;
        let inst = self;
        let f = CallFuture::<'r, 'a> {
            inst,
            fun_ref,
            args,
            fut_store: None,
        }
        .await;
        f
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
    fn asyncify_yield(&mut self, asyncify_stack: i32) -> Result<(), CoreError> {
        log::trace!("asyncify_yield {asyncify_stack}");

        let f = self
            .inner
            .get_func("asyncify_start_unwind")
            .or_else(|_| Err(CoreError::Asyncify))?;
        f.call(&self.executor, &[WasmVal::I32(asyncify_stack)])?;
        Ok(())
    }

    fn asyncify_normal(&mut self) -> Result<(), CoreError> {
        log::trace!("asyncify_normal");

        let f = self
            .inner
            .get_func("asyncify_stop_unwind")
            .or_else(|_| Err(CoreError::Asyncify))?;
        self.executor.run_func_ref(&f, &[])?;
        Ok(())
    }

    #[allow(dead_code)]
    fn asyncify_resume(&mut self, asyncify_stack: i32) -> Result<(), CoreError> {
        log::trace!("asyncify_resume");

        if !self.asyncify_is_normal()? {
            log::trace!("call asyncify_resume");

            let f = self
                .inner
                .get_func("asyncify_start_rewind")
                .or_else(|_| Err(CoreError::Asyncify))?;
            self.executor
                .run_func_ref(&f, &[WasmVal::I32(asyncify_stack)])?;
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

scoped_tls::scoped_thread_local!(static FUT_STORE_RAW_PTR:(Waker,*const c_void,i32));

enum FutureReady<'r> {
    Wait(Pin<ResultFuture<'r>>),
    Yield,
}

pub struct CallFuture<'r, 'inst: 'r> {
    inst: &'r mut AsyncInstance<'inst>,
    pub(crate) fun_ref: FuncRef,
    pub(crate) args: Vec<WasmVal>,
    fut_store: Option<FutureReady<'r>>,
}

use std::future::Future;
impl<'r, 'inst: 'r> Future for CallFuture<'r, 'inst> {
    type Output = Result<Vec<WasmVal>, CoreError>;

    fn poll(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        let asyncify_stack = self.inst.asyncify_stack;
        let CallFuture {
            inst,
            fun_ref,
            args,
            fut_store,
        } = self.as_mut().get_mut();
        let waker = cx.waker();

        if let Err(_) = inst.asyncify_resume() {
            return Poll::Ready(Err(CoreError::Asyncify));
        }

        let s = (
            waker.clone(),
            (fut_store as *const Option<FutureReady>).cast(),
            asyncify_stack,
        );

        let r = FUT_STORE_RAW_PTR.set(&s, || inst.executor.run_func_ref(&fun_ref, args));
        match r {
            Ok(v) => match fut_store {
                Some(FutureReady::Wait(_)) => Poll::Pending,
                Some(FutureReady::Yield) => Poll::Ready(Err(CoreError::Yield)),
                None => Poll::Ready(Ok(v)),
            },
            Err(e) => Poll::Ready(Err(e)),
        }
    }
}

pub type AsyncWasmFn<T> = for<'a> fn(
    &'a mut AsyncInstanceRef,
    &'a mut Memory,
    &'a mut T,
    Vec<WasmVal>,
) -> ResultFuture<'a>;
pub type ResultFuture<'a> = Box<dyn Future<Output = Result<Vec<WasmVal>, CoreError>> + 'a>;

pub type SyncWasmFn<T> = for<'a> fn(
    &'a mut AsyncInstanceRef,
    &'a mut Memory,
    &'a mut T,
    Vec<WasmVal>,
) -> Result<Vec<WasmVal>, CoreError>;

#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum AddFuncError {
    #[error("Found an interior nul byte")]
    NameError(#[from] std::ffi::NulError),
    #[error("Illegal Async Function name ")]
    IllegalName,
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

        let (w, ptr, asyncify_stack) = FUT_STORE_RAW_PTR.with(|(w, ptr, asyncify_stack)| {
            (
                w.clone(),
                (*ptr as *mut Option<FutureReady>).as_mut().unwrap(),
                *asyncify_stack,
            )
        });
        let mut cx = Context::from_waker(&w);

        let data_ptr = data_ptr.cast::<T>().as_mut();
        debug_assert!(data_ptr.is_some());
        let data_ptr = data_ptr.unwrap();

        let fut_is_ready;
        let r = {
            log::trace!("take fut");
            let mut fut = match ptr.take() {
                Some(FutureReady::Wait(fut)) => fut,
                Some(FutureReady::Yield) => unreachable!(),
                None => {
                    log::trace!("create fut");
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

                    Pin::from(real_fn(&mut inst, &mut mem, data_ptr, input))
                }
            };

            let return_len = return_len as usize;
            let raw_returns = std::slice::from_raw_parts_mut(returns, return_len);

            match Future::poll(fut.as_mut(), &mut cx) {
                std::task::Poll::Ready(result) => match result {
                    Ok(v) => {
                        fut_is_ready = true;

                        debug_assert!(v.len() == return_len);
                        for (idx, item) in v.into_iter().enumerate() {
                            raw_returns[idx] = item.into();
                        }
                        Ok(())
                    }
                    Err(e) => {
                        if let CoreError::Yield = e {
                            fut_is_ready = false;
                            let _ = ptr.insert(FutureReady::Yield);
                            Ok(())
                        } else {
                            fut_is_ready = true;
                            Err(e)
                        }
                    }
                },
                std::task::Poll::Pending => {
                    fut_is_ready = false;
                    let _ = ptr.insert(FutureReady::Wait(fut));
                    Ok(())
                }
            }
        };

        if fut_is_ready {
            inst.asyncify_normal()?;
        } else {
            inst.asyncify_yield(asyncify_stack)?;
        };
        r
    };
    match cous() {
        Ok(_) => ffi::WasmEdge_Result { Code: 0x0 },
        Err(e) => e.into(),
    }
}

pub(crate) unsafe extern "C" fn wrapper_sync_fn<T: Sized + Send>(
    key_ptr: *mut c_void,
    data_ptr: *mut c_void,
    calling_frame_ctx: *const ffi::WasmEdge_CallingFrameContext,
    params: *const ffi::WasmEdge_Value,
    param_len: u32,
    returns: *mut ffi::WasmEdge_Value,
    return_len: u32,
) -> ffi::WasmEdge_Result {
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

        let data_ptr = data_ptr.cast::<T>().as_mut();
        debug_assert!(data_ptr.is_some());
        let data_ptr = data_ptr.unwrap();

        let real_fn: fn(
            &mut AsyncInstanceRef,
            &mut Memory,
            &mut T,
            Vec<WasmVal>,
        ) -> Result<Vec<WasmVal>, CoreError> = std::mem::transmute(key_ptr);

        let input = {
            let raw_input = std::slice::from_raw_parts(params, param_len as usize);
            raw_input
                .iter()
                .map(|r| (*r).into())
                .collect::<Vec<WasmVal>>()
        };
        let v = real_fn(&mut inst, &mut mem, data_ptr, input)?;

        let return_len = return_len as usize;
        let raw_returns = std::slice::from_raw_parts_mut(returns, return_len);

        for (idx, item) in v.into_iter().enumerate() {
            raw_returns[idx] = item.into();
        }
        Ok(())
    };
    match cous() {
        Ok(_) => ffi::WasmEdge_Result { Code: 0x0 },
        Err(e) => e.into(),
    }
}

impl<T: Send + Sized> ImportModule<T> {
    pub unsafe fn add_custom_func(
        &mut self,
        name: &str,
        ty: (Vec<ValType>, Vec<ValType>),
        wrapper_fn: FnWrapper,
        real_fn: *mut c_void,
        data: *mut T,
    ) -> Result<(), AddFuncError> {
        let func_name = WasmEdgeString::new(name)?;
        let func = Function::custom_create(ty, wrapper_fn, real_fn, data.cast())
            .ok_or(AddFuncError::FunctionCreate)?;

        ffi::WasmEdge_ModuleInstanceAddFunction(
            self.inner.0,
            func_name.as_raw(),
            func.inner.0 as *mut _,
        );
        Ok(())
    }

    /// create a async host function
    ///
    /// # Errors
    /// It will return a error [AddFuncError::IllegalName] if `name` is not start with `async_`.
    pub fn add_async_func(
        &mut self,
        name: &str,
        ty: (Vec<ValType>, Vec<ValType>),
        real_fn: AsyncWasmFn<T>,
    ) -> Result<(), AddFuncError> {
        if !name.starts_with("async_") {
            return Err(AddFuncError::IllegalName);
        }
        unsafe {
            let data_ptr = self.data.as_mut() as *mut T;
            self.add_custom_func(name, ty, wrapper_async_fn::<T>, real_fn as *mut _, data_ptr)
        }
    }

    pub unsafe fn add_async_func_uncheck(
        &mut self,
        name: &str,
        ty: (Vec<ValType>, Vec<ValType>),
        real_fn: AsyncWasmFn<T>,
    ) -> Result<(), AddFuncError> {
        let data_ptr = self.data.as_mut() as *mut T;
        self.add_custom_func(name, ty, wrapper_async_fn::<T>, real_fn as *mut _, data_ptr)
    }

    pub fn add_sync_func(
        &mut self,
        name: &str,
        ty: (Vec<ValType>, Vec<ValType>),
        real_fn: SyncWasmFn<T>,
    ) -> Result<(), AddFuncError> {
        unsafe {
            let data_ptr = self.data.as_mut() as *mut T;
            self.add_custom_func(name, ty, wrapper_sync_fn::<T>, real_fn as *mut _, data_ptr)
        }
    }
}
