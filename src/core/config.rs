//! Defines WasmEdge Config struct.

use wasmedge_sys::ffi;
use wasmedge_types::error::WasmEdgeError;
use wasmedge_types::WasmEdgeResult;

#[derive(Debug)]
pub struct Config {
    pub(crate) inner: InnerConfig,
}
impl Drop for Config {
    fn drop(&mut self) {
        if !self.inner.0.is_null() {
            unsafe { ffi::WasmEdge_ConfigureDelete(self.inner.0) };
        }
    }
}

impl Config {
    pub fn create() -> WasmEdgeResult<Self> {
        let ctx = unsafe { ffi::WasmEdge_ConfigureCreate() };
        match ctx.is_null() {
            true => Err(WasmEdgeError::ConfigCreate),
            false => Ok(Self {
                inner: InnerConfig(ctx),
            }),
        }
    }

    pub fn copy_from(src: &Config) -> WasmEdgeResult<Self> {
        let mut config = Config::create()?;

        config.annotations(src.annotations_enabled());

        config.bulk_memory_operations(src.bulk_memory_operations_enabled());

        config.exception_handling(src.exception_handling_enabled());

        config.function_references(src.function_references_enabled());

        config.memory64(src.memory64_enabled());

        config.multi_value(src.multi_value_enabled());

        config.mutable_globals(src.mutable_globals_enabled());

        config.non_trap_conversions(src.non_trap_conversions_enabled());

        config.reference_types(src.reference_types_enabled());

        config.sign_extension_operators(src.sign_extension_operators_enabled());

        config.simd(src.simd_enabled());

        config.tail_call(src.tail_call_enabled());

        config.threads(src.threads_enabled());

        config.wasi(src.wasi_enabled());

        config.wasmedge_process(src.wasmedge_process_enabled());

        config.measure_cost(src.is_cost_measuring());

        config.count_instructions(src.is_instruction_counting());

        config.measure_time(src.is_time_measuring());

        config.set_max_memory_pages(src.get_max_memory_pages());

        #[cfg(feature = "aot")]
        {
            config.interruptible(src.interruptible_enabled());

            config.dump_ir(src.dump_ir_enabled());

            config.generic_binary(src.generic_binary_enabled());

            config.set_aot_compiler_output_format(src.get_aot_compiler_output_format());

            config.set_aot_optimization_level(src.get_aot_optimization_level());
        }

        Ok(config)
    }

    pub fn wasi(&mut self, enable: bool) {
        unsafe {
            if enable {
                ffi::WasmEdge_ConfigureAddHostRegistration(
                    self.inner.0,
                    ffi::WasmEdge_HostRegistration_Wasi,
                )
            } else {
                ffi::WasmEdge_ConfigureRemoveHostRegistration(
                    self.inner.0,
                    ffi::WasmEdge_HostRegistration_Wasi,
                )
            }
        }
    }

    pub fn wasi_enabled(&self) -> bool {
        unsafe {
            ffi::WasmEdge_ConfigureHasHostRegistration(
                self.inner.0,
                ffi::WasmEdge_HostRegistration_Wasi,
            )
        }
    }

    pub fn wasmedge_process(&mut self, enable: bool) {
        unsafe {
            if enable {
                ffi::WasmEdge_ConfigureAddHostRegistration(
                    self.inner.0,
                    ffi::WasmEdge_HostRegistration_WasmEdge_Process,
                )
            } else {
                ffi::WasmEdge_ConfigureRemoveHostRegistration(
                    self.inner.0,
                    ffi::WasmEdge_HostRegistration_WasmEdge_Process,
                )
            }
        }
    }

    pub fn wasmedge_process_enabled(&self) -> bool {
        unsafe {
            ffi::WasmEdge_ConfigureHasHostRegistration(
                self.inner.0,
                ffi::WasmEdge_HostRegistration_WasmEdge_Process,
            )
        }
    }

    pub fn set_max_memory_pages(&mut self, count: u32) {
        unsafe { ffi::WasmEdge_ConfigureSetMaxMemoryPage(self.inner.0, count) }
    }

    pub fn get_max_memory_pages(&self) -> u32 {
        unsafe { ffi::WasmEdge_ConfigureGetMaxMemoryPage(self.inner.0) }
    }

