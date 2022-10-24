use log::info;
mod et_poll;

fn main() {
    env_logger::init();
    let mut args = std::env::args();
    info!("args= {:?}", args);
    let first = args.nth(0).unwrap();
    let run_test_name = first.as_str();
    match run_test_name {
        "nslookup" => nslookup_test(),
        "block_socket" => block_socket_test(),
        "et_poll" => et_poll::main_run().unwrap(),
        _ => {}
    }
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
