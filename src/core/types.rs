use std::ffi::CString;

use wasmedge_sys::ffi;
use wasmedge_types::ValType;

use super::instance::function::{FuncRef, InnerFunc};

/// Struct of WasmEdge String.
#[derive(Debug)]
pub(crate) struct WasmEdgeString {
    inner: InnerWasmEdgeString,
}
impl Drop for InnerWasmEdgeString {
    fn drop(&mut self) {
        unsafe { ffi::WasmEdge_StringDelete(self.0) }
    }
}
impl WasmEdgeString {
    pub fn new(s: &str) -> Self {
        let cs = CString::new(s).unwrap_or_default();
        let ctx = unsafe { ffi::WasmEdge_StringCreateByCString(cs.as_ptr()) };

        Self {
            inner: InnerWasmEdgeString(ctx),
        }
    }
    pub(crate) fn as_raw(&self) -> ffi::WasmEdge_String {
        self.inner.0
    }
}
impl PartialEq for WasmEdgeString {
    fn eq(&self, other: &Self) -> bool {
        unsafe { ffi::WasmEdge_StringIsEqual(self.inner.0, other.inner.0) }
    }
}
impl Eq for WasmEdgeString {}
impl AsRef<str> for WasmEdgeString {
    fn as_ref(&self) -> &str {
        unsafe {
            let bs = std::slice::from_raw_parts(
                self.as_raw().Buf as *const u8,
                self.as_raw().Length as usize,
            );
            std::str::from_utf8_unchecked(bs)
        }
    }
}
impl From<WasmEdgeString> for String {
    fn from(s: WasmEdgeString) -> Self {
        let bytes: &[u8] = unsafe {
            std::slice::from_raw_parts(s.as_raw().Buf as *const u8, s.as_raw().Length as usize)
        };

        String::from_utf8(bytes.to_vec()).unwrap_or_default()
    }
}

#[derive(Debug)]
pub(crate) struct InnerWasmEdgeString(pub(crate) ffi::WasmEdge_String);
unsafe impl Send for InnerWasmEdgeString {}
unsafe impl Sync for InnerWasmEdgeString {}

#[derive(Debug, Clone)]
pub struct Extern {
    ctx: *mut std::ffi::c_void,
}

impl Extern {
    pub unsafe fn new<T>(ptr: *mut T) -> Self {
        Extern { ctx: ptr.cast() }
    }

    pub const fn cast<T>(&self) -> *mut T {
        self.ctx.cast()
    }
}

#[derive(Debug, Clone)]
pub enum WasmVal {
    I32(i32),
    I64(i64),
    F32(f32),
    F64(f64),
    V128(i128),
    FuncRef(FuncRef),
    ExternRef(Extern),
    None,
}

impl From<ffi::WasmEdge_Value> for WasmVal {
    fn from(raw_val: ffi::WasmEdge_Value) -> Self {
        unsafe {
            match raw_val.Type {
                ffi::WasmEdge_ValType_I32 => WasmVal::I32(ffi::WasmEdge_ValueGetI32(raw_val)),
                ffi::WasmEdge_ValType_I64 => WasmVal::I64(ffi::WasmEdge_ValueGetI64(raw_val)),
                ffi::WasmEdge_ValType_F32 => WasmVal::F32(ffi::WasmEdge_ValueGetF32(raw_val)),
                ffi::WasmEdge_ValType_F64 => WasmVal::F64(ffi::WasmEdge_ValueGetF64(raw_val)),
                ffi::WasmEdge_ValType_V128 => WasmVal::V128(ffi::WasmEdge_ValueGetV128(raw_val)),
                ffi::WasmEdge_ValType_FuncRef => {
                    let func_ref = ffi::WasmEdge_ValueGetFuncRef(raw_val);
                    WasmVal::FuncRef(FuncRef {
                        inner: InnerFunc(func_ref),
                    })
                }
                ffi::WasmEdge_ValType_ExternRef => {
                    let ctx = ffi::WasmEdge_ValueGetExternRef(raw_val);
                    WasmVal::ExternRef(Extern { ctx })
                }
                _ => WasmVal::None,
            }
        }
    }
}

impl Into<ffi::WasmEdge_Value> for WasmVal {
    fn into(self) -> ffi::WasmEdge_Value {
        unsafe {
            match self {
                WasmVal::I32(n) => ffi::WasmEdge_ValueGenI32(n),
                WasmVal::I64(n) => ffi::WasmEdge_ValueGenI64(n),
                WasmVal::F32(n) => ffi::WasmEdge_ValueGenF32(n),
                WasmVal::F64(n) => ffi::WasmEdge_ValueGenF64(n),
                WasmVal::V128(n) => ffi::WasmEdge_ValueGenV128(n),
                WasmVal::FuncRef(r) => {
                    // leak
                    let new_ctx = std::mem::ManuallyDrop::new(r.inner.clone());
                    ffi::WasmEdge_ValueGenFuncRef(new_ctx.0)
                }
                WasmVal::ExternRef(r) => ffi::WasmEdge_ValueGenExternRef(r.ctx),
                WasmVal::None => ffi::WasmEdge_ValueGenNullRef(ValType::None.into()),
            }
        }
    }
}

impl Into<ffi::WasmEdge_Value> for &WasmVal {
    fn into(self) -> ffi::WasmEdge_Value {
        unsafe {
            match self {
                WasmVal::I32(n) => ffi::WasmEdge_ValueGenI32(*n),
                WasmVal::I64(n) => ffi::WasmEdge_ValueGenI64(*n),
                WasmVal::F32(n) => ffi::WasmEdge_ValueGenF32(*n),
                WasmVal::F64(n) => ffi::WasmEdge_ValueGenF64(*n),
                WasmVal::V128(n) => ffi::WasmEdge_ValueGenV128(*n),
                WasmVal::FuncRef(r) => {
                    // leak
                    let new_ctx = std::mem::ManuallyDrop::new(r.inner.clone());
                    ffi::WasmEdge_ValueGenFuncRef(new_ctx.0)
                }
                WasmVal::ExternRef(r) => ffi::WasmEdge_ValueGenExternRef(r.ctx),
                WasmVal::None => ffi::WasmEdge_ValueGenNullRef(ValType::None.into()),
            }
        }
    }
}