    pub fn mutable_globals(&mut self, enable: bool) {
        unsafe {
            if enable {
                ffi::WasmEdge_ConfigureAddProposal(
                    self.inner.0,
                    ffi::WasmEdge_Proposal_ImportExportMutGlobals,
                )
            } else {
                ffi::WasmEdge_ConfigureRemoveProposal(
                    self.inner.0,
                    ffi::WasmEdge_Proposal_ImportExportMutGlobals,
                )
            }
        }
    }

    pub fn mutable_globals_enabled(&self) -> bool {
        unsafe {
            ffi::WasmEdge_ConfigureHasProposal(
                self.inner.0,
                ffi::WasmEdge_Proposal_ImportExportMutGlobals,
            )
        }
    }

    pub fn non_trap_conversions(&mut self, enable: bool) {
        unsafe {
            if enable {
                ffi::WasmEdge_ConfigureAddProposal(
                    self.inner.0,
                    ffi::WasmEdge_Proposal_NonTrapFloatToIntConversions,
                )
            } else {
                ffi::WasmEdge_ConfigureRemoveProposal(
                    self.inner.0,
                    ffi::WasmEdge_Proposal_NonTrapFloatToIntConversions,
                )
            }
        }
    }

    pub fn non_trap_conversions_enabled(&self) -> bool {
        unsafe {
            ffi::WasmEdge_ConfigureHasProposal(
                self.inner.0,
                ffi::WasmEdge_Proposal_NonTrapFloatToIntConversions,
            )
        }
    }

    pub fn sign_extension_operators(&mut self, enable: bool) {
        unsafe {
            if enable {
                ffi::WasmEdge_ConfigureAddProposal(
                    self.inner.0,
                    ffi::WasmEdge_Proposal_SignExtensionOperators,
                )
            } else {
                ffi::WasmEdge_ConfigureRemoveProposal(
                    self.inner.0,
                    ffi::WasmEdge_Proposal_SignExtensionOperators,
                )
            }
        }
    }

    pub fn sign_extension_operators_enabled(&self) -> bool {
        unsafe {
            ffi::WasmEdge_ConfigureHasProposal(
                self.inner.0,
                ffi::WasmEdge_Proposal_SignExtensionOperators,
            )
        }
    }

    pub fn multi_value(&mut self, enable: bool) {
        unsafe {
            if enable {
                ffi::WasmEdge_ConfigureAddProposal(self.inner.0, ffi::WasmEdge_Proposal_MultiValue)
            } else {
                ffi::WasmEdge_ConfigureRemoveProposal(
                    self.inner.0,
                    ffi::WasmEdge_Proposal_MultiValue,
                )
            }
        }
    }

    pub fn multi_value_enabled(&self) -> bool {
        unsafe {
            ffi::WasmEdge_ConfigureHasProposal(self.inner.0, ffi::WasmEdge_Proposal_MultiValue)
        }
    }

    pub fn multi_memories(&mut self, enable: bool) {
        unsafe {
            if enable {
                ffi::WasmEdge_ConfigureAddProposal(
                    self.inner.0,
                    ffi::WasmEdge_Proposal_MultiMemories,
                )
            } else {
                ffi::WasmEdge_ConfigureRemoveProposal(
                    self.inner.0,
                    ffi::WasmEdge_Proposal_MultiMemories,
                )
            }
        }
    }

    pub fn multi_memories_enabled(&self) -> bool {
        unsafe {
            ffi::WasmEdge_ConfigureHasProposal(self.inner.0, ffi::WasmEdge_Proposal_MultiMemories)
        }
    }

    pub fn bulk_memory_operations(&mut self, enable: bool) {
        unsafe {
            if enable {
                ffi::WasmEdge_ConfigureAddProposal(
                    self.inner.0,
                    ffi::WasmEdge_Proposal_BulkMemoryOperations,
                )
            } else {
                ffi::WasmEdge_ConfigureRemoveProposal(
                    self.inner.0,
                    ffi::WasmEdge_Proposal_BulkMemoryOperations,
                )
            }
        }
    }

    pub fn bulk_memory_operations_enabled(&self) -> bool {
        unsafe {
            ffi::WasmEdge_ConfigureHasProposal(
                self.inner.0,
                ffi::WasmEdge_Proposal_BulkMemoryOperations,
            )
        }
    }

