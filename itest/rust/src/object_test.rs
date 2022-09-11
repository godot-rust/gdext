use crate::itest;
use gdext_builtin::{GodotString, Variant, Vector3};
use gdext_class::api::{Node, Node3D, Object, RefCounted};
use gdext_class::obj::{Base, Gd};
use gdext_class::out;
use gdext_class::traits::{GodotExt, Share};
use gdext_macros::{godot_api, GodotClass};
use gdext_sys as sys;

use std::cell::RefCell;
use std::rc::Rc;
use sys::GodotFfi;

// pub(crate) fn register() {
//     gdext_class::register_class::<ObjPayload>();
//     gdext_class::register_class::<Tracker>();
// }

pub fn run() -> bool {
    let mut ok = true;
    ok &= object_construct_default();
    ok &= object_construct_value();
    // ok &= object_user_roundtrip_return();
    // ok &= object_user_roundtrip_write();
    ok &= object_engine_roundtrip();
    ok &= object_instance_id();
    ok &= object_user_convert_variant();
    ok &= object_engine_convert_variant();
    ok &= object_engine_upcast();
    ok &= object_engine_downcast();
    ok &= object_engine_bad_downcast();
    ok &= object_user_upcast();
    ok &= object_user_downcast();
    ok &= object_user_bad_downcast();
    ok &= object_engine_manual_drop();
    ok &= object_user_share_drop();
    ok
}

// TODO:
// * make sure that ptrcalls are used when possible (ie. when type info available; maybe GDScript integration test)

#[itest]
fn object_construct_default() {
    let obj = Gd::<ObjPayload>::new_default();
    assert_eq!(obj.inner().value, 111);
}

#[itest]
fn object_construct_value() {
    let obj = Gd::new(ObjPayload { value: 222 });
    assert_eq!(obj.inner().value, 222);
}

/*
#[itest]
fn object_user_roundtrip_return() {
    let value: i16 = 17943;
    let user = ObjPayload { value };

    let obj: Gd<ObjPayload> = Gd::new(user);
    assert_eq!(obj.inner().value, value);

    let ptr = obj.sys();
    // TODO drop/release?

    let obj2 = unsafe { Gd::<ObjPayload>::from_sys(ptr) };
    assert_eq!(obj2.inner().value, value);
}

#[itest]
fn object_user_roundtrip_write() {
    let value: i16 = 17943;
    let user = ObjPayload { value };

    let obj: Gd<ObjPayload> = Gd::new(user);
    assert_eq!(obj.inner().value, value);

    // TODO drop/release?

    let obj2 = unsafe { Gd::<ObjPayload>::from_sys_init(|ptr| obj.write_sys(ptr)) };
    assert_eq!(obj2.inner().value, value);
}
*/

#[itest]
fn object_engine_roundtrip() {
    let pos = Vector3::new(1.0, 2.0, 3.0);

    let mut obj: Gd<Node3D> = Node3D::new_alloc();
    obj.inner_mut().set_position(pos);
    assert_eq!(obj.inner().get_position(), pos);

    let ptr = obj.sys();

    let obj2 = unsafe { Gd::<Node3D>::from_sys(ptr) };
    assert_eq!(obj2.inner().get_position(), pos);
    obj.free();
}

#[itest]
fn object_instance_id() {
    let value: i16 = 17943;
    let user = ObjPayload { value };

    let obj: Gd<ObjPayload> = Gd::new(user);
    let id = obj.instance_id();

    let obj2 = Gd::<ObjPayload>::from_instance_id(id);
    assert_eq!(obj2.inner().value, value);
}

#[itest]
fn object_user_convert_variant() {
    let value: i16 = 17943;
    let user = ObjPayload { value };

    let obj: Gd<ObjPayload> = Gd::new(user);
    let variant = Variant::from(&obj);
    let obj2 = Gd::<ObjPayload>::from(&variant);

    assert_eq!(obj2.inner().value, value);
}

#[itest]
fn object_engine_convert_variant() {
    let pos = Vector3::new(1.0, 2.0, 3.0);

    let mut obj: Gd<Node3D> = Node3D::new_alloc();
    obj.inner_mut().set_position(pos);

    let variant = Variant::from(&obj);
    let obj2 = Gd::<Node3D>::from(&variant);

    assert_eq!(obj2.inner().get_position(), pos);
    obj.free();
}

