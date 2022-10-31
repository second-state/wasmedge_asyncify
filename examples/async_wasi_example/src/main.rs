use std::path::Path;
use std::{future::Future, path::PathBuf};

use wasmedge_asyncify::{
    module::{AsyncInstance, InstanceSnapshot},
    store::Store,
    wasi::serialize,
    *,
};

fn main() {
    simple_log::quick!("trace");
    single_thread_run(run_serial_test("sleep"));
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

#[allow(unused)]
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

    let r = inst.call("_start", vec![]).await.unwrap();
    log::info!("_start return {:?}", r);
}

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
    wasi_import.data.yield_hook = Some((std::time::Duration::from_secs(3), hook));

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
    let mut inst = AsyncInstance::instance(executor, &mut store, &ast_module).unwrap();

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

    let mut vfs = vec![];
    {
        for s_fd in &wasi_ctx_snapshot.vfs {
            use wasi::snapshots::env::vfs;
            use wasi::snapshots::env::vfs::INode;
            use wasi::snapshots::env::VFD;
            let fd = match s_fd {
                serialize::SerialVFD::Empty => None,
                serialize::SerialVFD::Std { fd: 0 } => {
                    Some(VFD::Inode(INode::Stdin(vfs::WasiStdin)))
                }
                serialize::SerialVFD::Std { fd: 1 } => {
                    Some(VFD::Inode(INode::Stdout(vfs::WasiStdout)))
                }
                serialize::SerialVFD::Std { fd: 2 } => {
                    Some(VFD::Inode(INode::Stderr(vfs::WasiStderr)))
                }

                serialize::SerialVFD::PreOpenDir {
                    guest_path,
                    dir_rights,
                    file_rights,
                } => match guest_path.as_str() {
                    "." => {
                        let mut dir =
                            vfs::WasiPreOpenDir::new(PathBuf::from("."), PathBuf::from("."));
                        dir.dir_rights =
                            wasi::snapshots::common::vfs::WASIRights::from_bits_truncate(
                                *dir_rights,
                            );
                        dir.file_rights =
                            wasi::snapshots::common::vfs::WASIRights::from_bits_truncate(
                                *file_rights,
                            );
                        Some(VFD::Inode(INode::PreOpenDir(dir)))
                    }
                    _ => None,
                },
                serialize::SerialVFD::TcpServer { state } => {
                    use std::net::TcpListener;
                    let wasi_state = state.into();
                    if let Some(addr) = &state.local_addr {
                        if let Ok(s) = TcpListener::bind(addr) {
                            if let Ok(async_socket) =  wasi::snapshots::common::net::async_tokio::AsyncWasiSocket::from_tcplistener(s, wasi_state){
                                vfs.push(Some(VFD::AsyncSocket(async_socket)));
                                continue;
                            }
                        }
                    }

                    vfs.push(Some(VFD::Closed));
                    continue;
                }
                serialize::SerialVFD::UdpSocket { state } => {
                    use std::net::UdpSocket;
                    let wasi_state = state.into();
                    if let Some(addr) = &state.local_addr {
                        if let Ok(s) = UdpSocket::bind(addr) {
                            if let Ok(async_socket) =  wasi::snapshots::common::net::async_tokio::AsyncWasiSocket::from_udpsocket(s, wasi_state){
                                vfs.push(Some(VFD::AsyncSocket(async_socket)));
                                continue;
                            }
                        }
                    }

                    vfs.push(Some(VFD::Closed));
                    continue;
                }
                _ => Some(wasi::snapshots::env::VFD::Closed),
            };

            vfs.push(fd);
        }
    }

    let wasi_ctx = (wasi_ctx_snapshot, vfs).into();
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
    let mut inst = AsyncInstance::instance(executor, &mut store, &ast_module).unwrap();

    log::info!("resume from snapshot");
    inst.apply_snapshot(snapshot).unwrap();

    // call _start function
    log::info!("resume call _start");
    let r = inst.call("_start", vec![]).await.unwrap();
    log::info!("_start return {:?}", r);
}

async fn run_serial_test(name: &str) {
    log::info!("run {name}");
    let config = Some(Config::create().unwrap());
    // read wasm
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let wasm_path = Path::new(&manifest_dir).join("../../wasm/wasi_impl_test.wasm");
    log::info!("load wasm from {:?}", wasm_path);
    let wasm = std::fs::read(wasm_path).unwrap();

    // load wasm from bytes
    log::info!("pass bytes");
    let asyncify_wasm_bytes = ast_module::pass_async_module(&wasm).unwrap();

    let snapshot = first_run(&config, &asyncify_wasm_bytes, name).await;
    resume_and_run(&config, &asyncify_wasm_bytes, name, snapshot).await;
}
