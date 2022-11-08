#[cfg(feature = "aot")]
mod aot;

pub mod instance;
pub mod wasi;

pub use instance::module;
pub use instance::store;

pub use aot::{AotCompiler, AotConfig, CompilerOptimizationLevel};