    pub fn reference_types(&mut self, enable: bool) {
        unsafe {
            if enable {
                ffi::WasmEdge_ConfigureAddProposal(
                    self.inner.0,
                    ffi::WasmEdge_Proposal_ReferenceTypes,
                )
            } else {
                ffi::WasmEdge_ConfigureRemoveProposal(
                    self.inner.0,
                    ffi::WasmEdge_Proposal_ReferenceTypes,
                )
            }
        }
    }

    /// Checks if the ReferenceTypes option turns on or not.
    pub fn reference_types_enabled(&self) -> bool {
        unsafe {
            ffi::WasmEdge_ConfigureHasProposal(self.inner.0, ffi::WasmEdge_Proposal_ReferenceTypes)
        }
    }

    /// Enables or disables the SIMD option.
    ///
    /// # Argument
    ///
    /// * `enable` - Whether the option turns on or not.
    pub fn simd(&mut self, enable: bool) {
        unsafe {
            if enable {
                ffi::WasmEdge_ConfigureAddProposal(self.inner.0, ffi::WasmEdge_Proposal_SIMD)
            } else {
                ffi::WasmEdge_ConfigureRemoveProposal(self.inner.0, ffi::WasmEdge_Proposal_SIMD)
            }
        }
    }

    /// Checks if the SIMD option turns on or not.
    pub fn simd_enabled(&self) -> bool {
        unsafe { ffi::WasmEdge_ConfigureHasProposal(self.inner.0, ffi::WasmEdge_Proposal_SIMD) }
    }

    /// Enables or disables the TailCall option.
    ///
    /// # Argument
    ///
    /// * `enable` - Whether the option turns on or not.
    pub fn tail_call(&mut self, enable: bool) {
        unsafe {
            if enable {
                ffi::WasmEdge_ConfigureAddProposal(self.inner.0, ffi::WasmEdge_Proposal_TailCall)
            } else {
                ffi::WasmEdge_ConfigureRemoveProposal(self.inner.0, ffi::WasmEdge_Proposal_TailCall)
            }
        }
    }

    /// Checks if the TailCall option turns on or not.
    pub fn tail_call_enabled(&self) -> bool {
        unsafe { ffi::WasmEdge_ConfigureHasProposal(self.inner.0, ffi::WasmEdge_Proposal_TailCall) }
    }

    /// Enables or disables the Annotations option.
    ///
    /// # Argument
    ///
    /// * `enable` - Whether the option turns on or not.
    pub fn annotations(&mut self, enable: bool) {
        unsafe {
            if enable {
                ffi::WasmEdge_ConfigureAddProposal(self.inner.0, ffi::WasmEdge_Proposal_Annotations)
            } else {
                ffi::WasmEdge_ConfigureRemoveProposal(
                    self.inner.0,
                    ffi::WasmEdge_Proposal_Annotations,
                )
            }
        }
    }

    /// Checks if the Annotations option turns on or not.
    pub fn annotations_enabled(&self) -> bool {
        unsafe {
            ffi::WasmEdge_ConfigureHasProposal(self.inner.0, ffi::WasmEdge_Proposal_Annotations)
        }
    }

    /// Enables or disables the Memory64 option.
    ///
    /// # Argument
    ///
    /// * `enable` - Whether the option turns on or not.
    pub fn memory64(&mut self, enable: bool) {
        unsafe {
            if enable {
                ffi::WasmEdge_ConfigureAddProposal(self.inner.0, ffi::WasmEdge_Proposal_Memory64)
            } else {
                ffi::WasmEdge_ConfigureRemoveProposal(self.inner.0, ffi::WasmEdge_Proposal_Memory64)
            }
        }
    }

    /// Checks if the Memory64 option turns on or not.
    pub fn memory64_enabled(&self) -> bool {
        unsafe { ffi::WasmEdge_ConfigureHasProposal(self.inner.0, ffi::WasmEdge_Proposal_Memory64) }
    }

    /// Enables or disables the Threads option.
    ///
    /// # Argument
    ///
    /// * `enable` - Whether the option turns on or not.
    pub fn threads(&mut self, enable: bool) {
        unsafe {
            if enable {
                ffi::WasmEdge_ConfigureAddProposal(self.inner.0, ffi::WasmEdge_Proposal_Threads)
            } else {
                ffi::WasmEdge_ConfigureRemoveProposal(self.inner.0, ffi::WasmEdge_Proposal_Threads)
            }
        }
    }

