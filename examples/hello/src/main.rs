use std::path::Path;

use wasmedge_asyncify::*;

fn async_host_sleep(_linker: &mut AsyncLinker, _args: Vec<types::WasmVal>) -> ResultFuture {
    Box::new(async {
        println!("host: sleep 1s ...");
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        println!("host: sleep awake");

        Ok(vec![])
    })
}

#[tokio::main]
async fn main() {
    let config = crate::Config::create().unwrap();

    let mut builder = crate::AsyncLinkerBuilder::new(&Some(config)).unwrap();

    // create a wasi module
    builder.create_wasi(&[], &["b=1", "a=1"], &[]).unwrap();

    // create a async import module
    builder
        .create_import_object("host", |b| {
            b.add_async_func("sleep", (vec![], vec![]), async_host_sleep)?;
            Ok(())
        })
        .unwrap();

    // read wasm
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let wasm_path = Path::new(&manifest_dir).join("../../wasm/hello.wasm");
    println!("load wasm from {:?}", wasm_path);
    let wasm = std::fs::read(wasm_path).unwrap();

    // load wasm from bytes
    let module = builder.load_wasm(&wasm).unwrap();

    // instance wasm
    let mut inst = builder.instance(&module).unwrap();

    // call _start function
    inst.call("_start", vec![]).await.unwrap();
}
