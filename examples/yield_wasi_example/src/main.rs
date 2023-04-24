use std::path::Path;
use std::{future::Future, path::PathBuf};

use wasmedge_asyncify::{
    module::{AsyncInstance, InstanceSnapshot},
    store::Store,
    wasi::serialize,
    *,
};

fn main() {
    simple_log::quick!("info");
    single_thread_run(tcp_listener::run_serial_test());
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

pub mod change_preopen {
    use super::*;

    async fn first_run<'a>(
        config: &Option<Config>,
        asyncify_wasm_bytes: &[u8],
        name: &str,
    ) -> (InstanceSnapshot, serialize::SerialWasiCtx) {
        use wasi::serialize::IoState;

        let executor = Executor::create(&config).unwrap();
        let mut store = Store::create().unwrap();
        let mut wasi_import = wasmedge_asyncify::wasi::AsyncWasiImport::new().unwrap();
        wasi_import.push_arg(name.to_string());
        wasi_import.push_env("RUST_LOG", "info");

        fn hook(io_state: &IoState) -> bool {
            log::info!("hook! {:?}", io_state);
            match io_state {
                IoState::Sleep { .. } => true,
                _ => false,
            }
        }
        wasi_import.data.yield_hook = Some((std::time::Duration::from_secs(1), hook));

        wasi_import
            .push_preopen(".".parse().unwrap(), ".".parse().unwrap())
            .unwrap();

        store
            .register_import_object(&executor, &mut wasi_import)
            .unwrap();

        let loader = Loader::create(&config).unwrap();
        let ast_module = loader
            .load_async_module_from_bytes(asyncify_wasm_bytes)
            .unwrap();
        // instance wasm
        log::info!("instance");
        let mut inst = AsyncInstance::instance(executor, store, &ast_module).unwrap();

        // call _start function
        log::info!("first_run call _start");
        let r = inst.call("_start", vec![]).await;
        if let Err(e) = &r {
            if e.is_yield() {
                log::info!("yield!");
            } else {
                Err(e.clone()).unwrap()
            }
        }
        let snapshot = inst.snapshot();
        let wasi_snapshot: wasi::serialize::SerialWasiCtx = (&wasi_import.data.wasi_ctx).into();

        (snapshot, wasi_snapshot)
    }

    async fn resume_and_run(
        config: &Option<Config>,
        asyncify_wasm_bytes: &[u8],
        name: &str,
        snapshot: (InstanceSnapshot, serialize::SerialWasiCtx),
    ) {
        use wasi::serialize::IoState;

        let (snapshot, wasi_ctx_snapshot) = snapshot;

        let executor = Executor::create(&config).unwrap();
        let mut store = Store::create().unwrap();

        let wasi_ctx = wasi_ctx_snapshot.resume(|s_fd| match s_fd {
            serialize::SerialVFD::Stdin(s) => s.clone().into(),
            serialize::SerialVFD::Stdout(s) => s.clone().into(),
            serialize::SerialVFD::Stderr(s) => s.clone().into(),

            serialize::SerialVFD::PreOpenDir(dir) => match dir.guest_path.as_str() {
                "." => dir.clone().to_vfd(PathBuf::from("./wasm")),
                _ => wasi::snapshots::env::VFD::Closed,
            },
            serialize::SerialVFD::TcpServer(s) => s.clone().default_to_async_socket().unwrap(),
            serialize::SerialVFD::UdpSocket(s) => s.clone().default_to_async_socket().unwrap(),
            _ => wasi::snapshots::env::VFD::Closed,
        });
        let mut wasi_import =
            wasmedge_asyncify::wasi::AsyncWasiImport::with_wasi_ctx(wasi_ctx).unwrap();
        wasi_import.push_arg(name.to_string());
        wasi_import.push_env("RUST_LOG", "info");

        fn hook(io_state: &IoState) -> bool {
            log::info!("hook! {:?}", io_state);
            false
        }
        wasi_import.data.yield_hook = Some((std::time::Duration::from_secs(3), hook));

        store
            .register_import_object(&executor, &mut wasi_import)
            .unwrap();

        let loader = Loader::create(&config).unwrap();
        let ast_module = loader
            .load_async_module_from_bytes(asyncify_wasm_bytes)
            .unwrap();
        // instance wasm
        log::info!("instance");
        let mut inst = AsyncInstance::instance(executor, store, &ast_module).unwrap();

        log::info!("resume from snapshot");
        inst.apply_snapshot(snapshot).unwrap();

        // call _start function
        log::info!("resume call _start");
        let r = inst.call("_start", vec![]).await.unwrap();
        log::info!("_start return {:?}", r);
    }

    #[allow(unused)]
    pub async fn run_serial_test() {
        let name = "list_cwd";
        log::info!("run {name}");
        let config = Some(Config::create().unwrap());
        // read wasm
        let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
        let wasm_path = Path::new(&manifest_dir).join("../../wasm/yield_wasm_test.wasm");
        log::info!("load wasm from {:?}", wasm_path);
        let wasm = std::fs::read(wasm_path).unwrap();

        // load wasm from bytes
        log::info!("pass bytes");
        let asyncify_wasm_bytes = ast_module::pass_async_module(&wasm).unwrap();

        let snapshot = first_run(&config, &asyncify_wasm_bytes, name).await;
        resume_and_run(&config, &asyncify_wasm_bytes, name, snapshot).await;
    }
}

