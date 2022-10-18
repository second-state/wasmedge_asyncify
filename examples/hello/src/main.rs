use std::path::Path;

use wasmedge_asyncify::{
    module::{AsyncInstance, AsyncInstanceRef, ResultFuture},
    store::Store,
    types, Config, Executor, ImportModule, Loader, Memory,
};

fn async_sleep<'a>(
    _inst: &'a mut AsyncInstanceRef,
    _mem: &'a mut Memory,
    data: &'a mut i32,
    _args: Vec<types::WasmVal>,
) -> ResultFuture<'a> {
    Box::new(async {
        let data = *data;
        println!("host: sleep {}s ...", data);
        tokio::time::sleep(std::time::Duration::from_secs(data as u64)).await;
        println!("host: sleep awake");
        Ok(vec![])
    })
}

#[tokio::main]
async fn main() {
    let config = Some(Config::create().unwrap());
    let executor = Executor::create(&config).unwrap();
    let mut store = Store::create().unwrap();
    let mut wasi_import = wasmedge_asyncify::wasi::AsyncWasiImport::new().unwrap();
    wasi_import.push_env("a", "1");
    wasi_import.push_env("b", "2");

    let mut host_import = ImportModule::create("host", 1).unwrap();
    host_import
        .add_async_func("async_sleep", (vec![], vec![]), async_sleep)
        .unwrap();
    store
        .register_import_object(&executor, &mut wasi_import)
        .unwrap();
    store
        .register_import_object(&executor, &mut host_import)
        .unwrap();

    let loader = Loader::create(&config).unwrap();

    // create a wasi module

    // create a async import module

    // read wasm
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let wasm_path = Path::new(&manifest_dir).join("../../wasm/hello.wasm");
    println!("load wasm from {:?}", wasm_path);
    let wasm = std::fs::read(wasm_path).unwrap();

    // load wasm from bytes
    let module = loader.load_async_module_from_bytes(&wasm).unwrap();

    // instance wasm
    let mut inst = AsyncInstance::instance(executor, &mut store, &module).unwrap();

    // call _start function
    println!("call _start");
    inst.call("_start", vec![]).unwrap().await.unwrap();
}
