//! Defines WasmEdge Instance and other relevant types.

use wasmedge_sys::ffi;
use wasmedge_types::{ValType, WasmEdgeResult};

use crate::core::ImportModule;

use crate::core::instance::function::Function;
use crate::core::types::{WasmEdgeString, WasmVal};

use super::linker::AsyncLinker;
use super::AsyncFn;

impl ImportModule {
    pub fn add_async_func(
        &mut self,
        name: &str,
        data: &mut AsyncLinker,
        ty: (Vec<ValType>, Vec<ValType>),
        real_fn: AsyncFn,
        cost: u64,
    ) -> WasmEdgeResult<()> {
        use super::instance::function::wrapper_async_fn;

        let func_name = WasmEdgeString::new(name);
        unsafe {
            let func =
                Function::custom_create(ty, wrapper_async_fn, real_fn as *mut _, data, cost)?;
            ffi::WasmEdge_ModuleInstanceAddFunction(
                self.inner.0,
                func_name.as_raw(),
                func.inner.0 as *mut _,
            );
            Ok(())
        }
    }

    pub fn add_func<T: Sized>(
        &mut self,
        name: &str,
        data: *mut T,
        ty: (Vec<ValType>, Vec<ValType>),
        real_fn: fn(&mut AsyncLinker, &[WasmVal]) -> WasmEdgeResult<Vec<WasmVal>>,
        cost: u64,
    ) -> WasmEdgeResult<()> {
        use super::instance::function::wrapper_fn;

        let func_name = WasmEdgeString::new(name);
        unsafe {
            let func = Function::custom_create(ty, wrapper_fn, real_fn as *mut _, data, cost)?;
            ffi::WasmEdge_ModuleInstanceAddFunction(
                self.inner.0,
                func_name.as_raw(),
                func.inner.0 as *mut _,
            );

            Ok(())
        }
    }
}

pub struct AsyncImportModuleBuilder<'a, 'b> {
    pub(crate) import_obj: ImportModule,
    pub(crate) linker_ctx: &'a mut AsyncLinker,
    pub(crate) async_fn_name: &'b mut Vec<String>,
}

impl AsyncImportModuleBuilder<'_, '_> {
    pub fn add_async_func(
        &mut self,
        name: &str,
        ty: (Vec<ValType>, Vec<ValType>),
        real_fn: super::AsyncFn,
    ) -> WasmEdgeResult<()> {
        self.import_obj
            .add_async_func(name, self.linker_ctx, ty, real_fn, 0)?;
        self.async_fn_name
            .push(format!("{}.{}", self.import_obj.name, name));
        Ok(())
    }

    pub fn add_func(
        &mut self,
        name: &str,
        ty: (Vec<ValType>, Vec<ValType>),
        real_fn: fn(&mut AsyncLinker, &[WasmVal]) -> WasmEdgeResult<Vec<WasmVal>>,
    ) -> WasmEdgeResult<()> {
        self.import_obj
            .add_func(name, self.linker_ctx, ty, real_fn, 0)
    }
}
