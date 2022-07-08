#![allow(dead_code)]

use gdext_class::api::RefCounted;
use gdext_class::Base;
use gdext_macros::{godot_api, GodotClass};



#[derive(GodotClass, Debug)]
#[godot(base = RefCounted)]
struct RustFfi {
    to_mirror: i64,

    #[base]
    some_base: Base<RefCounted>,
}

#[godot_api]
impl RustFfi {
    #[godot]
    fn create_int(&self) -> i64 {
        -468192231
    }

    #[godot]
    fn accept_int(&self, i: i64) -> bool {
        i == -468192231
    }

    #[godot]
    fn mirror_int(&self, i: i64) -> i64 {
        i
    }
}

pub(crate) fn run() -> bool {
    let ok = true;
    ok
}

pub(crate) fn register() {
    gdext_class::register_class::<RustFfi>();
}
