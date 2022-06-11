use crate::godot_itest;
use gdext_builtin::{Variant, Vector3};
use gdext_class::api::Node3D;
use gdext_class::marker::UserClass;
use gdext_class::{GodotClass, GodotExtensionClass, GodotExtensionClassMethods, GodotMethods, Obj};
use gdext_sys as sys;
use sys::GodotFfi;

pub(crate) fn register() {
    gdext_class::register_class::<ObjPayload>();
}

pub fn run() -> bool {
    let mut ok = true;
    ok &= object_construct_default();
    ok &= object_construct_value();
    ok &= object_user_roundtrip_return();
    ok &= object_user_roundtrip_write();
    ok &= object_engine_roundtrip();
    ok &= object_instance_id();
    ok &= object_user_convert_variant();
    ok &= object_engine_convert_variant();
    ok
}

godot_itest! { object_construct_default {
    let obj = Obj::<ObjPayload>::new_default();
    assert_eq!(obj.inner().value, 111);
}}

godot_itest! { object_construct_value {
    let obj = Obj::new(ObjPayload { value: 222 });
    assert_eq!(obj.inner().value, 222);
}}

godot_itest! { object_user_roundtrip_return {
    let value: i16 = 17943;
    let user = ObjPayload { value };

    let obj: Obj<ObjPayload> = Obj::new(user);
    assert_eq!(obj.inner().value, value);

    let ptr = obj.sys();
    // TODO drop/release?

    let obj2 = unsafe { Obj::<ObjPayload>::from_sys(ptr) };
    assert_eq!(obj2.inner().value, value);
}}

godot_itest! { object_user_roundtrip_write {
    let value: i16 = 17943;
    let user = ObjPayload { value };

    let obj: Obj<ObjPayload> = Obj::new(user);
    assert_eq!(obj.inner().value, value);

    // TODO drop/release?

    let obj2 = unsafe { Obj::<ObjPayload>::from_sys_init(|ptr| obj.write_sys(ptr)) };
    assert_eq!(obj2.inner().value, value);
}}

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

godot_itest! { object_instance_id {
    let value: i16 = 17943;
    let user = ObjPayload { value };

    let obj: Obj<ObjPayload> = Obj::new(user);
    let id = obj.instance_id();

    let obj2 = Obj::<ObjPayload>::from_instance_id(id);
    assert_eq!(obj2.inner().value, value);
}}

godot_itest! { object_user_convert_variant {
    let value: i16 = 17943;
    let user = ObjPayload { value };

    let obj: Obj<ObjPayload> = Obj::new(user);
    let variant = Variant::from(&obj);
    let obj2 = Obj::<ObjPayload>::from(&variant);

    assert_eq!(obj2.inner().value, value);
}}

godot_itest! { object_engine_convert_variant {
    let pos = Vector3::new(1.0, 2.0, 3.0);

    let obj: Obj<Node3D> = Node3D::new();
    obj.inner().set_position(pos);

    let variant = Variant::from(&obj);
    let obj2 = Obj::<Node3D>::from(&variant);

    assert_eq!(obj2.inner().get_position(), pos);
}}

// ----------------------------------------------------------------------------------------------------------------------------------------------

#[derive(Debug, Eq, PartialEq)]
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
    fn virtual_call(_name: &str) -> sys::GDNativeExtensionClassCallVirtual {
        todo!()
    }
    fn register_methods() {}
}
impl GodotMethods for ObjPayload {
    fn construct(_base: sys::GDNativeObjectPtr) -> Self {
        ObjPayload { value: 111 }
    }
}