    /// Checks if the Threads option turns on or not.
    pub fn threads_enabled(&self) -> bool {
        unsafe { ffi::WasmEdge_ConfigureHasProposal(self.inner.0, ffi::WasmEdge_Proposal_Threads) }
    }

    /// Enables or disables the ExceptionHandling option.
    ///
    /// # Argument
    ///
    /// * `enable` - Whether the option turns on or not.
    pub fn exception_handling(&mut self, enable: bool) {
        unsafe {
            if enable {
                ffi::WasmEdge_ConfigureAddProposal(
                    self.inner.0,
                    ffi::WasmEdge_Proposal_ExceptionHandling,
                )
            } else {
                ffi::WasmEdge_ConfigureRemoveProposal(
                    self.inner.0,
                    ffi::WasmEdge_Proposal_ExceptionHandling,
                )
            }
        }
    }

    /// Checks if the ExceptionHandling option turns on or not.
    pub fn exception_handling_enabled(&self) -> bool {
        unsafe {
            ffi::WasmEdge_ConfigureHasProposal(
                self.inner.0,
                ffi::WasmEdge_Proposal_ExceptionHandling,
            )
        }
    }

    /// Enables or disables the FunctionReferences option.
    ///
    /// # Argument
    ///
    /// * `enable` - Whether the option turns on or not.
    pub fn function_references(&mut self, enable: bool) {
        unsafe {
            if enable {
                ffi::WasmEdge_ConfigureAddProposal(
                    self.inner.0,
                    ffi::WasmEdge_Proposal_FunctionReferences,
                )
            } else {
                ffi::WasmEdge_ConfigureRemoveProposal(
                    self.inner.0,
                    ffi::WasmEdge_Proposal_FunctionReferences,
                )
            }
        }
    }

    /// Checks if the FunctionReferences option turns on or not.
    pub fn function_references_enabled(&self) -> bool {
        unsafe {
            ffi::WasmEdge_ConfigureHasProposal(
                self.inner.0,
                ffi::WasmEdge_Proposal_FunctionReferences,
            )
        }
    }

    // For AOT compiler

    /// Sets the optimization level of AOT compiler.
    ///
    /// Notice that this function is only available when the `aot` feature is enabled.
    ///
    /// # Argument
    ///
    /// * `opt_level` - The optimization level of AOT compiler.
    #[cfg(feature = "aot")]
    pub fn set_aot_optimization_level(&mut self, opt_level: CompilerOptimizationLevel) {
        unsafe {
            ffi::WasmEdge_ConfigureCompilerSetOptimizationLevel(self.inner.0, opt_level as u32)
        }
    }

    /// Returns the optimization level of AOT compiler.
    ///
    /// Notice that this function is only available when the `aot` feature is enabled.
    #[cfg(feature = "aot")]
    pub fn get_aot_optimization_level(&self) -> CompilerOptimizationLevel {
        let level = unsafe { ffi::WasmEdge_ConfigureCompilerGetOptimizationLevel(self.inner.0) };
        level.into()
    }

    /// Sets the output binary format of AOT compiler.
    ///
    /// Notice that this function is only available when the `aot` feature is enabled.
    ///
    /// # Argument
    ///
    /// * `format` - The format of the output binary.
    #[cfg(feature = "aot")]
    pub fn set_aot_compiler_output_format(&mut self, format: CompilerOutputFormat) {
        unsafe { ffi::WasmEdge_ConfigureCompilerSetOutputFormat(self.inner.0, format as u32) }
    }

    /// Returns the output binary format of AOT compiler.
    ///
    /// Notice that this function is only available when the `aot` feature is enabled.
    #[cfg(feature = "aot")]
    pub fn get_aot_compiler_output_format(&self) -> CompilerOutputFormat {
        let value = unsafe { ffi::WasmEdge_ConfigureCompilerGetOutputFormat(self.inner.0) };
        value.into()
    }

    /// Sets the dump IR option of AOT compiler.
    ///
    /// Notice that this function is only available when the `aot` feature is enabled.
    ///
    /// # Argument
    ///
    /// * `flag` - Whether dump ir or not.
    #[cfg(feature = "aot")]
    pub fn dump_ir(&mut self, flag: bool) {
        unsafe { ffi::WasmEdge_ConfigureCompilerSetDumpIR(self.inner.0, flag) }
    }

