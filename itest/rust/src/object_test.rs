use crate::itest;
use gdext_builtin::{GodotString, Variant, Vector3};
use gdext_class::api::{Node, Node3D, Object, RefCounted};
use gdext_class::{
    dom, mem, out, Base, GodotClass, GodotDefault, GodotExtensionClass, GodotMethods, Inherits,
    Obj, Share,
};
use gdext_sys as sys;
use std::cell::RefCell;
use std::rc::Rc;
use sys::GodotFfi;

pub(crate) fn register() {
    gdext_class::register_class::<ObjPayload>();
    gdext_class::register_class::<Tracker>();
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
    ok &= object_upcast();
    ok &= object_downcast();
    ok &= object_bad_downcast();
    ok &= object_user_upcast();
    ok &= object_user_downcast();
    ok &= object_user_bad_downcast();
    ok &= object_share_drop();
    ok
}

// TODO:
// * make sure that ptrcalls are used when possible (ie. when type info available; maybe GDScript integration test)

#[itest]
fn object_construct_default() {
    let obj = Obj::<ObjPayload>::new_default();
    assert_eq!(obj.inner().value, 111);
}

#[itest]
fn object_construct_value() {
    let obj = Obj::new(ObjPayload { value: 222 });
    assert_eq!(obj.inner().value, 222);
}

#[itest]
fn object_user_roundtrip_return() {
    let value: i16 = 17943;
    let user = ObjPayload { value };

    let obj: Obj<ObjPayload> = Obj::new(user);
    assert_eq!(obj.inner().value, value);

    let ptr = obj.sys();
    // TODO drop/release?

    let obj2 = unsafe { Obj::<ObjPayload>::from_sys(ptr) };
    assert_eq!(obj2.inner().value, value);
}

#[itest]
fn object_user_roundtrip_write() {
    let value: i16 = 17943;
    let user = ObjPayload { value };

    let obj: Obj<ObjPayload> = Obj::new(user);
    assert_eq!(obj.inner().value, value);

    // TODO drop/release?

    let obj2 = unsafe { Obj::<ObjPayload>::from_sys_init(|ptr| obj.write_sys(ptr)) };
    assert_eq!(obj2.inner().value, value);
}

#[itest]
fn object_engine_roundtrip() {
    let pos = Vector3::new(1.0, 2.0, 3.0);

    let mut obj: Obj<Node3D> = Node3D::new();
    obj.inner_mut().set_position(pos);
    assert_eq!(obj.inner().get_position(), pos);

    // TODO drop/release?
    let ptr = obj.sys();

    let obj2 = unsafe { Obj::<Node3D>::from_sys(ptr) };
    assert_eq!(obj2.inner().get_position(), pos);
}

#[itest]
fn object_instance_id() {
    let value: i16 = 17943;
    let user = ObjPayload { value };

    let obj: Obj<ObjPayload> = Obj::new(user);
    let id = obj.instance_id();

    let obj2 = Obj::<ObjPayload>::from_instance_id(id);
    assert_eq!(obj2.inner().value, value);
}

#[itest]
fn object_user_convert_variant() {
    let value: i16 = 17943;
    let user = ObjPayload { value };

    let obj: Obj<ObjPayload> = Obj::new(user);
    let variant = Variant::from(&obj);
    let obj2 = Obj::<ObjPayload>::from(&variant);

    assert_eq!(obj2.inner().value, value);
}

#[itest]
fn object_engine_convert_variant() {
    let pos = Vector3::new(1.0, 2.0, 3.0);

    let mut obj: Obj<Node3D> = Node3D::new();
    obj.inner_mut().set_position(pos);

    let variant = Variant::from(&obj);
    let obj2 = Obj::<Node3D>::from(&variant);

    assert_eq!(obj2.inner().get_position(), pos);
}

