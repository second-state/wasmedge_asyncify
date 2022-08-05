/// unsafe module
mod core;
mod sdk;
mod utils;

#[cfg(feature = "ffi")]
pub use wasmedge_sys::ffi;

pub use crate::core::config::Config;
pub use crate::core::types;
pub use sdk::*;
pub use wasmedge_types::error;
pub use wasmedge_types::ValType;
pub use wasmedge_types::WasmEdgeResult;

#[cfg(test)]
mod tests {
    use crate::*;
    use tokio;

    #[tokio::test]
    async fn it_works() {
        let mut config = crate::Config::create().unwrap();
        config.wasi(true);
        config.multi_memories(true);

        let mut builder = crate::AsyncLinkerBuilder::new(&Some(config)).unwrap();

        fn host_sleep(_linker: &mut AsyncLinker, _args: Vec<types::WasmVal>) -> ResultFuture {
            Box::new(async {
                println!("host sleep 1s ...");
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                println!("host sleep awake");

                Ok(vec![])
            })
        }

        builder.create_wasi(&[], &["a=1"], &[]).unwrap();
        builder
            .create_import_object("host", |b| {
                b.add_async_func("sleep", (vec![], vec![]), host_sleep, 0)?;
                Ok(())
            })
            .unwrap();

        let wasm = std::fs::read("wasm/hello.wasm").unwrap();

        let module = builder.load_wasm(&wasm).unwrap();

        let mut inst = builder.instance(&module).unwrap();

        inst.call("_start", vec![]).await.unwrap();
    }
}