pub mod tcp_listener {
    use super::*;

    async fn first_run<'a>(
        config: &Option<Config>,
        asyncify_wasm_bytes: &[u8],
        name: &str,
    ) -> (InstanceSnapshot, serialize::SerialWasiCtx) {
        use wasi::serialize::IoState;

        let executor = Executor::create(&config).unwrap();
        let mut store = Store::create().unwrap();
        let mut wasi_import = wasmedge_asyncify::wasi::AsyncWasiImport::new().unwrap();
        wasi_import.push_arg(name.to_string());
        wasi_import.push_env("RUST_LOG", "info");

        fn hook(io_state: &IoState) -> bool {
            log::info!("hook! {:?}", io_state);
            match io_state {
                IoState::Sleep { .. } => true,
                IoState::Accept { .. } => true,
                _ => false,
            }
        }
        wasi_import.data.yield_hook = Some((std::time::Duration::from_secs(1), hook));

        wasi_import
            .push_preopen(".".parse().unwrap(), ".".parse().unwrap())
            .unwrap();

        store
            .register_import_object(&executor, &mut wasi_import)
            .unwrap();

        let loader = Loader::create(&config).unwrap();
        let ast_module = loader
            .load_async_module_from_bytes(asyncify_wasm_bytes)
            .unwrap();
        // instance wasm
        log::info!("instance");
        let mut inst = AsyncInstance::instance(executor, store, &ast_module).unwrap();

        // call _start function
        log::info!("first_run call _start");
        let r = inst.call("_start", vec![]).await;
        if let Err(e) = &r {
            if e.is_yield() {
                log::info!("yield!");
            } else {
                Err(e.clone()).unwrap()
            }
        }
        let snapshot = inst.snapshot();
        let wasi_snapshot: wasi::serialize::SerialWasiCtx = (&wasi_import.data.wasi_ctx).into();

        (snapshot, wasi_snapshot)
    }

    async fn resume_and_run(
        config: &Option<Config>,
        asyncify_wasm_bytes: &[u8],
        name: &str,
        snapshot: (InstanceSnapshot, serialize::SerialWasiCtx),
    ) {
        let (snapshot, wasi_ctx_snapshot) = snapshot;

        let executor = Executor::create(&config).unwrap();
        let mut store = Store::create().unwrap();

        let wasi_ctx = wasi_ctx_snapshot.resume(|s_fd| match s_fd {
            serialize::SerialVFD::Stdin(s) => s.clone().into(),
            serialize::SerialVFD::Stdout(s) => s.clone().into(),
            serialize::SerialVFD::Stderr(s) => s.clone().into(),

            serialize::SerialVFD::PreOpenDir(dir) => match dir.guest_path.as_str() {
                "." => dir.clone().to_vfd(PathBuf::from(".")),
                _ => wasi::snapshots::env::VFD::Closed,
            },
            serialize::SerialVFD::TcpServer(s) => {
                if let Some(addr) = &s.state.local_addr {
                    use std::net::TcpListener;
                    if addr.to_string().as_str() == "0.0.0.0:1234" {
                        let addr = "0.0.0.0:1235";
                        let listener = TcpListener::bind(&addr).unwrap();
                        log::info!("listen on 1235");
                        s.clone().to_async_socket_with_std(listener).unwrap()
                    } else {
                        s.clone().default_to_async_socket().unwrap()
                    }
                } else {
                    s.clone().default_to_async_socket().unwrap()
                }
            }
            serialize::SerialVFD::UdpSocket(s) => s.clone().default_to_async_socket().unwrap(),
            _ => wasi::snapshots::env::VFD::Closed,
        });
        let mut wasi_import =
            wasmedge_asyncify::wasi::AsyncWasiImport::with_wasi_ctx(wasi_ctx).unwrap();
        wasi_import.push_arg(name.to_string());
        wasi_import.push_env("RUST_LOG", "info");

        store
            .register_import_object(&executor, &mut wasi_import)
            .unwrap();

        let loader = Loader::create(&config).unwrap();
        let ast_module = loader
            .load_async_module_from_bytes(asyncify_wasm_bytes)
            .unwrap();
        // instance wasm
        log::info!("instance");
        let mut inst = AsyncInstance::instance(executor, store, &ast_module).unwrap();

        log::info!("resume from snapshot");
        inst.apply_snapshot(snapshot).unwrap();

        // call _start function
        log::info!("resume call _start");
        let r = inst.call("_start", vec![]).await.unwrap();
        log::info!("_start return {:?}", r);
    }

    #[allow(unused)]
    pub async fn run_serial_test() {
        let name = "block_socket";
        log::info!("run {name}");
        let config = Some(Config::create().unwrap());
        // read wasm
        let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
        let wasm_path = Path::new(&manifest_dir).join("../../wasm/yield_wasm_test.wasm");
        log::info!("load wasm from {:?}", wasm_path);
        let wasm = std::fs::read(wasm_path).unwrap();

        // load wasm from bytes
        log::info!("pass bytes");
        let asyncify_wasm_bytes = ast_module::pass_async_module(&wasm).unwrap();

        let snapshot = first_run(&config, &asyncify_wasm_bytes, name).await;
        resume_and_run(&config, &asyncify_wasm_bytes, name, snapshot).await;
    }
}
