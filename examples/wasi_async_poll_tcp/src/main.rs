use std::path::Path;

use wasmedge_asyncify::{module::AsyncInstance, store::Store, *};

fn load_wasm_bytes(path: &str) -> Vec<u8> {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let wasm_path = Path::new(&manifest_dir).join(&format!("../../{}", path));
    println!("load wasm from {:?}", wasm_path);
    let wasm = std::fs::read(wasm_path).unwrap();
    wasm
}

#[tokio::main]
async fn main() {
    const WASM_PATH: &str = "wasm/poll_tcp_listener.wasm";

    let config = Some(Config::create().unwrap());
    let executor = Executor::create(&config).unwrap();

    // create a wasi module
    let mut wasi_import = wasmedge_asyncify::wasi::AsyncWasiImport::new().unwrap();
    wasi_import.push_env("a", "1");
    wasi_import.push_env("b", "2");

    let mut store = Store::create().unwrap();
    store
        .register_import_object(&executor, &mut wasi_import)
        .unwrap();

    let loader = Loader::create(&config).unwrap();

    // read wasm
    let wasm = load_wasm_bytes(WASM_PATH);

    // load a async wasm from bytes
    let module = loader.load_async_module_from_bytes(&wasm).unwrap();

    // instance wasm
    let mut inst = AsyncInstance::instance(executor, &mut store, &module).unwrap();

    // call _start function
    println!("call _start");
    inst.call("_start", vec![]).await.unwrap();
}
