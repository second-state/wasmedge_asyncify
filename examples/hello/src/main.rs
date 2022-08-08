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
    let wasm = std::fs::read("wasm/hello.wasm").unwrap();

    // load wasm
    let module = builder.load_wasm(&wasm).unwrap();

    // instance wasm
    let mut inst = builder.instance(&module).unwrap();

    // call _start function
    inst.call("_start", vec![]).await.unwrap();
}