#[itest]
fn object_upcast() {
    let node3d: Obj<Node3D> = Node3D::new();
    let id = node3d.instance_id();

    let object = node3d.upcast::<Object>();
    assert_eq!(object.instance_id(), id);
    assert_eq!(object.inner().get_class(), GodotString::from("Node3D"));
}

#[itest]
fn object_downcast() {
    let pos = Vector3::new(1.0, 2.0, 3.0);
    let mut node3d: Obj<Node3D> = Node3D::new();
    node3d.inner_mut().set_position(pos);
    let id = node3d.instance_id();

    let object = node3d.upcast::<Object>();
    let node: Obj<Node> = object.cast::<Node>();
    let node3d: Obj<Node3D> = node.try_cast::<Node3D>().expect("try_cast");

    assert_eq!(node3d.instance_id(), id);
    assert_eq!(node3d.inner().get_position(), pos);
}

#[itest]
fn object_bad_downcast() {
    let object: Obj<Object> = Object::new();
    let node3d: Option<Obj<Node3D>> = object.try_cast::<Node3D>();

    assert!(node3d.is_none());
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
    let intermediate: Obj<Node3D> = object.cast::<Node3D>();
    assert_eq!(intermediate.instance_id(), id);

    let concrete: Obj<ObjPayload> = intermediate.try_cast::<ObjPayload>().expect("try_cast");
    assert_eq!(concrete.instance_id(), id);
    assert_eq!(concrete.inner().value, 17943);
}

#[itest]
fn object_user_bad_downcast() {
    let obj = user_object();
    let object = obj.upcast::<Object>();
    let node3d: Option<Obj<RefCounted>> = object.try_cast::<RefCounted>();

    assert!(node3d.is_none());
}

#[itest]
fn object_share_drop() {
    let drop_count = Rc::new(RefCell::new(0));

    let object: Obj<Tracker> = Obj::new(Tracker {
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
fn user_object() -> Obj<ObjPayload> {
    let value: i16 = 17943;
    let user = ObjPayload { value };
    Obj::new(user)
}

#[derive(Debug, Eq, PartialEq)]
pub struct ObjPayload {
    value: i16,
}
impl GodotClass for ObjPayload {
    type Base = Node3D;
    type Declarer = dom::UserDomain;
    type Mem = mem::ManualMemory;

    fn class_name() -> String {
        "ObjPayload".to_string()
    }
}
impl GodotExtensionClass for ObjPayload {
    fn virtual_call(_name: &str) -> sys::GDNativeExtensionClassCallVirtual {
        todo!()
    }
    fn register_methods() {}
}
impl GodotDefault for ObjPayload {
    fn construct(_base: Base<Self::Base>) -> Self {
        ObjPayload { value: 111 }
    }
}
impl GodotMethods for Tracker {
    fn construct(_base: Obj<Self::Base>) -> Self {
        todo!()
    }
}
impl Inherits<Object> for ObjPayload {}
impl Inherits<Node> for ObjPayload {}
impl Inherits<Node3D> for ObjPayload {}

// ----------------------------------------------------------------------------------------------------------------------------------------------

#[derive(Debug, Eq, PartialEq)]
pub struct Tracker {
    drop_count: Rc<RefCell<i32>>,
}
impl GodotClass for Tracker {
    type Base = RefCounted;
    type Declarer = dom::UserDomain;
    type Mem = mem::StaticRefCount;

    fn class_name() -> String {
        "Tracker".to_string()
    }
}
impl GodotExtensionClass for Tracker {
    fn virtual_call(_name: &str) -> sys::GDNativeExtensionClassCallVirtual {
        todo!()
    }
    fn register_methods() {}
}
impl GodotDefault for Tracker {
    fn construct(_base: Base<Self::Base>) -> Self {
        panic!("not invoked")
    }
}
impl Drop for Tracker {
    fn drop(&mut self) {
        out!("      Tracker::drop");
        *self.drop_count.borrow_mut() += 1;
    }
}
