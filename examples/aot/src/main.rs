use std::path::Path;

use wasmedge_asyncify::{
    ast_module::pass_async_module,
    module::{AsyncInstance, AsyncInstanceRef, ResultFuture},
    store::Store,
    types, Config, Executor, ImportModule, Loader, Memory,
};

fn load_wasm_bytes(path: &str) -> Vec<u8> {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let wasm_path = Path::new(&manifest_dir).join(&format!("../../{}", path));
    println!("load wasm from {:?}", wasm_path);
    let wasm = std::fs::read(wasm_path).unwrap();
    wasm
}

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
    simple_log::quick!("trace");
    let config = Some(Config::create().unwrap());
    let executor = Executor::create(&config).unwrap();
    let mut store = Store::create().unwrap();

    // create a wasi module
    let mut wasi_import = wasmedge_asyncify::wasi::AsyncWasiImport::new().unwrap();
    wasi_import.push_env("a", "1");
    wasi_import.push_env("b", "2");

    // create a async import module
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

    // read wasm
    let wasm = load_wasm_bytes("wasm/hello.wasm");
    let wasm = pass_async_module(&wasm).unwrap();

    let aot_config = wasmedge_asyncify::AotConfig::create().unwrap();
    let mut compiler = wasmedge_asyncify::AotCompiler::create(&aot_config).unwrap();
    compiler.compile(&wasm, "hello.aot.wasm").unwrap();

    let aot_wasm = std::fs::read("hello.aot.wasm").unwrap();

    // load wasm from bytes

    // use wasm ok
    let module = loader.load_async_module_from_bytes(&wasm).unwrap();
    // use aot_wasm fail
    // let module = loader.load_async_module_from_bytes(&aot_wasm).unwrap();

    // instance wasm
    let mut inst = AsyncInstance::instance(executor, &mut store, &module).unwrap();

    // call _start function
    println!("call _start");
    inst.call("_start", vec![]).await.unwrap();
}
