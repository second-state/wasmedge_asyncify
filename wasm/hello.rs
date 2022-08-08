// rustup target add wasm32-wasi
// rustc --target=wasm32-wasi -O hello.rs
#[link(wasm_import_module = "host")]
extern "C" {
    fn sleep();
}

fn main() {
    println!(
        "env(a)={:?} env(b)={:?}",
        std::env::var("a"),
        std::env::var("b")
    );
    println!("wasm: hello");
    unsafe { sleep() };
    println!("wasm: world");
}
