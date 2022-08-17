use std::path::Path;

use wasmedge_asyncify::*;

#[tokio::main]
async fn main() {
    let config = Config::create().unwrap();

    let mut builder = AsyncLinkerBuilder::new(&Some(config)).unwrap();

    // create a wasi module
    builder.create_wasi(&[], &["b=1", "a=1"], &[]).unwrap();

    // read wasm
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    // poll_tcp_listener.wasm from https://github.com/second-state/wasmedge_wasi_socket/blob/main/examples/poll_tcp_listener.rs
    let wasm_path = Path::new(&manifest_dir).join("../../wasm/poll_tcp_listener.wasm");
    println!("load wasm from {:?}", wasm_path);
    let wasm = std::fs::read(wasm_path).unwrap();

    // load wasm from bytes
    let module = builder.load_wasm(&wasm).unwrap();

    // instance wasm
    let mut inst = builder.instance(&module).unwrap();

    // call _start function
    inst.call("_start", vec![]).await.unwrap();
}
