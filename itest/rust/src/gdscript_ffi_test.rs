use gdext_class::api::Node3D;
use gdext_class::Obj;
use gdext_macros::GodotClass;

#[derive(GodotClass, Debug)]
#[godot(base = Node3D)]
struct RustApi {
    #[export]
    exp: i32,

    pure_rust: i32,

    #[base]
    some_base: Obj<Node3D>,
}

pub fn run() -> bool {
    let mut ok = true;

    ok
}
