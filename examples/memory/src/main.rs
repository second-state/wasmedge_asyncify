use std::path::Path;

use wasmedge_asyncify::{
    error::CoreError,
    module::{AsyncInstance, AsyncInstanceRef},
    store::Store,
    types::{ValType, WasmVal},
    *,
};

fn sync_sleep<'a>(
    _inst: &'a mut AsyncInstanceRef,
    mem: &'a mut Memory,
    _data: &'a mut i32,
    args: Vec<types::WasmVal>,
) -> Result<Vec<WasmVal>, CoreError> {
    if let (Some(WasmVal::I32(offset)), Some(WasmVal::I32(len))) = (args.get(0), args.get(1)) {
        let bytes = mem.data_pointer_mut(*offset as usize, *len as usize);
        if bytes.is_none() {
            return Ok(vec![WasmVal::I32(-1)]);
        }
        let bytes = bytes.unwrap();
        if let Ok(s) = std::str::from_utf8_mut(bytes) {
            let new_s = s.to_uppercase();
            bytes.clone_from_slice(new_s.as_bytes());
            Ok(vec![WasmVal::I32(*len)])
        } else {
            Ok(vec![WasmVal::I32(-1)])
        }
    } else {
        Ok(vec![WasmVal::I32(-1)])
    }
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
        .add_sync_func(
            "to_uppercase",
            (vec![ValType::I32, ValType::I32], vec![ValType::I32]),
            sync_sleep,
        )
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
    let wasm_path = Path::new(&manifest_dir).join("../../wasm/memory.wasm");
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
