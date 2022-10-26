use std::future::Future;
use std::path::Path;

use wasmedge_asyncify::{module::AsyncInstance, store::Store, *};

fn main() {
    simple_log::quick!("trace");
    single_thread_run(run_wasi_test("list_cwd"));
}

#[allow(unused)]
fn multi_thread_run<F: Future>(f: F) -> F::Output {
    let multi_thread_runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .worker_threads(4)
        .build()
        .unwrap();
    multi_thread_runtime.block_on(f)
}

#[allow(unused)]
fn single_thread_run<F: Future>(f: F) -> F::Output {
    let single_thread_runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    single_thread_runtime.block_on(async {
        let tick_loop = tokio::spawn(async {
            loop {
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                log::info!("tick");
            }
        });
        f.await
    })
}

async fn run_wasi_test(name: &str) {
    log::info!("run {name}");
    let config = Some(Config::create().unwrap());
    let executor = Executor::create(&config).unwrap();
    let mut store = Store::create().unwrap();
    let mut wasi_import = wasmedge_asyncify::wasi::AsyncWasiImport::new().unwrap();
    wasi_import.push_arg(name.to_string());
    wasi_import.push_env("RUST_LOG", "info");
    wasi_import
        .push_preopen(".".parse().unwrap(), ".".parse().unwrap())
        .unwrap();

    store
        .register_import_object(&executor, &mut wasi_import)
        .unwrap();

    let loader = Loader::create(&config).unwrap();

    // create a wasi module

    // create a async import module

    // read wasm
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let wasm_path = Path::new(&manifest_dir).join("../../wasm/wasi_impl_test.wasm");
    log::info!("load wasm from {:?}", wasm_path);
    let wasm = std::fs::read(wasm_path).unwrap();

    // load wasm from bytes
    log::info!("pass bytes");
    let module = loader.load_async_module_from_bytes(&wasm).unwrap();

    // instance wasm
    log::info!("instance");
    let mut inst = AsyncInstance::instance(executor, &mut store, &module).unwrap();

    // call _start function
    log::info!("call _start");
    inst.call("_start", vec![]).unwrap().await.unwrap();
}
