//! Defines WasmEdge Instance and other relevant types.

use wasmedge_sys_ffi as ffi;

use crate::error::InstanceError;

use super::{
    instance::{function::FuncRef, memory::Memory},
    instance::{function::InnerFunc, memory::InnerMemory},
    types::WasmEdgeString,
};

pub(crate) trait AsInnerInstance {
    unsafe fn get_mut_ptr(&self) -> *mut ffi::WasmEdge_ModuleInstanceContext;
}

impl AsInnerInstance for InnerInstance {
    unsafe fn get_mut_ptr(&self) -> *mut ffi::WasmEdge_ModuleInstanceContext {
        self.0
    }
}

#[derive(Debug)]
pub(crate) struct InnerInstance(pub(crate) *mut ffi::WasmEdge_ModuleInstanceContext);
impl Drop for InnerInstance {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe {
                ffi::WasmEdge_ModuleInstanceDelete(self.0);
            }
        }
    }
}
unsafe impl Send for InnerInstance {}
unsafe impl Sync for InnerInstance {}

pub(crate) trait AsInstance {
    /// Returns the exported [function instance](crate::Function) by name.
    ///
    /// # Argument
    ///
    /// * `name` - The name of the target exported [function instance](crate::Function).
    ///
    /// # Error
    ///
    /// If fail to find the target [function](crate::Function), then an error is returned.
    fn get_func(&self, name: impl AsRef<str>) -> Result<FuncRef, InstanceError>;

    /// Returns the length of the exported [function instances](crate::Function) in this module instance.
    fn func_len(&self) -> u32;

    /// Returns the names of the exported [function instances](crate::Function) in this module instance.
    fn func_names(&self) -> Option<Vec<String>>;

    /// Returns the exported [memory instance](crate::Memory) by name.
    ///
    /// # Argument
    ///
    /// * `name` - The name of the target exported [memory instance](crate::Memory).
    ///
    /// # Error
    ///
    /// If fail to find the target [memory instance](crate::Memory), then an error is returned.
    fn get_memory(&self, name: &str) -> Result<Memory, InstanceError>;

    /// Returns the length of the exported [memory instances](crate::Memory) in this module instance.
    fn mem_len(&self) -> u32;

    /// Returns the names of all exported [memory instances](crate::Memory) in this module instance.
    fn mem_names(&self) -> Option<Vec<String>>;
}

#[derive(Debug)]
pub struct ImportModule<T: Sized + Send> {
    pub(crate) inner: InnerInstance,
    pub name: String,
    pub data: Box<T>,
}

impl<T: Sized + Send> ImportModule<T> {
    pub fn create<S: AsRef<str>>(name: S, data: T) -> Result<Self, InstanceError> {
        let raw_name = WasmEdgeString::new(name.as_ref())?;
        let ctx = unsafe { ffi::WasmEdge_ModuleInstanceCreate(raw_name.as_raw()) };

        match ctx.is_null() {
            true => Err(InstanceError::CreateImportModule),
            false => Ok(Self {
                inner: InnerInstance(ctx),
                name: name.as_ref().to_string(),
                data: Box::new(data),
            }),
        }
    }

    pub fn name(&self) -> String {
        self.name.to_owned()
    }

    pub fn unpack(self) -> Box<T> {
        self.data
    }
}

impl<T: Sized + Send> AsInnerInstance for ImportModule<T> {
    unsafe fn get_mut_ptr(&self) -> *mut ffi::WasmEdge_ModuleInstanceContext {
        self.inner.0
    }
}

impl<T: AsInnerInstance> AsInstance for T {
    fn get_func(&self, name: impl AsRef<str>) -> Result<FuncRef, InstanceError> {
        let func_name = WasmEdgeString::new(name.as_ref())?;
        let func_ctx = unsafe {
            ffi::WasmEdge_ModuleInstanceFindFunction(self.get_mut_ptr(), func_name.as_raw())
        };
        if func_ctx.is_null() {
            Err(InstanceError::NotFoundFunc(name.as_ref().to_string()))
        } else {
            Ok(FuncRef {
                inner: InnerFunc(func_ctx),
            })
        }
    }

    fn get_memory(&self, name: &str) -> Result<Memory, InstanceError> {
        let mem_name: WasmEdgeString = WasmEdgeString::new(name)?;
        let ctx = unsafe {
            ffi::WasmEdge_ModuleInstanceFindMemory(self.get_mut_ptr(), mem_name.as_raw())
        };
        if ctx.is_null() {
            Err(InstanceError::NotFoundMem(name.to_string()))
        } else {
            Ok(Memory {
                inner: InnerMemory(ctx),
            })
        }
    }

    /// Returns the length of the exported [function instances](crate::Function) in this module instance.
    fn func_len(&self) -> u32 {
        unsafe { ffi::WasmEdge_ModuleInstanceListFunctionLength(self.get_mut_ptr()) }
    }

    /// Returns the names of the exported [function instances](crate::Function) in this module instance.
    fn func_names(&self) -> Option<Vec<String>> {
        let len_func_names = self.func_len();
        if len_func_names > 0 {
            let mut func_names = Vec::with_capacity(len_func_names as usize);
            unsafe {
                ffi::WasmEdge_ModuleInstanceListFunction(
                    self.get_mut_ptr(),
                    func_names.as_mut_ptr(),
                    len_func_names,
                );
                func_names.set_len(len_func_names as usize);
            }

            let names = func_names
                .into_iter()
                .map(|x| {
                    let r: Result<String, std::str::Utf8Error> = x.into();
                    r.unwrap_or_default()
                })
                .collect::<Vec<String>>();
            Some(names)
        } else {
            None
        }
    }

    /// Returns the length of the exported [memory instances](crate::Memory) in this module instance.
    fn mem_len(&self) -> u32 {
        unsafe { ffi::WasmEdge_ModuleInstanceListMemoryLength(self.get_mut_ptr()) }
    }

    /// Returns the names of all exported [memory instances](crate::Memory) in this module instance.
    fn mem_names(&self) -> Option<Vec<String>> {
        let len_mem_names = self.mem_len();
        match len_mem_names > 0 {
            true => {
                let mut mem_names = Vec::with_capacity(len_mem_names as usize);
                unsafe {
                    ffi::WasmEdge_ModuleInstanceListMemory(
                        self.get_mut_ptr(),
                        mem_names.as_mut_ptr(),
                        len_mem_names,
                    );
                    mem_names.set_len(len_mem_names as usize);
                }

                let names = mem_names
                    .into_iter()
                    .map(|x| {
                        let r: Result<String, std::str::Utf8Error> = x.into();
                        r.unwrap_or_default()
                    })
                    .collect::<Vec<String>>();
                Some(names)
            }
            false => None,
        }
    }
}
