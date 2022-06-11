use crate::godot_itest;
use gdext_builtin::Vector3;
use gdext_class::api::Node3D;
use gdext_class::marker::UserClass;
use gdext_class::{GodotClass, GodotExtensionClass, GodotExtensionClassMethods, GodotMethods, Obj};
use gdext_sys::{self as sys, GDNativeExtensionClassCallVirtual, GDNativeObjectPtr, GodotFfi};
use std::fmt::{Debug, Formatter};

pub(crate) fn register() {
    gdext_class::register_class::<ObjPayload>();
}

pub fn run() -> bool {
    let mut ok = true;
   // ok &= object_engine_roundtrip();
    ok &= object_user_roundtrip();
    ok
}

godot_itest! { object_engine_roundtrip {
    let pos = Vector3::new(1.0, 2.0, 3.0);

    let obj: Obj<Node3D> = Node3D::new();
    obj.inner().set_position(pos);
    assert_eq!(obj.inner().get_position(), pos);

    // TODO drop/release?
    let ptr = obj.sys();

    let obj2 = unsafe { Obj::<Node3D>::from_sys(ptr) };
    assert_eq!(obj2.inner().get_position(), pos);
}}

godot_itest! { object_user_roundtrip {
    let value: i16 = 17943;
    let user = ObjPayload { value };

    let obj: Obj<ObjPayload> = Obj::new(user);
    assert_eq!(obj.inner().value, value);

    // TODO drop/release?
    let ptr = obj.sys();

    let obj2 = unsafe { Obj::<ObjPayload>::from_sys(ptr) };
    assert_eq!(obj2.inner().value, value);
}}

// ----------------------------------------------------------------------------------------------------------------------------------------------

#[derive(Debug)]
pub struct ObjPayload {
    value: i16,
}

impl GodotClass for ObjPayload {
    type Base = Node3D;
    type Declarer = UserClass;

    fn class_name() -> String {
        "ObjPayload".to_string()
    }
}
impl GodotExtensionClass for ObjPayload {}
impl GodotExtensionClassMethods for ObjPayload {
    fn virtual_call(_name: &str) -> GDNativeExtensionClassCallVirtual { todo!() }
    fn register_methods() {}
}
impl GodotMethods for ObjPayload{
    fn construct(_base: GDNativeObjectPtr) -> Self {
        ObjPayload { value: 0 }
    }
}