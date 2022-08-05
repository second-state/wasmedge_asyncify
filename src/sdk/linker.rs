use std::{ffi::c_void, marker::PhantomPinned, pin::Pin, ptr::NonNull, task::Waker};

use wasmedge_types::{
    error::{CoreCommonError, CoreError, WasmEdgeError},
    WasmEdgeResult,
};

use crate::core::{
    config::Config, executor::Executor, types::WasmVal, AsInstance, AstModule, CodegenConfig,
    ImportModule, Instance, Loader,
};

use super::{
    instance::function::{ResultFuture, WasmEdgeResultFuture},
    module::AsyncImportModuleBuilder,
};

// std::collections::LinkedList<Pin<ResultFuture<'this>>>
struct AsyncFutureList(NonNull<c_void>);
unsafe impl Sync for AsyncFutureList {}
unsafe impl Send for AsyncFutureList {}
impl Drop for AsyncFutureList {
    fn drop<'a>(&'a mut self) {
        unsafe {
            let ptr = self.0.as_ptr() as *mut std::collections::LinkedList<Pin<ResultFuture<'a>>>;
            let futures = Box::from_raw(ptr);
            std::mem::drop(futures);
        }
    }
}

pub struct AsyncLinker {
    pub(crate) cx: Waker,
    pub(crate) inst: Option<Instance>,
    pub(crate) executor: Executor,
    pub(crate) vm_err: Option<WasmEdgeError>,

    func_futures_ptr: AsyncFutureList,
    _unpin: PhantomPinned,
}

impl AsyncLinker {
    pub(crate) fn func_futures<'a>(
        &'a mut self,
    ) -> &'a mut std::collections::LinkedList<Pin<ResultFuture<'a>>> {
        unsafe { self.func_futures_ptr.0.cast().as_mut() }
    }

    fn new(config: &Option<Config>) -> WasmEdgeResult<Box<Self>> {
        unsafe {
            let func_futures_ptr = Box::leak(Box::new(std::collections::LinkedList::<
                Pin<ResultFuture<'static>>,
            >::new())) as *mut _ as *mut c_void;

            Ok(Box::new(AsyncLinker {
                cx: waker_fn::waker_fn(|| {}),
                func_futures_ptr: AsyncFutureList(NonNull::new_unchecked(func_futures_ptr)),
                _unpin: PhantomPinned,
                inst: None,
                executor: Executor::create(config)?,
                vm_err: None,
            }))
        }
    }

    pub fn call(&mut self, name: &str, args: Vec<WasmVal>) -> WasmEdgeResultFuture {
        WasmEdgeResultFuture {
            linker: self,
            name: name.to_string(),
            args,
        }
    }

    pub(crate) fn real_call(
        &mut self,
        name: &str,
        args: &[WasmVal],
    ) -> WasmEdgeResult<Vec<WasmVal>> {
        let f = if let Some(inst) = &self.inst {
            inst.get_func(name)
        } else {
            Err(WasmEdgeError::Core(CoreError::Common(
                CoreCommonError::RuntimeError,
            )))
        }?;
        f.call(&mut self.executor, args)
    }

    pub(crate) fn asyncify_yield(&mut self) -> WasmEdgeResult<()> {
        self.real_call("asyncify_start_unwind", &[])?;
        Ok(())
    }

    pub(crate) fn asyncify_resume(&mut self) -> WasmEdgeResult<()> {
        if !self.asyncify_done()? {
            self.real_call("asyncify_start_rewind", &[])?;
        }

        Ok(())
    }

    pub(crate) fn asyncify_normal(&mut self) -> WasmEdgeResult<()> {
        self.real_call("asyncify_stop_unwind", &[])?;
        Ok(())
    }

    pub(crate) fn asyncify_done(&mut self) -> WasmEdgeResult<bool> {
        let r = self.real_call("asyncify_get_state", &[])?;
        if let Some(WasmVal::I32(i)) = r.first() {
            return Ok(*i == 0);
        }
        return Ok(true);
    }
}

pub trait AsLinker {
    fn call(&mut self, name: &str, args: Vec<WasmVal>) -> WasmEdgeResultFuture;
}

impl AsLinker for Pin<Box<AsyncLinker>> {
    fn call(&mut self, name: &str, args: Vec<WasmVal>) -> WasmEdgeResultFuture {
        let linker_ctx = unsafe { self.as_mut().get_unchecked_mut() };
        WasmEdgeResultFuture {
            linker: linker_ctx,
            name: name.to_string(),
            args,
        }
    }
}

pub struct AsyncLinkerBuilder {
    linker: Box<AsyncLinker>,
    loader: Loader,
    async_fn_name: Vec<String>,
}

impl AsyncLinkerBuilder {
    pub fn new(config: &Option<Config>) -> WasmEdgeResult<Self> {
        Ok(AsyncLinkerBuilder {
            linker: AsyncLinker::new(config)?,
            async_fn_name: vec![],
            loader: Loader::create(config)?,
        })
    }

    pub fn create_wasi<S: AsRef<str>>(
        &mut self,
        args: &[S],
        envs: &[S],
        preopens: &[S],
    ) -> Result<(), WasmEdgeError> {
        let import_obj = ImportModule::create_wasi(args, envs, preopens)?;
        self.linker.executor.register_import_object(import_obj)?;
        Ok(())
    }

    pub fn create_import_object<
        F: FnOnce(&mut AsyncImportModuleBuilder) -> Result<(), WasmEdgeError>,
    >(
        &mut self,
        name: &str,
        f: F,
    ) -> Result<(), WasmEdgeError> {
        let AsyncLinkerBuilder {
            linker,
            async_fn_name,
            ..
        } = self;
        let mut builder = AsyncImportModuleBuilder {
            import_obj: ImportModule::create(name)?,
            linker_ctx: linker,
            async_fn_name,
        };
        f(&mut builder)?;
        let AsyncImportModuleBuilder {
            import_obj,
            linker_ctx,
            ..
        } = builder;
        linker_ctx.executor.register_import_object(import_obj)?;
        Ok(())
    }

    pub fn load_wasm(&mut self, wasm: &[u8]) -> WasmEdgeResult<AstModule> {
        let AsyncLinkerBuilder {
            async_fn_name,
            loader,
            ..
        } = self;

        let asyncify_imports = async_fn_name.join(",");

        let mut codegen_config = CodegenConfig::default();
        codegen_config.optimization_level = 2;
        codegen_config
            .pass_argument
            .push(("asyncify-imports".to_string(), asyncify_imports));
        let ast_module =
            loader.load_async_module_from_bytes(wasm, ["asyncify", "strip"], &codegen_config)?;
        Ok(ast_module)
    }

    pub fn instance(self, module: &AstModule) -> WasmEdgeResult<Pin<Box<AsyncLinker>>> {
        let AsyncLinkerBuilder { mut linker, .. } = self;
        let inst = linker.executor.instantiate(module)?;
        linker.inst = Some(inst);
        Ok(Pin::from(linker))
    }
}
