#![allow(dead_code)]

use gdext_builtin::GodotString;
use gdext_class::api::RefCounted;
use gdext_class::{Base, GodotMethods, Obj};
use gdext_macros::{godot_api, itest, GodotClass};

/// Simple class, that deliberately has no constructor accessible from GDScript
#[derive(GodotClass, Debug)]
#[godot(base=RefCounted)]
struct WithoutInit {
    #[base]
    some_base: Base<RefCounted>,
}

#[derive(GodotClass, Debug)]
#[godot(init, base=RefCounted)]
struct VirtualMethodTest {
    #[base]
    some_base: Base<RefCounted>,

    integer: i32,
}

#[godot_api]
impl VirtualMethodTest {}

#[godot_api]
impl GodotMethods for VirtualMethodTest {
    fn to_string(&self) -> GodotString {
        format!("VirtualMethodTest[integer={}]", self.integer).into()
    }
}

pub(crate) fn run() -> bool {
    let mut ok = true;
    ok &= test_to_string();
    ok
}

// pub(crate) fn register() {
//     gdext_class::register_class::<VirtualMethodTest>();
// }

// ----------------------------------------------------------------------------------------------------------------------------------------------

#[itest]
fn test_to_string() {
    let _obj = Obj::<VirtualMethodTest>::new_default();
    dbg!(_obj);
}
