use std::{ffi::CString, fmt::Debug};

#[cfg(feature = "wasm_ref")]
use super::instance::function::{FuncRef, InnerFunc};

use wasmedge_sys_ffi as ffi;

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
    pub fn new(s: &str) -> Result<Self, std::ffi::NulError> {
        let cs = CString::new(s)?;
        let ctx = unsafe { ffi::WasmEdge_StringCreateByCString(cs.as_ptr()) };

        Ok(Self {
            inner: InnerWasmEdgeString(ctx),
        })
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
    #[cfg(feature = "wasm_ref")]
    FuncRef(FuncRef),
    #[cfg(feature = "wasm_ref")]
    ExternRef(Extern),
    None,
}

impl PartialEq for WasmVal {
    fn eq(&self, other: &Self) -> bool {
        use WasmVal::*;
        match (self, other) {
            (I32(i), I32(other)) => *i == *other,
            (I64(i), I64(other)) => *i == *other,
            (F32(i), F32(other)) => *i == *other,
            (F64(i), F64(other)) => *i == *other,
            (V128(i), V128(other)) => *i == *other,
            #[cfg(feature = "wasm_ref")]
            (FuncRef(i), FuncRef(other)) => i.inner.0 == other.inner.0,
            #[cfg(feature = "wasm_ref")]
            (ExternRef(i), ExternRef(other)) => i.ctx == other.ctx,
            (None, None) => true,
            _ => false,
        }
    }
}
impl Eq for WasmVal {}

impl From<ffi::WasmEdge_Value> for WasmVal {
    fn from(raw_val: ffi::WasmEdge_Value) -> Self {
        unsafe {
            match raw_val.Type {
                ffi::WasmEdge_ValType_I32 => WasmVal::I32(ffi::WasmEdge_ValueGetI32(raw_val)),
                ffi::WasmEdge_ValType_I64 => WasmVal::I64(ffi::WasmEdge_ValueGetI64(raw_val)),
                ffi::WasmEdge_ValType_F32 => WasmVal::F32(ffi::WasmEdge_ValueGetF32(raw_val)),
                ffi::WasmEdge_ValType_F64 => WasmVal::F64(ffi::WasmEdge_ValueGetF64(raw_val)),
                ffi::WasmEdge_ValType_V128 => WasmVal::V128(ffi::WasmEdge_ValueGetV128(raw_val)),
                #[cfg(feature = "wasm_ref")]
                ffi::WasmEdge_ValType_FuncRef => {
                    let func_ref = ffi::WasmEdge_ValueGetFuncRef(raw_val);
                    WasmVal::FuncRef(FuncRef {
                        inner: InnerFunc(func_ref),
                    })
                }
                #[cfg(feature = "wasm_ref")]
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
                #[cfg(feature = "wasm_ref")]
                WasmVal::FuncRef(r) => {
                    // leak
                    let new_ctx = std::mem::ManuallyDrop::new(r.inner.clone());
                    ffi::WasmEdge_ValueGenFuncRef(new_ctx.0)
                }
                #[cfg(feature = "wasm_ref")]
                WasmVal::ExternRef(r) => ffi::WasmEdge_ValueGenExternRef(r.ctx),
                WasmVal::None => ffi::WasmEdge_ValueGenNullRef(ffi::WasmEdge_ValType_None),
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
                #[cfg(feature = "wasm_ref")]
                WasmVal::FuncRef(r) => {
                    // leak
                    let new_ctx = std::mem::ManuallyDrop::new(r.inner.clone());
                    ffi::WasmEdge_ValueGenFuncRef(new_ctx.0)
                }
                #[cfg(feature = "wasm_ref")]
                WasmVal::ExternRef(r) => ffi::WasmEdge_ValueGenExternRef(r.ctx),
                WasmVal::None => ffi::WasmEdge_ValueGenNullRef(ffi::WasmEdge_ValType_None),
            }
        }
    }
}

#[derive(Copy, Clone, Eq, PartialEq)]
pub struct ValType(pub(crate) ffi::WasmEdge_NumType);

impl ValType {
    pub const I32: ValType = ValType(ffi::WasmEdge_NumType_I32);
    pub const I64: ValType = ValType(ffi::WasmEdge_NumType_I64);
    pub const F32: ValType = ValType(ffi::WasmEdge_NumType_F32);
    pub const F64: ValType = ValType(ffi::WasmEdge_NumType_F64);
    pub const V128: ValType = ValType(ffi::WasmEdge_NumType_V128);
    #[cfg(feature = "wasm_ref")]
    pub const FUNC_REF: ValType = ValType(ffi::WasmEdge_RefType_FuncRef);
    #[cfg(feature = "wasm_ref")]
    pub const EXTERN_REF: ValType = ValType(ffi::WasmEdge_RefType_ExternRef);
    pub const NONE: ValType = ValType(ffi::WasmEdge_ValType_None);
}

impl Debug for ValType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            Self::I32 => write!(f, "ValType::I32"),
            Self::I64 => write!(f, "ValType::I64"),
            Self::F32 => write!(f, "ValType::F32"),
            Self::F64 => write!(f, "ValType::F64"),
            Self::V128 => write!(f, "ValType::V128"),
            #[cfg(feature = "wasm_ref")]
            Self::FUNC_REF => write!(f, "ValType::FuncRef"),
            #[cfg(feature = "wasm_ref")]
            Self::EXTERN_REF => write!(f, "ValType::ExternRef"),
            Self::NONE => write!(f, "ValType::None"),
            _ => {
                write!(f, "ValType::Unknown")
            }
        }
    }
}

impl From<u32> for ValType {
    fn from(value: u32) -> Self {
        match value {
            ffi::WasmEdge_NumType_I32 => ValType::I32,
            ffi::WasmEdge_NumType_I64 => ValType::I64,
            ffi::WasmEdge_NumType_F32 => ValType::F32,
            ffi::WasmEdge_NumType_F64 => ValType::F64,
            ffi::WasmEdge_NumType_V128 => ValType::V128,
            #[cfg(feature = "wasm_ref")]
            ffi::WasmEdge_RefType_FuncRef => ValType::FUNC_REF,
            #[cfg(feature = "wasm_ref")]
            ffi::WasmEdge_RefType_ExternRef => ValType::EXTERN_REF,
            ffi::WasmEdge_ValType_None => ValType::NONE,
            _ => panic!("[wasmedge-types] Invalid WasmEdge_ValType: {:#X}", value),
        }
    }
}
impl From<ValType> for u32 {
    fn from(value: ValType) -> Self {
        value.0
    }
}
impl From<i32> for ValType {
    fn from(value: i32) -> Self {
        let value = value as u32;
        ValType::from(value)
    }
}
impl From<ValType> for i32 {
    fn from(value: ValType) -> Self {
        value.0 as i32
    }
}
