// unsafe module
mod core;
pub mod error;
mod sdk;
mod utils;

pub use crate::core::ast_module;
pub use crate::core::config::Config;
pub use crate::core::executor::Executor;
pub use crate::core::instance::memory::Memory;
pub use crate::core::types;
pub use crate::core::AstModule;
pub use crate::core::ImportModule;
pub use crate::core::Loader;
pub use sdk::*;

#[tokio::test]
async fn run_test() {
    let code = r#"
    (module
        (import "env1" "async_f1" (func $f1 (result i32)))
        (import "env2" "async_f2" (func $f2 (result i32)))
        (memory 1 1)
        (func $test (result i32)
          (i32.add (call $f1) (call $f2)))
        (export "test" (func $test))
    )
    "#;

    let wasm = wat::parse_str(code).unwrap();
    let config = Config::create();
    let loader = Loader::create(&config).unwrap();
    let module = loader.load_async_module_from_bytes(&wasm).unwrap();

    fn f1<'a>(
        _inst: &'a mut module::AsyncInstanceRef,
        _mem: &'a mut Memory,
        data: &'a mut i32,
        _args: Vec<types::WasmVal>,
    ) -> module::ResultFuture<'a> {
        Box::new(async {
            let i = std::time::Instant::now();
            let delay = std::time::Duration::from_secs(1);
            tokio::time::sleep(delay).await;
            assert!(i.elapsed() >= delay);
            Ok(vec![types::WasmVal::I32(*data)])
        })
    }
    let mut env_import = ImportModule::create("env1", 2).unwrap();
    env_import
        .add_async_func("async_f1", (vec![], vec![types::ValType::I32]), f1)
        .unwrap();

    fn f2<'a>(
        _inst: &'a mut module::AsyncInstanceRef,
        _mem: &'a mut Memory,
        _data: &'a mut (),
        _args: Vec<types::WasmVal>,
    ) -> module::ResultFuture<'a> {
        Box::new(async { Ok(vec![types::WasmVal::I32(1)]) })
    }
    let mut env_import2 = ImportModule::create("env2", ()).unwrap();
    env_import2
        .add_async_func("async_f2", (vec![], vec![types::ValType::I32]), f2)
        .unwrap();

    let executor = Executor::create(&config).unwrap();

    let mut store = store::Store::create().unwrap();
    store
        .register_import_object(&executor, &mut env_import)
        .unwrap();
    store
        .register_import_object(&executor, &mut env_import2)
        .unwrap();

    let mut inst = module::AsyncInstance::instance(executor, store, &module).unwrap();

    let r = inst.call("test", vec![]).await.unwrap();
    assert_eq!(r.first().cloned(), Some(types::WasmVal::I32(3)));
}
