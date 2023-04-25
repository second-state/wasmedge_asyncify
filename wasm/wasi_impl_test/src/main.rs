use log::info;
mod et_poll;
mod fs;

fn main() {
    env_logger::init();
    let mut args = std::env::args();
    info!("args= {:?}", args);
    let first = args.nth(0).unwrap();
    let run_test_name = first.as_str();
    match run_test_name {
        "nslookup" => nslookup_test(),
        "block_socket" => block_socket_test(),
        "accept_would_block" => accept_would_block(),
        "connect_in_progress" => connect_in_progress(),
        "et_poll" => et_poll::main_run().unwrap(),
        "sleep" => sleep_test(),
        "list_cwd" => fs::list_cwd(),
        "tokio_sleep" => tokio_sleep_run(),
        "bind_device" => sock_bind_device(),
        _ => {}
    }
}

fn sleep_test() {
    info!("sleep 5s ...");
    std::thread::sleep(std::time::Duration::from_secs(5));
    info!("sleep 5s wake up!")
}

fn nslookup_test() {
    use wasmedge_wasi_socket::*;
    let s = nslookup_v4("httpbin.org");
    info!("result = {s:?}")
}

fn block_socket_test() {
    use std::io::Write;
    use wasmedge_wasi_socket::*;
    let s = TcpListener::bind("0.0.0.0:1234", false).unwrap();
    fn handler(cs: (TcpStream, SocketAddr)) {
        info!("accept {cs:?}");
        let mut cs = cs.0;
        writeln!(cs, "hello").unwrap();
        cs.shutdown(std::net::Shutdown::Both).unwrap()
    }
    for _i in 0..3 {
        let cs = s.accept(false).unwrap();
        handler(cs)
    }
}

fn accept_would_block() {
    use wasmedge_wasi_socket::*;
    let s = TcpListener::bind("0.0.0.0:1234", true).unwrap();

    let e = s.accept(false).unwrap_err();
    assert_eq!(e.kind(), std::io::ErrorKind::WouldBlock, "{}", e);
}

fn connect_in_progress() {
    use wasmedge_wasi_socket::*;
    let _s = TcpListener::bind("0.0.0.0:1234", true).unwrap();

    let cs = socket::Socket::new(socket::AddressFamily::Inet4, socket::SocketType::Stream).unwrap();
    let addr = "127.0.0.1:1234".parse().unwrap();
    cs.set_nonblocking(true).unwrap();
    let e = cs.connect(&addr).unwrap_err();
    assert_eq!(e.raw_os_error(), Some(libc::EINPROGRESS), "{}", e);
}

fn sock_bind_device() {
    use std::os::wasi::prelude::AsRawFd;
    use wasmedge_wasi_socket::*;

    let cs = socket::Socket::new(socket::AddressFamily::Inet4, socket::SocketType::Stream).unwrap();
    let addr = "127.0.0.1:1234".parse().unwrap();
    // cs.set_nonblocking(true).unwrap();
    unsafe {
        #[link(wasm_import_module = "wasi_snapshot_preview1")]
        extern "C" {
            pub fn sock_bind_device(fd: u32, name: *mut u8, len: u32) -> u32;
        }

        let fd = cs.as_raw_fd() as u32;
        let mut device_name = b"lo".to_vec();
        let res = sock_bind_device(fd, device_name.as_mut_ptr(), device_name.len() as _);
        let res = if res != 0 {
            Err(std::io::Error::from_raw_os_error(res as i32))
        } else {
            Ok(())
        };
        info!("bind_device res = {res:?}");
    }

    let e = cs.connect(&addr);
    info!("connect res = {e:?}");
    // assert_eq!(e.raw_os_error(), Some(libc::EINPROGRESS), "{}", e);
}

fn tokio_sleep_run() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(tokio_sleep());
}

async fn tokio_sleep() {
    loop {
        info!("connect");
        let timeout = tokio::time::timeout(
            std::time::Duration::from_secs(1),
            tokio::time::sleep(std::time::Duration::from_secs(5)),
        )
        .await;
        info!("exit timeout");
        if let Ok(_) = timeout {
            break;
        } else {
            info!("reconnect");
        };
    }
}
