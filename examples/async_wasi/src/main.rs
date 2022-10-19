use std::path::Path;

use wasmedge_asyncify::{module::AsyncInstance, store::Store, *};

#[tokio::main]
async fn main() {
    let config = Some(Config::create().unwrap());
    let executor = Executor::create(&config).unwrap();
    let mut store = Store::create().unwrap();
    let mut wasi_import = wasmedge_asyncify::wasi::AsyncWasiImport::new().unwrap();
    wasi_import.push_env("a", "1");
    wasi_import.push_env("b", "2");

    store
        .register_import_object(&executor, &mut wasi_import)
        .unwrap();

    let loader = Loader::create(&config).unwrap();

    // create a wasi module

    // create a async import module

    // read wasm
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let wasm_path = Path::new(&manifest_dir).join("../../wasm/poll_tcp_listener.wasm");
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
