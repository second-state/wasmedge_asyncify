// rustup target add wasm32-wasi
// rustc --target=wasm32-wasi -O memory.rs
#[link(wasm_import_module = "host")]
extern "C" {
    fn to_uppercase(ptr: *mut u8, len: usize) -> i32;
}

fn main() {
    let mut s = String::from("hello wasm");
    unsafe { to_uppercase(s.as_mut_ptr(), s.len()) };
    println!("{}", s);
}
