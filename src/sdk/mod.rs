#[cfg(feature = "aot")]
mod aot;

mod instance;
mod wasi;

pub use instance::module;
pub use instance::store;

pub use aot::{AotCompiler, AotConfig, CompilerOptimizationLevel};
