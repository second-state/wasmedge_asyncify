## Prerequisites

Make sure that you will have GCC installed on Ubuntu 20.04.

```shell
$ sudo apt update
$ sudo apt install build-essential
```

Install [Rust](https://www.rust-lang.org/tools/install) and use the following command to install the `wasm32-wasi` target.

```shell
$ rustup target add wasm32-wasi
```

Install [wasmedge CLI tool](https://wasmedge.org/book/en/start/install.html). 

## Hello world

`examples/hello` is a simple demo for wasmedge async function.

You can run it with `cargo run`:

### Run

```shell
$ cargo run --package hello
```
