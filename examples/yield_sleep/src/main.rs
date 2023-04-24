use log::info;
use std::{
    path::{Path, PathBuf},
    time::{Duration, SystemTime},
};

use wasmedge_asyncify::{
    ast_module::pass_async_module,
    error::CoreError,
    module::{AsyncInstance, InstanceSnapshot},
    store::Store,
    types::WasmVal,
    wasi::{
        self,
        serialize::{self, IoState, SerialWasiCtx},
        AsyncWasiImport,
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

async fn run_wasm<'a>(
    config: &Option<Config>,
    asyncify_wasm_bytes: &[u8],
    snapshot: (Option<InstanceSnapshot>, Option<serialize::SerialWasiCtx>),
) -> Result<Vec<WasmVal>, (InstanceSnapshot, serialize::SerialWasiCtx)> {
    let executor = Executor::create(&config).unwrap();
    let mut store = Store::create().unwrap();

    let (snapshot, wasi_ctx) = snapshot;

    let mut wasi_import = if let Some(ctx) = wasi_ctx {
        resume_wasi(ctx)
    } else {
        let mut wasi_import = AsyncWasiImport::new().unwrap();
        wasi_import.push_env("RUST_LOG", "info");
        wasi_import
            .push_preopen(".".parse().unwrap(), ".".parse().unwrap())
            .unwrap();
        wasi_import
    };

    fn hook(io_state: &IoState) -> bool {
        log::info!("hook!");
        match io_state {
            IoState::Sleep { ddl } => {
                let timeout = ddl.duration_since(SystemTime::now());
                // wasm yield if sleep time over 30s
                if let Ok(dur) = timeout {
                    dur > Duration::from_secs(30)
                } else {
                    false
                }
            }
            _ => false,
        }
    }
    // check io every 5s
    wasi_import.data.yield_hook = Some((std::time::Duration::from_secs(5), hook));

    store
        .register_import_object(&executor, &mut wasi_import)
        .unwrap();

    let loader = Loader::create(&config).unwrap();
    let ast_module = loader
        .load_async_module_from_bytes(asyncify_wasm_bytes)
        .unwrap();

    // instance wasm
    let mut inst = AsyncInstance::instance(executor, store, &ast_module).unwrap();
    if let Some(s) = snapshot {
        inst.apply_snapshot(s).unwrap();
    }
    // call _start function
    log::info!("run wasm");
    let r = inst.call("_start", vec![]).await;
    match r {
        Ok(r) => Ok(r),
        Err(CoreError::Yield) => {
            log::info!("yield");
            let snapshot = inst.snapshot();
            let wasi_snapshot: serialize::SerialWasiCtx = (&wasi_import.data.wasi_ctx).into();

            Err((snapshot, wasi_snapshot))
        }
        Err(e) => Err(e).unwrap(),
    }
}

fn resume_wasi(wasi_ctx_snapshot: SerialWasiCtx) -> AsyncWasiImport {
    let wasi_ctx = wasi_ctx_snapshot.resume(|s_fd| {
        let fd = match s_fd {
            serialize::SerialVFD::Stdin(s) => s.clone().into(),
            serialize::SerialVFD::Stdout(s) => s.clone().into(),
            serialize::SerialVFD::Stderr(s) => s.clone().into(),

            serialize::SerialVFD::PreOpenDir(dir) => match dir.guest_path.as_str() {
                "." => dir.clone().to_vfd(PathBuf::from(".")),
                _ => wasi::snapshots::env::VFD::Closed,
            },
            serialize::SerialVFD::TcpServer(s) => s.clone().default_to_async_socket().unwrap(),
            serialize::SerialVFD::UdpSocket(s) => s.clone().default_to_async_socket().unwrap(),
            _ => wasi::snapshots::env::VFD::Closed,
        };
        fd
    });

    AsyncWasiImport::with_wasi_ctx(wasi_ctx).unwrap()
}

struct Task {
    ddl: SystemTime,
    task: (InstanceSnapshot, SerialWasiCtx),
}

use tokio::sync::mpsc;
// any service like linux crond
struct CrondMoke {
    task_receiver: mpsc::Receiver<Task>,
    task_sender: mpsc::Sender<Task>,
}

impl CrondMoke {
    fn new() -> Self {
        let (wx, rx) = mpsc::channel(10);
        CrondMoke {
            task_receiver: rx,
            task_sender: wx,
        }
    }

    async fn wait_task(task: Task) -> Task {
        let dur = task.ddl.duration_since(SystemTime::now());
        if let Ok(dur) = dur {
            tokio::time::sleep(dur).await;
            task
        } else {
            task
        }
    }

    async fn save_task(&mut self, task: Task) {
        let wx = self.task_sender.clone();
        tokio::spawn(async move {
            let _ = wx.send(Self::wait_task(task).await).await;
        });
    }

    async fn take_task(&mut self) -> Task {
        self.task_receiver.recv().await.unwrap()
    }
}

#[tokio::main]
async fn main() {
    simple_log::quick!("info");
    let mut crond = CrondMoke::new();

    // read wasm
    let wasm = load_wasm_bytes("wasm/delay_task.wasm");

    // pass async module
    let async_wasm = pass_async_module(&wasm).unwrap();

    let config = Config::create();
    let mut snapshot = None;
    let mut wasi_snapshot = None;

    loop {
        match run_wasm(
            &config,
            &async_wasm,
            (snapshot.take(), wasi_snapshot.take()),
        )
        .await
        {
            Ok(_r) => {
                log::info!("wasm exit");
                break;
            }
            Err((s, wasi)) => {
                let ddl = if let IoState::Sleep { ddl } = &wasi.io_state {
                    ddl.clone()
                } else {
                    // because fn hook return true only Sleep;
                    unreachable!()
                };
                crond
                    .save_task(Task {
                        ddl,
                        task: (s, wasi),
                    })
                    .await;
                info!("save 1 task");
            }
        }

        let Task { task, .. } = crond.take_task().await;
        info!("take 1 task");
        let (s, wasi) = task;
        snapshot = Some(s);
        wasi_snapshot = Some(wasi);
    }
}
