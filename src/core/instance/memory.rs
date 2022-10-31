use crate::error::CoreError;
use crate::utils::check;
use wasmedge_sys_ffi as ffi;

/// Defines a WebAssembly memory instance, which is a linear memory described by its [type](crate::MemType). Each memory instance consists of a vector of bytes and an optional maximum size, and its size is a multiple of the WebAssembly page size (*64KiB* of each page).
#[derive(Debug)]
pub struct Memory {
    pub(crate) inner: InnerMemory,
}

impl Memory {
    pub fn from_raw(raw_ptr: *mut ffi::WasmEdge_MemoryInstanceContext) -> Self {
        Memory {
            inner: InnerMemory(raw_ptr),
        }
    }

    pub fn create(ty: MemType) -> Option<Self> {
        let ctx = unsafe { ffi::WasmEdge_MemoryInstanceCreate(ty.inner.0 as *const _) };
        ty.delete();
        if ctx.is_null() {
            None
        } else {
            Some(Memory {
                inner: InnerMemory(ctx),
            })
        }
    }

    pub fn get_type(&self) -> Option<(u32, Option<u32>, bool)> {
        let ty_ctx = unsafe { ffi::WasmEdge_MemoryInstanceGetMemoryType(self.inner.0) };
        if ty_ctx.is_null() {
            None
        } else {
            let ptr = MemType {
                inner: InnerMemType(ty_ctx as *mut _),
            };
            Some(ptr.limit())
        }
    }

    pub fn read_bytes(&self, offset: u32, len: u32) -> Result<Vec<u8>, CoreError> {
        let mut data = Vec::with_capacity(len as usize);
        unsafe {
            check(ffi::WasmEdge_MemoryInstanceGetData(
                self.inner.0,
                data.as_mut_ptr(),
                offset,
                len,
            ))?;
            data.set_len(len as usize);
        }

        Ok(data.into_iter().collect())
    }

    pub fn write_bytes<D: AsRef<[u8]>>(&mut self, data: D, offset: u32) -> Result<(), CoreError> {
        let data = data.as_ref();
        unsafe {
            check(ffi::WasmEdge_MemoryInstanceSetData(
                self.inner.0,
                data.as_ptr(),
                offset,
                data.len() as u32,
            ))
        }
    }

    pub fn data_pointer<'a>(&'a self, offset: usize, len: usize) -> Option<&'a [u8]> {
        let ptr = unsafe {
            ffi::WasmEdge_MemoryInstanceGetPointerConst(self.inner.0, offset as u32, len as u32)
        };
        if ptr.is_null() {
            None
        } else {
            Some(unsafe { std::slice::from_raw_parts(ptr, len) })
        }
    }

    pub fn data_pointer_mut<'a>(&'a mut self, offset: usize, len: usize) -> Option<&'a mut [u8]> {
        let ptr = unsafe {
            ffi::WasmEdge_MemoryInstanceGetPointer(self.inner.0, offset as u32, len as u32)
        };
        if ptr.is_null() {
            None
        } else {
            Some(unsafe { std::slice::from_raw_parts_mut(ptr, len) })
        }
    }

    #[allow(unused)]
    pub(crate) unsafe fn data_pointer_raw(&self, offset: usize, len: usize) -> Option<*const u8> {
        let ptr = unsafe {
            ffi::WasmEdge_MemoryInstanceGetPointerConst(self.inner.0, offset as u32, len as u32)
        };
        if ptr.is_null() {
            None
        } else {
            Some(ptr)
        }
    }

    #[allow(unused)]
    pub(crate) unsafe fn data_pointer_mut_raw(
        &mut self,
        offset: usize,
        len: usize,
    ) -> Option<*mut u8> {
        let ptr = unsafe {
            ffi::WasmEdge_MemoryInstanceGetPointer(self.inner.0, offset as u32, len as u32)
        };
        if ptr.is_null() {
            None
        } else {
            Some(ptr)
        }
    }

    /// Get the current page size (64 KiB of each page) of a memory instance.
    pub fn page_size(&self) -> u32 {
        unsafe { ffi::WasmEdge_MemoryInstanceGetPageSize(self.inner.0) as u32 }
    }

    pub fn grow(&mut self, count: u32) -> Result<(), CoreError> {
        unsafe { check(ffi::WasmEdge_MemoryInstanceGrowPage(self.inner.0, count)) }
    }

    pub fn delete(self) {
        unsafe { ffi::WasmEdge_MemoryInstanceDelete(self.inner.0) };
    }
}

#[derive(Debug)]
pub(crate) struct InnerMemory(pub(crate) *mut ffi::WasmEdge_MemoryInstanceContext);
unsafe impl Send for InnerMemory {}
unsafe impl Sync for InnerMemory {}

/// Defines the type of a wasm memory instance
#[derive(Debug)]
pub struct MemType {
    pub(crate) inner: InnerMemType,
}
impl MemType {
    pub fn limit(&self) -> (u32, Option<u32>, bool) {
        let limit = unsafe { ffi::WasmEdge_MemoryTypeGetLimit(self.inner.0) };
        (
            limit.Min,
            if limit.HasMax { Some(limit.Max) } else { None },
            limit.Shared,
        )
    }

    pub(crate) fn delete(self) {
        if !self.inner.0.is_null() {
            unsafe { ffi::WasmEdge_MemoryTypeDelete(self.inner.0) }
        }
    }
}

#[derive(Debug)]
pub(crate) struct InnerMemType(pub(crate) *mut ffi::WasmEdge_MemoryTypeContext);
unsafe impl Send for InnerMemType {}
unsafe impl Sync for InnerMemType {}
