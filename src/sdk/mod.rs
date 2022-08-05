use crate::core::types::WasmVal;

mod instance;
mod linker;
mod module;

pub use crate::core::instance::memory::Memory;
pub use instance::function::ResultFuture;

pub type AsyncFn = for<'a> fn(&'a mut linker::AsyncLinker, Vec<WasmVal>) -> ResultFuture<'a>;
pub use instance::function::WasmEdgeResultFuture;
pub use linker::{AsLinker, AsyncLinker, AsyncLinkerBuilder};
pub use module::AsyncImportModuleBuilder;
