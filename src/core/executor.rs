//! Defines WasmEdge Executor.
use std::collections::HashMap;

use wasmedge_types::error::WasmEdgeError;
use wasmedge_types::WasmEdgeResult;

use super::ast_module::AstModule;
use super::instance::function::FuncRef;
use super::module::{ImportModule, InnerInstance, Instance};
use super::{config::Config, types::WasmVal};

use crate::utils::check;

use wasmedge_sys::ffi;

/// Defines an execution environment for both pure WASM and compiled WASM.
#[derive(Debug)]
pub struct Executor {
    pub(crate) inner: InnerExecutor,
    pub(crate) inner_store: InnerStore,
    imports: HashMap<String, ImportModule>,
}
impl Executor {
    pub fn create(config: &Option<Config>) -> WasmEdgeResult<Self> {
        unsafe {
            let conf_ctx = match config {
                Some(cfg) => cfg.inner.0,
                None => std::ptr::null_mut(),
            };
            let ctx = ffi::WasmEdge_ExecutorCreate(conf_ctx, std::ptr::null_mut());
            let store_ctx = ffi::WasmEdge_StoreCreate();

            match ctx.is_null() {
                true => Err(WasmEdgeError::ExecutorCreate),
                false => Ok(Executor {
                    inner: InnerExecutor(ctx),
                    inner_store: InnerStore(store_ctx),
                    imports: HashMap::new(),
                }),
            }
        }
    }

    pub fn register_import_object(&mut self, import: ImportModule) -> WasmEdgeResult<()> {
        unsafe {
            check(ffi::WasmEdge_ExecutorRegisterImport(
                self.inner.0,
                self.inner_store.0,
                import.inner.0,
            ))?;
            self.imports.insert(import.name(), import);
        }

        Ok(())
    }

    // fixme
    pub fn instantiate(&mut self, module: &AstModule) -> WasmEdgeResult<Instance> {
        let mut instance_ctx = std::ptr::null_mut();
        unsafe {
            check(ffi::WasmEdge_ExecutorInstantiate(
                self.inner.0,
                &mut instance_ctx,
                self.inner_store.0,
                module.inner,
            ))?;
        }

        if instance_ctx.is_null() {
            return Err(WasmEdgeError::Instance(
                wasmedge_types::error::InstanceError::Create,
            ));
        }

        Ok(Instance {
            inner: InnerInstance(instance_ctx),
        })
    }

    pub fn run_func_ref(
        &mut self,
        func: &FuncRef,
        params: &[WasmVal],
    ) -> WasmEdgeResult<Vec<WasmVal>> {
        let raw_params = params.into_iter().map(|x| x.into()).collect::<Vec<_>>();

        // get the length of the function's returns
        let returns_len = func.func_return_size()?;

        unsafe {
            let mut returns = Vec::with_capacity(returns_len);

            check(ffi::WasmEdge_ExecutorInvoke(
                self.inner.0,
                func.inner.0,
                raw_params.as_ptr(),
                raw_params.len() as u32,
                returns.as_mut_ptr(),
                returns_len as u32,
            ))?;
            returns.set_len(returns_len);
            Ok(returns.into_iter().map(Into::into).collect::<Vec<_>>())
        }
    }
}

#[derive(Debug)]
pub(crate) struct InnerExecutor(pub(crate) *mut ffi::WasmEdge_ExecutorContext);
impl Drop for InnerExecutor {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe { ffi::WasmEdge_ExecutorDelete(self.0) }
        }
    }
}
unsafe impl Send for InnerExecutor {}
unsafe impl Sync for InnerExecutor {}

#[derive(Debug)]
pub(crate) struct InnerStore(pub(crate) *mut ffi::WasmEdge_StoreContext);
impl Drop for InnerStore {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe { ffi::WasmEdge_StoreDelete(self.0) }
        }
    }
}
unsafe impl Send for InnerStore {}
unsafe impl Sync for InnerStore {}
