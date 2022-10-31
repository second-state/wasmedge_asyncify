use log::info;
use std::fs;

fn main() {
    env_logger::init();
    let mut args = std::env::args();
    info!("args= {:?}", args);
    let first = args.nth(0).unwrap();
    let run_test_name = first.as_str();
    match run_test_name {
        "list_cwd" => list_with_yield(),
        "block_socket" => block_socket(),
        _ => {}
    }
}

pub fn list_with_yield() {
    fn list_cwd(times: &str) {
        let dirs = fs::read_dir(".").unwrap();
        for dir in dirs {
            log::info!("{times} {:?}", dir);
        }
    }

    list_cwd("first");
    std::thread::sleep(std::time::Duration::from_secs(3));
    list_cwd("second");
}

fn block_socket() {
    use std::io::Write;
    use wasmedge_wasi_socket::*;
    let s = TcpListener::bind("0.0.0.0:1234", false).unwrap();
    info!("listen on 1234");
    fn handler(cs: (TcpStream, SocketAddr)) {
        info!("accept {cs:?}");
        let mut cs = cs.0;
        writeln!(cs, "hello").unwrap();
    }
    for _i in 0..3 {
        info!("accept...");
        let cs = s.accept(false).unwrap();
        handler(cs)
    }
}
