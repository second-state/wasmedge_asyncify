use crate::utils::check;
use wasmedge_sys::ffi;
use wasmedge_types::error::{MemError, WasmEdgeError};
use wasmedge_types::{MemoryType, WasmEdgeResult};

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

    pub fn create(ty: MemType) -> WasmEdgeResult<Self> {
        let ctx = unsafe { ffi::WasmEdge_MemoryInstanceCreate(ty.inner.0 as *const _) };
        ty.delete();
        match ctx.is_null() {
            true => Err(WasmEdgeError::Mem(MemError::Create)),
            false => Ok(Memory {
                inner: InnerMemory(ctx),
            }),
        }
    }

    pub fn get_type(&self) -> WasmEdgeResult<(u32, Option<u32>, bool)> {
        let ty_ctx = unsafe { ffi::WasmEdge_MemoryInstanceGetMemoryType(self.inner.0) };
        if ty_ctx.is_null() {
            Err(WasmEdgeError::Mem(MemError::Type))
        } else {
            let ptr = MemType {
                inner: InnerMemType(ty_ctx as *mut _),
            };
            Ok(ptr.limit())
        }
    }

    pub fn get_data(&self, offset: u32, len: u32) -> WasmEdgeResult<Vec<u8>> {
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

    pub fn set_data<D: AsRef<[u8]>>(&mut self, data: D, offset: u32) -> WasmEdgeResult<()> {
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

    pub fn data_pointer<'a>(&'a self, offset: usize, len: usize) -> WasmEdgeResult<&'a [u8]> {
        let ptr = unsafe {
            ffi::WasmEdge_MemoryInstanceGetPointerConst(self.inner.0, offset as u32, len as u32)
        };
        if ptr.is_null() {
            Err(WasmEdgeError::Mem(MemError::ConstPtr))
        } else {
            Ok(unsafe { std::slice::from_raw_parts(ptr, len) })
        }
    }

    pub fn data_pointer_mut<'a>(
        &'a mut self,
        offset: usize,
        len: usize,
    ) -> WasmEdgeResult<&'a mut [u8]> {
        let ptr = unsafe {
            ffi::WasmEdge_MemoryInstanceGetPointer(self.inner.0, offset as u32, len as u32)
        };
        if ptr.is_null() {
            Err(WasmEdgeError::Mem(MemError::MutPtr))
        } else {
            Ok(unsafe { std::slice::from_raw_parts_mut(ptr, len) })
        }
    }

    #[allow(unused)]
    pub(crate) unsafe fn data_pointer_raw(
        &self,
        offset: usize,
        len: usize,
    ) -> WasmEdgeResult<*const u8> {
        let ptr = unsafe {
            ffi::WasmEdge_MemoryInstanceGetPointerConst(self.inner.0, offset as u32, len as u32)
        };
        if ptr.is_null() {
            Err(WasmEdgeError::Mem(MemError::ConstPtr))
        } else {
            Ok(ptr)
        }
    }

    #[allow(unused)]
    pub(crate) unsafe fn data_pointer_mut_raw(
        &mut self,
        offset: usize,
        len: usize,
    ) -> WasmEdgeResult<*mut u8> {
        let ptr = unsafe {
            ffi::WasmEdge_MemoryInstanceGetPointer(self.inner.0, offset as u32, len as u32)
        };
        if ptr.is_null() {
            Err(WasmEdgeError::Mem(MemError::MutPtr))
        } else {
            Ok(ptr)
        }
    }

    pub fn size(&self) -> u32 {
        unsafe { ffi::WasmEdge_MemoryInstanceGetPageSize(self.inner.0) as u32 }
    }

    pub fn grow(&mut self, count: u32) -> WasmEdgeResult<()> {
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
    pub fn create(min: u32, max: Option<u32>, shared: bool) -> WasmEdgeResult<Self> {
        let ty = MemoryType::new(min, max, shared)?;
        let ctx = unsafe {
            ffi::WasmEdge_MemoryTypeCreate(ffi::WasmEdge_Limit {
                HasMax: ty.maximum().is_some(),
                Shared: ty.shared(),
                Min: ty.minimum(),
                Max: ty.maximum().unwrap_or(ty.minimum()),
            })
        };
        match ctx.is_null() {
            true => Err(WasmEdgeError::MemTypeCreate),
            false => Ok(Self {
                inner: InnerMemType(ctx),
            }),
        }
    }

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

// impl From<wasmedge_types::MemoryType> for WasmEdgeResult<MemType> {
//     fn from(ty: wasmedge_types::MemoryType) -> Self {
//         MemType::create(ty.minimum(), ty.maximum(), ty.shared()).expect(
//             "[wasmedge] Failed to convert wasmedge_types::MemoryType into wasmedge_sys::MemType.",
//         )
//     }
// }

impl From<MemType> for wasmedge_types::MemoryType {
    fn from(ty: MemType) -> Self {
        let limit = ty.limit();
        wasmedge_types::MemoryType::new(limit.0, limit.1, limit.2).unwrap()
    }
}

#[derive(Debug)]
pub(crate) struct InnerMemType(pub(crate) *mut ffi::WasmEdge_MemoryTypeContext);
unsafe impl Send for InnerMemType {}
unsafe impl Sync for InnerMemType {}
