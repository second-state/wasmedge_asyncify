use std::path::Path;

use wasmedge_asyncify::{types::WasmVal, *};

fn async_host_to_uppercase(linker: &mut AsyncLinker, args: Vec<types::WasmVal>) -> ResultFuture {
    Box::new(async move {
        if let (Some(WasmVal::I32(offset)), Some(WasmVal::I32(len))) = (args.get(0), args.get(1)) {
            let bytes = linker.get_memory_mut("memory", *offset as usize, *len as usize)?;
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
            b.add_async_func(
                "to_uppercase",
                (vec![ValType::I32, ValType::I32], vec![ValType::I32]),
                async_host_to_uppercase,
            )?;
            Ok(())
        })
        .unwrap();

    // read wasm
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let wasm_path = Path::new(&manifest_dir).join("../../wasm/memory.wasm");
    println!("load wasm from {:?}", wasm_path);
    let wasm = std::fs::read(wasm_path).unwrap();

    // load wasm from bytes
    let module = builder.load_wasm(&wasm).unwrap();

    // instance wasm
    let mut inst = builder.instance(&module).unwrap();

    // call _start function
    inst.call("_start", vec![]).await.unwrap();
}
