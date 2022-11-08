use log::{error, info};
use std::io::{Read, Write};
use wasmedge_wasi_socket::*;

fn main() {
    env_logger::init();

    let s = TcpListener::bind("0.0.0.0:1234", false).unwrap();
    fn handle(cs: (TcpStream, SocketAddr)) -> std::io::Result<()> {
        info!("accept {cs:?}");
        let mut cs = cs.0;
        writeln!(cs, "hello")?;
        let mut recv_buf = [0u8; 128];
        let n = cs.read(&mut recv_buf)?;
        let s = std::str::from_utf8(&recv_buf[0..n]);
        info!("recv: {:?}", s);
        Ok(())
    }
    for _i in 0..3 {
        let cs = s.accept(false).unwrap();
        if let Err(e) = handle(cs) {
            error!("handle error: {}", e);
        };
    }
}
