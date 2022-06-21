use gdext_macros::GodotClass;

#[derive(GodotClass)]
#[godot(base = Node3D)]
struct RustApi {
    i: i32,
}

pub fn run() -> bool {
    let mut ok = true;

    ok
}
