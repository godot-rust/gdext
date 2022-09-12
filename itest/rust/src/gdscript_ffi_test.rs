#![allow(dead_code)]

#[path = "gen/rust_ffi.rs"]
mod rust_ffi;

pub(crate) fn run() -> bool {
    let ok = true;
    ok
}
