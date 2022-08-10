//! Defines WasmEdge Instance and other relevant types.

use std::ffi::CString;
use std::os::raw::c_char;
use wasmedge_sys::ffi;
use wasmedge_types::error::{InstanceError, WasmEdgeError};
use wasmedge_types::WasmEdgeResult;

use super::{
    instance::{function::FuncRef, memory::Memory},
    instance::{function::InnerFunc, memory::InnerMemory},
    types::WasmEdgeString,
};

trait AsInnerInstance {
    unsafe fn get_mut_ptr(&self) -> *mut ffi::WasmEdge_ModuleInstanceContext;
}

#[derive(Debug)]
pub struct Instance {
    pub(crate) inner: InnerInstance,
}

impl AsInnerInstance for Instance {
    unsafe fn get_mut_ptr(&self) -> *mut ffi::WasmEdge_ModuleInstanceContext {
        self.inner.0
    }
}

impl Instance {
    pub fn name(&self) -> Option<String> {
        let name = unsafe { ffi::WasmEdge_ModuleInstanceGetModuleName(self.inner.0 as *const _) };

        let name: String = name.into();
        if name.is_empty() {
            return None;
        }

        Some(name)
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
    fn get_func(&self, name: impl AsRef<str>) -> WasmEdgeResult<FuncRef>;

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
    fn get_memory(&self, name: &str) -> WasmEdgeResult<Memory>;

    /// Returns the length of the exported [memory instances](crate::Memory) in this module instance.
    fn mem_len(&self) -> u32;

    /// Returns the names of all exported [memory instances](crate::Memory) in this module instance.
    fn mem_names(&self) -> Option<Vec<String>>;
}

#[derive(Debug)]
pub struct ImportModule {
    pub(crate) inner: InnerInstance,
    pub(crate) name: String,
}

impl ImportModule {
    pub fn create<S: AsRef<str>>(name: S) -> WasmEdgeResult<Self> {
        let raw_name = WasmEdgeString::new(name.as_ref())?;
        let ctx = unsafe { ffi::WasmEdge_ModuleInstanceCreate(raw_name.as_raw()) };

        match ctx.is_null() {
            true => Err(WasmEdgeError::Instance(InstanceError::CreateImportModule)),
            false => Ok(Self {
                inner: InnerInstance(ctx),
                name: name.as_ref().to_string(),
            }),
        }
    }

    pub fn create_wasi<S: AsRef<str>>(
        args: &[S],
        envs: &[S],
        preopens: &[S],
    ) -> WasmEdgeResult<Self> {
        fn to_cstring_vec<S: AsRef<str>>(s: &[S]) -> Vec<CString> {
            let mut r = vec![];
            for s in s {
                if let Ok(cs) = CString::new(s.as_ref()) {
                    r.push(cs);
                }
            }
            r
        }
        fn cstring_vec_to_ptr(s: &[CString]) -> Vec<*const c_char> {
            let mut r = vec![];
            for cs in s {
                r.push(cs.as_ptr())
            }
            r
        }

        let args = to_cstring_vec(args);
        let args_ptrs = cstring_vec_to_ptr(&args);
        let args_len = args.len();

        let envs = to_cstring_vec(envs);
        let envs_ptrs = cstring_vec_to_ptr(&envs);
        let envs_len = envs.len();

        let preopens = to_cstring_vec(preopens);
        let preopens_ptrs = cstring_vec_to_ptr(&preopens);
        let preopens_len = preopens.len();

        let ctx = unsafe {
            ffi::WasmEdge_ModuleInstanceCreateWASI(
                args_ptrs.as_ptr(),
                args_len as u32,
                envs_ptrs.as_ptr(),
                envs_len as u32,
                preopens_ptrs.as_ptr(),
                preopens_len as u32,
            )
        };
        match ctx.is_null() {
            true => Err(WasmEdgeError::ImportObjCreate),
            false => Ok(Self {
                inner: InnerInstance(ctx),
                name: String::from("wasi_snapshot_preview1"),
            }),
        }
    }

    pub fn name(&self) -> String {
        self.name.to_owned()
    }
}

impl AsInnerInstance for ImportModule {
    unsafe fn get_mut_ptr(&self) -> *mut ffi::WasmEdge_ModuleInstanceContext {
        self.inner.0
    }
}

impl<T: AsInnerInstance> AsInstance for T {
    fn get_func(&self, name: impl AsRef<str>) -> WasmEdgeResult<FuncRef> {
        let func_name = WasmEdgeString::new(name.as_ref())?;
        let func_ctx = unsafe {
            ffi::WasmEdge_ModuleInstanceFindFunction(self.get_mut_ptr(), func_name.as_raw())
        };
        match func_ctx.is_null() {
            true => Err(WasmEdgeError::Instance(InstanceError::NotFoundFunc(
                name.as_ref().to_string(),
            ))),
            false => Ok(FuncRef {
                inner: InnerFunc(func_ctx),
            }),
        }
    }

    fn get_memory(&self, name: &str) -> WasmEdgeResult<Memory> {
        let mem_name: WasmEdgeString = WasmEdgeString::new(name)?;
        let ctx = unsafe {
            ffi::WasmEdge_ModuleInstanceFindMemory(self.get_mut_ptr(), mem_name.as_raw())
        };
        match ctx.is_null() {
            true => Err(WasmEdgeError::Instance(InstanceError::NotFoundMem(
                name.to_string(),
            ))),
            false => Ok(Memory {
                inner: InnerMemory(ctx),
            }),
        }
    }

    /// Returns the length of the exported [function instances](crate::Function) in this module instance.
    fn func_len(&self) -> u32 {
        unsafe { ffi::WasmEdge_ModuleInstanceListFunctionLength(self.get_mut_ptr()) }
    }

    /// Returns the names of the exported [function instances](crate::Function) in this module instance.
    fn func_names(&self) -> Option<Vec<String>> {
        let len_func_names = self.func_len();
        match len_func_names > 0 {
            true => {
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
                    .map(|x| x.into())
                    .collect::<Vec<String>>();
                Some(names)
            }
            false => None,
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
                    .map(|x| x.into())
                    .collect::<Vec<String>>();
                Some(names)
            }
            false => None,
        }
    }
}