    /// Checks if the dump IR option turns on or not.
    ///
    /// Notice that this function is only available when the `aot` feature is enabled.
    #[cfg(feature = "aot")]
    pub fn dump_ir_enabled(&self) -> bool {
        unsafe { ffi::WasmEdge_ConfigureCompilerIsDumpIR(self.inner.0) }
    }

    /// Sets the generic binary option of AOT compiler.
    ///
    /// Notice that this function is only available when the `aot` feature is enabled.
    ///
    /// # Argument
    ///
    /// * `flag` - Whether generate the generic binary or not when perform AOT compilation.
    #[cfg(feature = "aot")]
    pub fn generic_binary(&mut self, flag: bool) {
        unsafe { ffi::WasmEdge_ConfigureCompilerSetGenericBinary(self.inner.0, flag) }
    }

    /// Checks if the generic binary option of AOT compiler turns on or not.
    ///
    /// Notice that this function is only available when the `aot` feature is enabled.
    #[cfg(feature = "aot")]
    pub fn generic_binary_enabled(&self) -> bool {
        unsafe { ffi::WasmEdge_ConfigureCompilerIsGenericBinary(self.inner.0) }
    }

    /// Enables or Disables the `Interruptible` option of AOT compiler. This option determines to generate interruptible binary or not when compilation in AOT compiler.
    ///
    /// Notice that this function is only available when the `aot` feature is enabled.
    ///
    /// # Argument
    ///
    /// * `enable` - Whether turn on the `Interruptible` option.
    #[cfg(feature = "aot")]
    pub fn interruptible(&mut self, enable: bool) {
        unsafe { ffi::WasmEdge_ConfigureCompilerSetInterruptible(self.inner.0, enable) }
    }

    /// Checks if the `Interruptible` option of AOT Compiler turns on or not.
    ///
    /// Notice that this function is only available when the `aot` feature is enabled.
    #[cfg(feature = "aot")]
    pub fn interruptible_enabled(&self) -> bool {
        unsafe { ffi::WasmEdge_ConfigureCompilerIsInterruptible(self.inner.0) }
    }

    // For Statistics

    /// Sets the instruction counting option.
    ///
    /// # Argument
    ///
    /// * `flag` - Whether support instruction counting or not when execution after AOT compilation.
    pub fn count_instructions(&mut self, flag: bool) {
        unsafe { ffi::WasmEdge_ConfigureStatisticsSetInstructionCounting(self.inner.0, flag) }
    }

    /// Checks if the instruction counting option turns on or not.
    pub fn is_instruction_counting(&self) -> bool {
        unsafe { ffi::WasmEdge_ConfigureStatisticsIsInstructionCounting(self.inner.0) }
    }

    /// Sets the cost measuring option.
    ///
    /// # Argument
    ///
    /// * `flag` - Whether support cost measuring or not when execution after AOT compilation.
    pub fn measure_cost(&mut self, flag: bool) {
        unsafe { ffi::WasmEdge_ConfigureStatisticsSetCostMeasuring(self.inner.0, flag) }
    }

    /// Checks if the cost measuring option turns on or not.
    pub fn is_cost_measuring(&self) -> bool {
        unsafe { ffi::WasmEdge_ConfigureStatisticsIsCostMeasuring(self.inner.0) }
    }

    /// Sets the time measuring option.
    ///
    /// # Argument
    ///
    /// * `flag` - Whether support time measuring or not when execution after AOT compilation.
    pub fn measure_time(&mut self, flag: bool) {
        unsafe { ffi::WasmEdge_ConfigureStatisticsSetTimeMeasuring(self.inner.0, flag) }
    }

    /// Checks if the time measuring option turns on or not.
    pub fn is_time_measuring(&self) -> bool {
        unsafe { ffi::WasmEdge_ConfigureStatisticsIsTimeMeasuring(self.inner.0) }
    }
}

#[derive(Debug)]
pub(crate) struct InnerConfig(pub(crate) *mut ffi::WasmEdge_ConfigureContext);
unsafe impl Send for InnerConfig {}
unsafe impl Sync for InnerConfig {}
