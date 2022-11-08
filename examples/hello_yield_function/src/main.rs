use log::info;
use serde::{Deserialize, Serialize};
use std::path::Path;
use wasmedge_asyncify::{
    ast_module::pass_async_module,
    error::CoreError,
    module::{AsyncInstance, AsyncInstanceRef, InstanceSnapshot, ResultFuture},
    store::Store,
    types, Config, Executor, ImportModule, Loader, Memory,
};

fn load_wasm_bytes(path: &str) -> Vec<u8> {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let wasm_path = Path::new(&manifest_dir).join(&format!("../../{}", path));
    info!("load wasm from {:?}", wasm_path);
    let wasm = std::fs::read(wasm_path).unwrap();
    wasm
}

// not must impl serde, just mean it is serializable
#[derive(Debug, Clone, Deserialize, Serialize)]
struct ResumeAbleData {
    pub times: usize,
    pub is_resume: bool,
}

// when times is `2`, it will yield to host
fn async_counting<'a>(
    _inst: &'a mut AsyncInstanceRef,
    _mem: &'a mut Memory,
    data: &'a mut ResumeAbleData,
    _args: Vec<types::WasmVal>,
) -> ResultFuture<'a> {
    Box::new(async {
        if !data.is_resume {
            info!("ResumeAbleData.times = {}", data.times);
            if data.times == 2 {
                data.is_resume = true;
                return Err(CoreError::Yield);
            }
            data.times += 1;
        } else {
            data.is_resume = false;
            // continue from last yield (line 39)
            data.times += 1;
        }
        Ok(vec![])
    })
}

async fn run_until_yield(async_wasm: &[u8]) -> (InstanceSnapshot, Box<ResumeAbleData>) {
    let config = Some(Config::create().unwrap());
    let executor = Executor::create(&config).unwrap();

    // create a wasi module
    let mut wasi_import = wasmedge_asyncify::wasi::AsyncWasiImport::new().unwrap();
    wasi_import.push_env("RUST_LOG", "info");

    // create a async import module
    let data = ResumeAbleData {
        times: 0,
        is_resume: false,
    };
    let mut host_import = ImportModule::create("host", data).unwrap();
    host_import
        .add_async_func("async_counting", (vec![], vec![]), async_counting)
        .unwrap();

    let mut store = Store::create().unwrap();
    store
        .register_import_object(&executor, &mut wasi_import)
        .unwrap();
    store
        .register_import_object(&executor, &mut host_import)
        .unwrap();

    let loader = Loader::create(&config).unwrap();
    let module = loader.load_async_module_from_bytes(async_wasm).unwrap();

    // instance wasm
    let mut inst = AsyncInstance::instance(executor, &mut store, &module).unwrap();

    // call _start function
    info!("first run inst");
    let r = inst.call("_start", vec![]).await;

    assert!(r.unwrap_err().is_yield());
    info!("yield from wasm");

    (inst.snapshot(), host_import.data)
}

async fn resume_wasm(async_wasm: &[u8], snapshot: InstanceSnapshot, data: ResumeAbleData) {
    let config = Some(Config::create().unwrap());
    let executor = Executor::create(&config).unwrap();

    // create a wasi module
    let mut wasi_import = wasmedge_asyncify::wasi::AsyncWasiImport::new().unwrap();
    wasi_import.push_env("RUST_LOG", "info");

    // resume custom import module
    let mut host_import = ImportModule::create("host", data).unwrap();
    host_import
        .add_async_func("async_counting", (vec![], vec![]), async_counting)
        .unwrap();

    let mut store = Store::create().unwrap();
    store
        .register_import_object(&executor, &mut wasi_import)
        .unwrap();
    store
        .register_import_object(&executor, &mut host_import)
        .unwrap();

    let loader = Loader::create(&config).unwrap();
    let module = loader.load_async_module_from_bytes(async_wasm).unwrap();

    // instance wasm
    let mut inst = AsyncInstance::instance(executor, &mut store, &module).unwrap();
    info!("resume inst state");
    inst.apply_snapshot(snapshot).unwrap();

    // call _start function
    info!("second inst");
    inst.call("_start", vec![]).await.unwrap();
}

#[tokio::main]
async fn main() {
    simple_log::quick!("info");
    // read wasm
    let wasm = load_wasm_bytes("wasm/yield_count.wasm");

    // pass async module
    let async_wasm = pass_async_module(&wasm).unwrap();

    let (snapshot, data) = run_until_yield(&async_wasm).await;

    // we can save snapshot & data into some file or database,
    // and load then in other process
    info!("save snapshot & data ...");
    // save snapshot
    let data_str = serde_json::to_string(&data).unwrap();

    info!("load...");
    let data = serde_json::from_str(&data_str).unwrap();
    resume_wasm(&async_wasm, snapshot, data).await;
}
