/// unsafe module
mod core;
mod sdk;
mod utils;

#[cfg(feature = "ffi")]
pub use wasmedge_sys::ffi;

pub use crate::core::config::Config;
pub use crate::core::types;
pub use sdk::*;
pub use wasmedge_types::error;
pub use wasmedge_types::ValType;
pub use wasmedge_types::WasmEdgeResult;