#[itest]
fn object_engine_upcast() {
    let node3d: Gd<Node3D> = Node3D::new_alloc();
    let id = node3d.instance_id();

    let object = node3d.upcast::<Object>();
    assert_eq!(object.instance_id(), id);
    assert_eq!(object.inner().get_class(), GodotString::from("Node3D"));

    // Deliberate free on upcast object
    object.free();
}

#[itest]
fn object_engine_downcast() {
    let pos = Vector3::new(1.0, 2.0, 3.0);
    let mut node3d: Gd<Node3D> = Node3D::new_alloc();
    node3d.inner_mut().set_position(pos);
    let id = node3d.instance_id();

    let object = node3d.upcast::<Object>();
    let node: Gd<Node> = object.cast::<Node>();
    let node3d: Gd<Node3D> = node.try_cast::<Node3D>().expect("try_cast");

    assert_eq!(node3d.instance_id(), id);
    assert_eq!(node3d.inner().get_position(), pos);

    node3d.free();
}

#[itest]
fn object_engine_bad_downcast() {
    let object: Gd<Object> = Object::new_alloc();
    let free_ref = object.share();
    let node3d: Option<Gd<Node3D>> = object.try_cast::<Node3D>();

    assert!(node3d.is_none());
    free_ref.free();
}

#[itest]
fn object_user_upcast() {
    let obj = user_object();
    let id = obj.instance_id();

    let object = obj.upcast::<Object>();
    assert_eq!(object.instance_id(), id);
    assert_eq!(object.inner().get_class(), GodotString::from("ObjPayload"));
}

#[itest]
fn object_user_downcast() {
    let obj = user_object();
    let id = obj.instance_id();

    let object = obj.upcast::<Object>();
    let intermediate: Gd<RefCounted> = object.cast::<RefCounted>();
    assert_eq!(intermediate.instance_id(), id);

    let concrete: Gd<ObjPayload> = intermediate.try_cast::<ObjPayload>().expect("try_cast");
    assert_eq!(concrete.instance_id(), id);
    assert_eq!(concrete.inner().value, 17943);
}

#[itest]
fn object_user_bad_downcast() {
    let obj = user_object();
    let object = obj.upcast::<Object>();
    let node3d: Option<Gd<Node>> = object.try_cast::<Node>();

    assert!(node3d.is_none());
}

#[itest]
fn object_engine_manual_drop() {
    let panic = std::panic::catch_unwind(|| {
        let node = Node3D::new_alloc();
        let node2 = node.share();
        node.free();
        node2.free();
    });
    assert!(panic.is_err(), "double free() panics");
}

#[itest]
fn object_user_share_drop() {
    let drop_count = Rc::new(RefCell::new(0));

    let object: Gd<Tracker> = Gd::new(Tracker {
        drop_count: Rc::clone(&drop_count),
    });
    assert_eq!(*drop_count.borrow(), 0);

    let shared = object.share();
    assert_eq!(*drop_count.borrow(), 0);

    drop(shared);
    assert_eq!(*drop_count.borrow(), 0);

    drop(object);
    assert_eq!(*drop_count.borrow(), 1);
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

#[inline(never)] // force to move "out of scope", can trigger potential dangling pointer errors
fn user_object() -> Gd<ObjPayload> {
    let value: i16 = 17943;
    let user = ObjPayload { value };
    Gd::new(user)
}

#[derive(GodotClass, Debug, Eq, PartialEq)]
//#[godot(init)]
pub struct ObjPayload {
    value: i16,
}

#[godot_api]
impl GodotExt for ObjPayload {
    fn init(_base: Base<Self::Base>) -> Self {
        Self { value: 111 }
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

#[derive(GodotClass, Debug, Eq, PartialEq)]
pub struct Tracker {
    drop_count: Rc<RefCell<i32>>,
}
/*impl GodotClass for Tracker {
    type Base = RefCounted;
    type Declarer = dom::UserDomain;
    type Mem = mem::StaticRefCount;

    fn class_name() -> String {
        "Tracker".to_string()
    }
}
impl UserMethodBinds for Tracker {
    fn register_methods() {}
}
impl UserVirtuals for Tracker {}
impl GodotExt for Tracker {}
impl GodotDefault for Tracker {
    fn __godot_init(_base: Base<Self::Base>) -> Self {
        panic!("not invoked")
    }
}*/
impl Drop for Tracker {
    fn drop(&mut self) {
        out!("      Tracker::drop");
        *self.drop_count.borrow_mut() += 1;
    }
}
