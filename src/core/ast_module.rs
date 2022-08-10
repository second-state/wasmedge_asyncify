use std::borrow::Cow;

use wasmedge_types::error::WasmEdgeError;

use super::config::Config;
use crate::utils::check;

use wasmedge_sys::ffi;

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
unsafe impl Sync for Loader {}

impl Loader {
    pub fn create(config: &Option<Config>) -> Result<Self, WasmEdgeError> {
        unsafe {
            let config_ctx = if let Some(c) = config {
                c.inner.0
            } else {
                std::ptr::null()
            };

            let loader_inner = ffi::WasmEdge_LoaderCreate(config_ctx);
            if loader_inner.is_null() {
                return Err(WasmEdgeError::LoaderCreate);
            }

            let validator_inner = ffi::WasmEdge_ValidatorCreate(config_ctx);
            if validator_inner.is_null() {
                return Err(WasmEdgeError::ValidatorCreate);
            }
            Ok(Self {
                loader_inner,
                validator_inner,
            })
        }
    }

    pub fn load_module_from_bytes(&mut self, wasm: &[u8]) -> Result<AstModule, WasmEdgeError> {
        unsafe {
            let mut mod_ctx: *mut ffi::WasmEdge_ASTModuleContext = std::ptr::null_mut();

            check(ffi::WasmEdge_LoaderParseFromBuffer(
                self.loader_inner,
                &mut mod_ctx,
                wasm.as_ptr(),
                wasm.len() as u32,
            ))?;

            if mod_ctx.is_null() {
                return Err(WasmEdgeError::ModuleCreate);
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

    pub fn pass_async_module_from_bytes<'a, B: AsRef<str>, I: IntoIterator<Item = B>>(
        &mut self,
        wasm: &'a [u8],
        passes: I,
        codegen_config: &CodegenConfig,
    ) -> Result<Cow<'a, [u8]>, WasmEdgeError> {
        let mut module = binaryen::Module::read(wasm).map_err(|_| WasmEdgeError::ModuleCreate)?;

        if module.get_export("asyncify_get_state").unwrap().is_null() {
            // skip run start on init
            {
                if let Some(start) = module.get_start() {
                    let global_ref = module.add_global("start_initialized", true, 0_i32).unwrap();
                    let new_body = module.binaryen_if(
                        module.binaryen_get_global(global_ref),
                        start.body(),
                        module.binaryen_set_global(global_ref, module.binaryen_const_value(1_i32)),
                    );
                    start.set_body(new_body);
                    module
                        .add_function_export(&start, "__original_start")
                        .unwrap();
                }
            }

            module
                .run_optimization_passes(passes, &codegen_config)
                .map_err(|_| WasmEdgeError::ModuleCreate)?;

            let new_wasm = module.write();
            Ok(Cow::Owned(new_wasm))
        } else {
            Ok(Cow::Borrowed(wasm))
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
