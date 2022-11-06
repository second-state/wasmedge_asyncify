use log::info;
use std::path::{Path, PathBuf};
use tokio::io::unix::AsyncFd;
use wasmedge_asyncify::{
    ast_module::pass_async_module,
    module::{AsyncInstance, InstanceSnapshot},
    store::Store,
    wasi::{
        self,
        serialize::{self, IoState},
    },
    Config, Executor, Loader,
};

fn load_wasm_bytes(path: &str) -> Vec<u8> {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let wasm_path = Path::new(&manifest_dir).join(&format!("../../{}", path));
    info!("load wasm from {:?}", wasm_path);
    let wasm = std::fs::read(wasm_path).unwrap();
    wasm
}

async fn run_until_yield<'a>(
    config: &Option<Config>,
    asyncify_wasm_bytes: &[u8],
) -> (InstanceSnapshot, serialize::SerialWasiCtx) {
    let executor = Executor::create(&config).unwrap();
    let mut store = Store::create().unwrap();
    let mut wasi_import = wasmedge_asyncify::wasi::AsyncWasiImport::new().unwrap();
    wasi_import.push_env("RUST_LOG", "info");

    fn hook(io_state: &IoState) -> bool {
        log::info!("hook! {:?}", io_state);
        match io_state {
            IoState::Accept { .. } => true,
            _ => false,
        }
    }
    // set io timeout with 5s
    wasi_import.data.yield_hook = Some((std::time::Duration::from_secs(10), hook));

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
    let mut inst = AsyncInstance::instance(executor, &mut store, &ast_module).unwrap();

    // call _start function
    log::info!("first call _start");
    let r = inst.call("_start", vec![]).await;
    if let Err(e) = &r {
        if e.is_yield() {
            log::info!("yield!");
        } else {
            Err(e.clone()).unwrap()
        }
    }
    let snapshot = inst.snapshot();
    let wasi_snapshot: serialize::SerialWasiCtx = (&wasi_import.data.wasi_ctx).into();

    (snapshot, wasi_snapshot)
}

async fn resume_and_run(
    config: &Option<Config>,
    asyncify_wasm_bytes: &[u8],
    snapshot: (InstanceSnapshot, serialize::SerialWasiCtx),
    listener: std::net::TcpListener,
) {
    let (snapshot, wasi_ctx_snapshot) = snapshot;

    let executor = Executor::create(&config).unwrap();
    let mut store = Store::create().unwrap();

    // resume wasi
    let mut vfs = vec![];
    {
        let listen_addr = match &wasi_ctx_snapshot.io_state {
            IoState::Accept { bind } => Some(bind.clone()),
            _ => None,
        };

        let mut listener = Some(listener);

        for s_fd in &wasi_ctx_snapshot.vfs {
            let fd = match s_fd {
                serialize::SerialVFD::Empty => None,
                serialize::SerialVFD::Stdin(s) => Some(s.clone().into()),
                serialize::SerialVFD::Stdout(s) => Some(s.clone().into()),
                serialize::SerialVFD::Stderr(s) => Some(s.clone().into()),

                serialize::SerialVFD::PreOpenDir(dir) => match dir.guest_path.as_str() {
                    "." => Some(dir.clone().to_vfd(PathBuf::from("."))),
                    _ => None,
                },
                serialize::SerialVFD::TcpServer(s) => {
                    if s.state.local_addr == listen_addr && listener.is_some() {
                        Some(
                            s.clone()
                                .to_async_socket_with_std(listener.take().unwrap())
                                .unwrap(),
                        )
                    } else {
                        Some(s.clone().default_to_async_socket().unwrap())
                    }
                }
                serialize::SerialVFD::UdpSocket(s) => {
                    Some(s.clone().default_to_async_socket().unwrap())
                }
                _ => Some(wasi::snapshots::env::VFD::Closed),
            };

            vfs.push(fd);
        }
    }

    let wasi_ctx = (wasi_ctx_snapshot, vfs).into();
    let mut wasi_import =
        wasmedge_asyncify::wasi::AsyncWasiImport::with_wasi_ctx(wasi_ctx).unwrap();

    store
        .register_import_object(&executor, &mut wasi_import)
        .unwrap();

    let loader = Loader::create(&config).unwrap();
    let ast_module = loader
        .load_async_module_from_bytes(asyncify_wasm_bytes)
        .unwrap();
    // instance wasm
    let mut inst = AsyncInstance::instance(executor, &mut store, &ast_module).unwrap();

    log::info!("resume inst");
    inst.apply_snapshot(snapshot).unwrap();

    // call _start function
    log::info!("resume run");
    inst.call("_start", vec![]).await.unwrap();
}

#[tokio::main]
async fn main() {
    simple_log::quick!("info");
    // read wasm
    let wasm = load_wasm_bytes("wasm/simple_http_server.wasm");

    // pass async module
    let async_wasm = pass_async_module(&wasm).unwrap();

    let config = Config::create();

    let (snapshot, data) = run_until_yield(&config, &async_wasm).await;

    // we can save snapshot & data into some file or database,
    // and load then in other process
    info!("wasm is idle, save it into file or database");

    // maybe in a new process.
    // only listener a port, until a new connect.
    info!("create a new listener on 1234");
    let listener = std::net::TcpListener::bind("0.0.0.0:1234").unwrap();
    listener.set_nonblocking(true).unwrap();
    let async_fd = AsyncFd::with_interest(listener, tokio::io::Interest::READABLE).unwrap();
    // wait a new connect, but not accept;
    info!("wait connect");
    let _ = async_fd.readable().await.unwrap();

    // resume wasm
    resume_and_run(
        &config,
        &async_wasm,
        (snapshot, data),
        async_fd.into_inner(),
    )
    .await;
}
