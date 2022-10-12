use std::{
    ops::{Deref, DerefMut},
    path::Path,
};
use thiserror::Error;

use crate::{core::pass_async_module, core::CodegenConfig, error::CoreError, utils, Config};
use wasmedge_sys_ffi as ffi;

/// Defines WasmEdge AOT compiler optimization level.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CompilerOptimizationLevel(u32);

impl CompilerOptimizationLevel {
    /// Disable as many optimizations as possible.
    pub const O0: Self = CompilerOptimizationLevel(0);

    /// Optimize quickly without destroying debuggability.
    pub const O1: Self = CompilerOptimizationLevel(1);

    /// Optimize for fast execution as much as possible without triggering significant incremental compile time or code size growth.
    pub const O2: Self = CompilerOptimizationLevel(2);

    ///  Optimize for fast execution as much as possible.
    pub const O3: Self = CompilerOptimizationLevel(3);

    ///  Optimize for small code size as much as possible without triggering
    ///  significant incremental compile time or execution time slowdowns.
    #[allow(non_upper_case_globals)]
    pub const Os: Self = CompilerOptimizationLevel(4);

    /// Optimize for small code size as much as possible.
    #[allow(non_upper_case_globals)]
    pub const Oz: Self = CompilerOptimizationLevel(5);
}
impl From<u32> for CompilerOptimizationLevel {
    fn from(val: u32) -> CompilerOptimizationLevel {
        match val {
            0 => CompilerOptimizationLevel::O0,
            1 => CompilerOptimizationLevel::O1,
            2 => CompilerOptimizationLevel::O2,
            3 => CompilerOptimizationLevel::O3,
            4 => CompilerOptimizationLevel::Os,
            5 => CompilerOptimizationLevel::Oz,
            _ => panic!("Unknown CompilerOptimizationLevel value: {}", val),
        }
    }
}
impl From<CompilerOptimizationLevel> for u32 {
    fn from(val: CompilerOptimizationLevel) -> u32 {
        val.0
    }
}
impl From<i32> for CompilerOptimizationLevel {
    fn from(val: i32) -> CompilerOptimizationLevel {
        match val {
            0 => CompilerOptimizationLevel::O0,
            1 => CompilerOptimizationLevel::O1,
            2 => CompilerOptimizationLevel::O2,
            3 => CompilerOptimizationLevel::O3,
            4 => CompilerOptimizationLevel::Os,
            5 => CompilerOptimizationLevel::Oz,
            _ => panic!("Unknown CompilerOptimizationLevel value: {}", val),
        }
    }
}
impl From<CompilerOptimizationLevel> for i32 {
    fn from(val: CompilerOptimizationLevel) -> i32 {
        val.0 as i32
    }
}

/// Defines WasmEdge AOT compiler output binary format.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CompilerOutputFormat {
    /// Native dynamic library format.
    Native,

    /// WebAssembly with AOT compiled codes in custom sections.
    Wasm,
}
impl From<u32> for CompilerOutputFormat {
    fn from(val: u32) -> CompilerOutputFormat {
        match val {
            0 => CompilerOutputFormat::Native,
            1 => CompilerOutputFormat::Wasm,
            _ => panic!("Unknown CompilerOutputFormat value: {}", val),
        }
    }
}
impl From<CompilerOutputFormat> for u32 {
    fn from(val: CompilerOutputFormat) -> u32 {
        match val {
            CompilerOutputFormat::Native => 0,
            CompilerOutputFormat::Wasm => 1,
        }
    }
}
impl From<i32> for CompilerOutputFormat {
    fn from(val: i32) -> CompilerOutputFormat {
        match val {
            0 => CompilerOutputFormat::Native,
            1 => CompilerOutputFormat::Wasm,
            _ => panic!("Unknown CompilerOutputFormat value: {}", val),
        }
    }
}
impl From<CompilerOutputFormat> for i32 {
    fn from(val: CompilerOutputFormat) -> i32 {
        match val {
            CompilerOutputFormat::Native => 0,
            CompilerOutputFormat::Wasm => 1,
        }
    }
}

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
    pub fn create() -> Option<Self> {
        let mut config = AotConfig {
            inner: Config::create()?,
        };
        config.dump_ir(false);
        config.set_aot_compiler_output_format(CompilerOutputFormat::Wasm);
        Some(config)
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
            ffi::WasmEdge_ConfigureCompilerSetOptimizationLevel(self.inner.inner.0, opt_level.0)
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

    pub fn copy_from(src: &Self) -> Option<Self> {
        let inner = Config::copy_from(&src.inner)?;
        let mut config = AotConfig { inner };
        config.dump_ir(src.dump_ir_enabled());
        config.generic_binary(src.generic_binary_enabled());
        config.set_aot_compiler_output_format(src.get_aot_compiler_output_format());
        config.set_aot_optimization_level(src.get_aot_optimization_level());
        Some(config)
    }
}

#[derive(Debug)]
pub(crate) struct InnerCompiler(pub(crate) *mut ffi::WasmEdge_CompilerContext);
impl Drop for InnerCompiler {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe { ffi::WasmEdge_CompilerDelete(self.0) }
        }
    }
}
unsafe impl Send for InnerCompiler {}
unsafe impl Sync for InnerCompiler {}

#[derive(Error, Clone, Debug, PartialEq, Eq)]
pub enum AotCompileError {
    #[error("Output path Error")]
    PathError(#[from] std::ffi::NulError),
    #[error("{0}")]
    CoreError(#[from] CoreError),
    #[error("Pass Asyncify Error")]
    PassAsyncifyError,
}

#[derive(Debug)]
pub struct AotCompiler {
    inner: InnerCompiler,
}

impl AotCompiler {
    pub fn create(config: &AotConfig) -> Option<Self> {
        unsafe {
            let ctx = ffi::WasmEdge_CompilerCreate(config.inner.inner.0);
            if ctx.is_null() {
                None
            } else {
                Some(AotCompiler {
                    inner: InnerCompiler(ctx),
                })
            }
        }
    }

    pub fn compile<P: AsRef<Path>>(
        &mut self,
        wasm_bytes: &[u8],
        out_path: P,
    ) -> Result<(), AotCompileError> {
        unsafe {
            let output = utils::path_to_cstring(out_path.as_ref())?;

            utils::check(ffi::WasmEdge_CompilerCompileFromBuffer(
                self.inner.0,
                wasm_bytes.as_ptr(),
                wasm_bytes.len() as u64,
                output.as_ptr(),
            ))?;
            Ok(())
        }
    }

    pub fn compile_async_module<P: AsRef<Path>>(
        &mut self,
        wasm: &[u8],
        out_path: P,
    ) -> Result<(), AotCompileError> {
        let mut codegen_config = CodegenConfig::default();
        codegen_config.optimization_level = 2;
        codegen_config
            .pass_argument
            .push(("asyncify-imports".to_string(), "*.async_".to_string()));

        let new_wasm = pass_async_module(wasm, ["asyncify", "strip"], &codegen_config)
            .ok_or(AotCompileError::PassAsyncifyError)?;

        self.compile(&new_wasm, &out_path)?;
        Ok(())
    }
}
