use log::info;

#[link(wasm_import_module = "host")]
extern "C" {
    fn async_counting();
}

fn main() {
    env_logger::init();
    let mut times = 0;
    for _ in 0..5 {
        info!("times in wasm = {}", times);
        times += 1;
        unsafe { async_counting() };
    }
}
