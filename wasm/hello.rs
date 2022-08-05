// rustup target add wasm32-wasi
// rustc --target=wasm32-wasi -O  hello.rs
#[link(wasm_import_module = "host")]
extern "C" {
    fn sleep();
}

fn main() {
    println!("wasm[{:?}]=> hello", std::env::var("a"));
    unsafe { sleep() };
    println!("wasm=> world");
}
