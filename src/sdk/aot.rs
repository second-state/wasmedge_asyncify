use std::{
    ops::{Deref, DerefMut},
    path::Path,
};

use wasmedge_sys::ffi;
use wasmedge_types::{error::WasmEdgeError, CompilerOutputFormat, WasmEdgeResult};

pub use wasmedge_types::CompilerOptimizationLevel;

use crate::{utils, AsyncLinkerBuilder, Config};

#[derive(Debug)]
pub struct AotConfig {
    pub(crate) inner: Config,
}

impl Deref for AotConfig {
    type Target = Config;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for AotConfig {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl AotConfig {
    pub fn create() -> WasmEdgeResult<Self> {
        let mut config = AotConfig {
            inner: Config::create()?,
        };
        config.dump_ir(false);
        config.set_aot_compiler_output_format(CompilerOutputFormat::Wasm);
        Ok(config)
    }

    /// Sets the optimization level of AOT compiler.
    ///
    /// Notice that this function is only available when the `aot` feature is enabled.
    ///
    /// # Argument
    ///
    /// * `opt_level` - The optimization level of AOT compiler.
    pub fn set_aot_optimization_level(&mut self, opt_level: CompilerOptimizationLevel) {
        unsafe {
            ffi::WasmEdge_ConfigureCompilerSetOptimizationLevel(
                self.inner.inner.0,
                opt_level as u32,
            )
        }
    }

    /// Returns the optimization level of AOT compiler.
    ///
    /// Notice that this function is only available when the `aot` feature is enabled.
    pub fn get_aot_optimization_level(&self) -> CompilerOptimizationLevel {
        let level =
            unsafe { ffi::WasmEdge_ConfigureCompilerGetOptimizationLevel(self.inner.inner.0) };
        level.into()
    }

    /// Sets the output binary format of AOT compiler.
    ///
    /// Notice that this function is only available when the `aot` feature is enabled.
    ///
    /// # Argument
    ///
    /// * `format` - The format of the output binary.
    pub fn set_aot_compiler_output_format(&mut self, format: CompilerOutputFormat) {
        unsafe { ffi::WasmEdge_ConfigureCompilerSetOutputFormat(self.inner.inner.0, format as u32) }
    }

    /// Returns the output binary format of AOT compiler.
    ///
    /// Notice that this function is only available when the `aot` feature is enabled.
    pub fn get_aot_compiler_output_format(&self) -> CompilerOutputFormat {
        let value = unsafe { ffi::WasmEdge_ConfigureCompilerGetOutputFormat(self.inner.inner.0) };
        value.into()
    }

    /// Sets the dump IR option of AOT compiler.
    ///
    /// Notice that this function is only available when the `aot` feature is enabled.
    ///
    /// # Argument
    ///
    /// * `flag` - Whether dump ir or not.
    #[allow(unused)]
    pub(crate) fn dump_ir(&mut self, flag: bool) {
        unsafe { ffi::WasmEdge_ConfigureCompilerSetDumpIR(self.inner.inner.0, flag) }
    }

    /// Checks if the dump IR option turns on or not.
    ///
    /// Notice that this function is only available when the `aot` feature is enabled.
    #[allow(unused)]
    pub(crate) fn dump_ir_enabled(&self) -> bool {
        unsafe { ffi::WasmEdge_ConfigureCompilerIsDumpIR(self.inner.inner.0) }
    }

    /// Sets the generic binary option of AOT compiler.
    ///
    /// Notice that this function is only available when the `aot` feature is enabled.
    ///
    /// # Argument
    ///
    /// * `flag` - Whether generate the generic binary or not when perform AOT compilation.
    pub fn generic_binary(&mut self, flag: bool) {
        unsafe { ffi::WasmEdge_ConfigureCompilerSetGenericBinary(self.inner.inner.0, flag) }
    }

    /// Checks if the generic binary option of AOT compiler turns on or not.
    ///
    /// Notice that this function is only available when the `aot` feature is enabled.
    pub fn generic_binary_enabled(&self) -> bool {
        unsafe { ffi::WasmEdge_ConfigureCompilerIsGenericBinary(self.inner.inner.0) }
    }

    /// Enables or Disables the `Interruptible` option of AOT compiler. This option determines to generate interruptible binary or not when compilation in AOT compiler.
    ///
    /// Notice that this function is only available when the `aot` feature is enabled.
    ///
    /// # Argument
    ///
    /// * `enable` - Whether turn on the `Interruptible` option.
    pub fn interruptible(&mut self, enable: bool) {
        unsafe { ffi::WasmEdge_ConfigureCompilerSetInterruptible(self.inner.inner.0, enable) }
    }

    /// Checks if the `Interruptible` option of AOT Compiler turns on or not.
    ///
    /// Notice that this function is only available when the `aot` feature is enabled.
    pub fn interruptible_enabled(&self) -> bool {
        unsafe { ffi::WasmEdge_ConfigureCompilerIsInterruptible(self.inner.inner.0) }
    }

    pub fn copy_from(src: &Self) -> WasmEdgeResult<Self> {
        let inner = Config::copy_from(&src.inner)?;
        let mut config = AotConfig { inner };
        config.dump_ir(src.dump_ir_enabled());
        config.generic_binary(src.generic_binary_enabled());
        config.set_aot_compiler_output_format(src.get_aot_compiler_output_format());
        config.set_aot_optimization_level(src.get_aot_optimization_level());
        Ok(config)
    }
}

#[derive(Debug)]
pub(crate) struct InnerCompiler(pub(crate) *mut ffi::WasmEdge_CompilerContext);
impl Drop for InnerCompiler {
    fn drop(&mut self) {
        println!("drop InnerCompiler");
        if !self.0.is_null() {
            unsafe { ffi::WasmEdge_CompilerDelete(self.0) }
        }
    }
}
unsafe impl Send for InnerCompiler {}
unsafe impl Sync for InnerCompiler {}

#[derive(Debug)]
pub struct AotCompiler {
    inner: InnerCompiler,
}

impl AotCompiler {
    pub fn create(config: &AotConfig) -> WasmEdgeResult<Self> {
        unsafe {
            let ctx = ffi::WasmEdge_CompilerCreate(config.inner.inner.0);
            if ctx.is_null() {
                Err(WasmEdgeError::CompilerCreate)
            } else {
                Ok(AotCompiler {
                    inner: InnerCompiler(ctx),
                })
            }
        }
    }

    pub fn compile<P: AsRef<Path>>(&mut self, in_path: P, out_path: P) -> WasmEdgeResult<()> {
        unsafe {
            let input = utils::path_to_cstring(in_path.as_ref())?;
            let output = utils::path_to_cstring(out_path.as_ref())?;

            utils::check(ffi::WasmEdge_CompilerCompile(
                self.inner.0,
                input.as_ptr(),
                output.as_ptr(),
            ))
        }
    }

    pub fn compile_async_module<P: AsRef<Path>>(
        &mut self,
        builder: &mut AsyncLinkerBuilder,
        wasm: &[u8],
        out_path: P,
    ) -> WasmEdgeResult<Option<std::io::Error>> {
        let new_wasm = builder.pass_asyncify_wasm(wasm)?;
        if let Err(e) = std::fs::write(&out_path, &new_wasm) {
            return Ok(Some(e));
        }
        self.compile(&out_path, &out_path)?;
        Ok(None)
    }
}
