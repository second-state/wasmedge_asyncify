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
