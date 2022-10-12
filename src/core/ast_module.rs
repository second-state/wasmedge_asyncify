use std::borrow::Cow;

use super::config::Config;
use crate::error::{CoreCommonError, CoreError, CoreLoadError};
use crate::utils::check;

use wasmedge_sys_ffi as ffi;

pub type CodegenConfig = binaryen::CodegenConfig;

pub struct Loader {
    pub(crate) loader_inner: *mut ffi::WasmEdge_LoaderContext,
    pub(crate) validator_inner: *mut ffi::WasmEdge_ValidatorContext,
}
impl Drop for Loader {
    fn drop(&mut self) {
        unsafe {
            if !self.loader_inner.is_null() {
                ffi::WasmEdge_LoaderDelete(self.loader_inner)
            }
            if !self.validator_inner.is_null() {
                ffi::WasmEdge_ValidatorDelete(self.validator_inner)
            }
        }
    }
}
unsafe impl Send for Loader {}

impl Loader {
    pub fn create(config: &Option<Config>) -> Option<Self> {
        unsafe {
            let config_ctx = if let Some(c) = config {
                c.inner.0
            } else {
                std::ptr::null()
            };

            let loader_inner = ffi::WasmEdge_LoaderCreate(config_ctx);
            if loader_inner.is_null() {
                return None;
            }

            let validator_inner = ffi::WasmEdge_ValidatorCreate(config_ctx);
            if validator_inner.is_null() {
                ffi::WasmEdge_LoaderDelete(loader_inner);
                return None;
            }
            Some(Self {
                loader_inner,
                validator_inner,
            })
        }
    }

    pub fn load_async_module_from_bytes(&self, wasm: &[u8]) -> Result<AstModule, CoreError> {
        let mut codegen_config = CodegenConfig::default();
        codegen_config.optimization_level = 2;
        codegen_config
            .pass_argument
            .push(("asyncify-imports".to_string(), "*.async_".to_string()));

        let new_wasm = pass_async_module(wasm, ["asyncify", "strip"], &codegen_config)
            .ok_or(CoreError::Load(CoreLoadError::ReadError))?;
        self.load_module_from_bytes(&new_wasm)
    }

    pub fn load_module_from_bytes(&self, wasm: &[u8]) -> Result<AstModule, CoreError> {
        unsafe {
            let mut mod_ctx: *mut ffi::WasmEdge_ASTModuleContext = std::ptr::null_mut();

            check(ffi::WasmEdge_LoaderParseFromBuffer(
                self.loader_inner,
                &mut mod_ctx,
                wasm.as_ptr(),
                wasm.len() as u32,
            ))?;

            debug_assert!(!mod_ctx.is_null());
            if mod_ctx.is_null() {
                return Err(CoreError::Common(CoreCommonError::RuntimeError));
            }

            let validate_result = check(ffi::WasmEdge_ValidatorValidate(
                self.validator_inner,
                mod_ctx,
            ));

            if let Err(e) = validate_result {
                ffi::WasmEdge_ASTModuleDelete(mod_ctx);
                return Err(e);
            }

            Ok(AstModule { inner: mod_ctx })
        }
    }
}

#[derive(Debug)]
pub struct AstModule {
    pub(crate) inner: *mut ffi::WasmEdge_ASTModuleContext,
}
impl Drop for AstModule {
    fn drop(&mut self) {
        if !self.inner.is_null() {
            unsafe { ffi::WasmEdge_ASTModuleDelete(self.inner) };
        }
    }
}
unsafe impl Send for AstModule {}
unsafe impl Sync for AstModule {}

pub(crate) fn pass_async_module<'a, B: AsRef<str>, I: IntoIterator<Item = B>>(
    wasm: &'a [u8],
    passes: I,
    codegen_config: &CodegenConfig,
) -> Option<Cow<'a, [u8]>> {
    let mut module = binaryen::Module::read(wasm).ok()?;

    if module.get_export("asyncify_get_state").unwrap().is_null() {
        module
            .run_optimization_passes(passes, &codegen_config)
            .ok()?;

        let new_wasm = module.write();
        Some(Cow::Owned(new_wasm))
    } else {
        Some(Cow::Borrowed(wasm))
    }
}
