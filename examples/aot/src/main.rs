use std::path::{Path, PathBuf};

use wasmedge_asyncify::*;

fn async_host_sleep(_linker: &mut AsyncLinker, _args: Vec<types::WasmVal>) -> ResultFuture {
    Box::new(async {
        println!("host: sleep 1s ...");
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        println!("host: sleep awake");

        Ok(vec![])
    })
}

fn create_builder() -> AsyncLinkerBuilder {
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

    builder
}

fn read_wasm() -> Vec<u8> {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let wasm_path = Path::new(&manifest_dir).join("../../wasm/hello.wasm");
    println!("load wasm from {:?}", wasm_path);
    std::fs::read(wasm_path).unwrap()
}

fn aot_out_path() -> PathBuf {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let wasm_path = Path::new(&manifest_dir).join("hello.aot.wasm");
    wasm_path
}

fn aot_to_file() {
    let wasm = read_wasm();
    let mut builder = create_builder();
    let config = AotConfig::create().unwrap();

    let mut compiler = AotCompiler::create(&config).unwrap();

    let out_path = aot_out_path();

    if let Some(e) = compiler
        .compile_async_module(&mut builder, &wasm, out_path.as_path())
        .unwrap()
    {
        let e: Result<(), std::io::Error> = Err(e);
        e.unwrap()
    }
    println!("compile wasm to {:?}", out_path);
}

#[tokio::main]
async fn main() {
    aot_to_file();

    let aot_wasm = std::fs::read(aot_out_path()).unwrap();

    let mut builder = create_builder();
    // load wasm from bytes
    let module = builder.load_wasm(&aot_wasm).unwrap();

    // instance wasm
    let mut inst = builder.instance(&module).unwrap();

    // call _start function
    inst.call("_start", vec![]).await.unwrap();
}
