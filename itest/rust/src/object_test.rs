/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::cell::{Cell, RefCell};
use std::mem;
use std::rc::Rc;

use godot::bind::{godot_api, GodotClass};
use godot::builtin::{
    FromVariant, GodotString, StringName, ToVariant, Variant, VariantConversionError, Vector3,
};
use godot::engine::{
    file_access, Area2D, Camera3D, FileAccess, Node, Node3D, Object, RefCounted, RefCountedVirtual,
};
use godot::obj::{Base, Gd, InstanceId};
use godot::obj::{Inherits, Share};
use godot::sys::{self, GodotFfi};

use crate::{expect_panic, itest, TestContext};

// TODO:
// * make sure that ptrcalls are used when possible (ie. when type info available; maybe GDScript integration test)
// * Deref impl for user-defined types

#[itest]
fn object_construct_default() {
    let obj = Gd::<ObjPayload>::new_default();
    assert_eq!(obj.bind().value, 111);
}

#[itest]
fn object_construct_value() {
    let obj = Gd::new(ObjPayload { value: 222 });
    assert_eq!(obj.bind().value, 222);
}

// TODO(#23): DerefMut on Gd pointer may be used to break subtyping relations
#[itest(skip)]
fn object_subtype_swap() {
    let mut a: Gd<Node> = Node::new_alloc();
    let mut b: Gd<Node3D> = Node3D::new_alloc();

    /*
    let a_id = a.instance_id();
    let b_id = b.instance_id();
    let a_class = a.get_class();
    let b_class = b.get_class();

    dbg!(a_id);
    dbg!(b_id);
    dbg!(&a_class);
    dbg!(&b_class);
    println!("..swap..");
    */

    mem::swap(&mut *a, &mut *b);

    /*
    dbg!(a_id);
    dbg!(b_id);
    dbg!(&a_class);
    dbg!(&b_class);
    */

    // This should not panic
    a.free();
    b.free();
}

#[itest]
fn object_user_roundtrip_return() {
    let value: i16 = 17943;
    let user = ObjPayload { value };

    let obj: Gd<ObjPayload> = Gd::new(user);
    assert_eq!(obj.bind().value, value);

    let ptr = obj.sys();
    std::mem::forget(obj);

    let obj2 = unsafe { Gd::<ObjPayload>::from_sys(ptr) };
    assert_eq!(obj2.bind().value, value);
} // drop

#[itest]
fn object_user_roundtrip_write() {
    let value: i16 = 17943;
    let user = ObjPayload { value };

    let obj: Gd<ObjPayload> = Gd::new(user);
    assert_eq!(obj.bind().value, value);

    let obj2 = unsafe {
        Gd::<ObjPayload>::from_sys_init(|ptr| {
            obj.move_return_ptr(sys::AsUninit::force_init(ptr), sys::PtrcallType::Standard)
        })
    };
    assert_eq!(obj2.bind().value, value);
} // drop

#[itest]
fn object_engine_roundtrip() {
    let pos = Vector3::new(1.0, 2.0, 3.0);

    let mut obj: Gd<Node3D> = Node3D::new_alloc();
    obj.set_position(pos);
    assert_eq!(obj.get_position(), pos);

    let ptr = obj.sys();

    let obj2 = unsafe { Gd::<Node3D>::from_sys(ptr) };
    assert_eq!(obj2.get_position(), pos);
    obj.free();
}

#[itest]
fn object_user_display() {
    let obj = Gd::new(ObjPayload { value: 774 });

    let actual = format!(".:{obj}:.");
    let expected = ".:value=774:.".to_string();

    assert_eq!(actual, expected);
}

#[itest]
fn object_engine_display() {
    let obj = Node3D::new_alloc();
    let id = obj.instance_id();

    let actual = format!(".:{obj}:.");
    let expected = format!(".:<Node3D#{id}>:.");

    assert_eq!(actual, expected);
    obj.free();
}

#[itest]
fn object_debug() {
    let obj = Node3D::new_alloc();
    let id = obj.instance_id();

    let actual = format!(".:{obj:?}:.");
    let expected = format!(".:Gd {{ id: {id}, class: Node3D }}:.");

    assert_eq!(actual, expected);
    obj.free();
}

#[itest]
fn object_instance_id() {
    let value: i16 = 17943;
    let user = ObjPayload { value };

    let obj: Gd<ObjPayload> = Gd::new(user);
    let id = obj.instance_id();

    let obj2 = Gd::<ObjPayload>::from_instance_id(id);
    assert_eq!(obj2.bind().value, value);
}

#[itest]
fn object_instance_id_when_freed() {
    let node: Gd<Node3D> = Node3D::new_alloc();
    assert!(node.is_instance_valid());

    node.share().free(); // destroys object without moving out of reference
    assert!(!node.is_instance_valid());

    expect_panic("instance_id() on dead object", move || {
        node.instance_id();
    });
}

#[itest]
fn object_from_invalid_instance_id() {
    let id = InstanceId::try_from_i64(0xDEADBEEF).unwrap();

    let obj2 = Gd::<ObjPayload>::try_from_instance_id(id);
    assert!(obj2.is_none());
}

#[itest]
fn object_from_instance_id_inherits_type() {
    let descr = GodotString::from("some very long description");

    let mut node: Gd<Node3D> = Node3D::new_alloc();
    node.set_editor_description(descr.clone());

    let id = node.instance_id();

    let node_as_base = Gd::<Node>::from_instance_id(id);
    assert_eq!(node_as_base.instance_id(), id);
    assert_eq!(node_as_base.get_editor_description(), descr);

    node_as_base.free();
}

#[itest]
fn object_from_instance_id_unrelated_type() {
    let node: Gd<Node3D> = Node3D::new_alloc();
    let id = node.instance_id();

    let obj = Gd::<RefCounted>::try_from_instance_id(id);
    assert!(
        obj.is_none(),
        "try_from_instance_id() with bad type must fail"
    );

    node.free();
}

#[itest]
fn object_user_eq() {
    let value: i16 = 17943;
    let a = ObjPayload { value };
    let b = ObjPayload { value };

    let a1 = Gd::new(a);
    let a2 = a1.share();
    let b1 = Gd::new(b);

    assert_eq!(a1, a2);
    assert_ne!(a1, b1);
    assert_ne!(a2, b1);
}

#[itest]
fn object_engine_eq() {
    let a1 = Node3D::new_alloc();
    let a2 = a1.share();
    let b1 = Node3D::new_alloc();

    assert_eq!(a1, a2);
    assert_ne!(a1, b1);
    assert_ne!(a2, b1);

    a1.free();
    b1.free();
}

#[itest]
fn object_dead_eq() {
    let a = Node3D::new_alloc();
    let b = Node3D::new_alloc();
    let b2 = b.share();

    // Destroy b1 without consuming it
    b.share().free();

    {
        let lhs = a.share();
        expect_panic("Gd::eq() panics when one operand is dead", move || {
            let _ = lhs == b;
        });
    }
    {
        let rhs = a.share();
        expect_panic("Gd::ne() panics when one operand is dead", move || {
            let _ = b2 != rhs;
        });
    }

    a.free();
}

#[itest]
fn object_user_convert_variant() {
    let value: i16 = 17943;
    let user = ObjPayload { value };

    let obj: Gd<ObjPayload> = Gd::new(user);
    let variant = obj.to_variant();
    let obj2 = Gd::<ObjPayload>::from_variant(&variant);

    assert_eq!(obj2.bind().value, value);
}

#[itest]
fn object_engine_convert_variant() {
    let pos = Vector3::new(1.0, 2.0, 3.0);

    let mut obj: Gd<Node3D> = Node3D::new_alloc();
    obj.set_position(pos);

    let variant = obj.to_variant();
    let obj2 = Gd::<Node3D>::from_variant(&variant);

    assert_eq!(obj2.get_position(), pos);
    obj.free();
}

#[itest]
fn object_user_convert_variant_refcount() {
    let obj: Gd<ObjPayload> = Gd::new(ObjPayload { value: -22222 });
    let obj = obj.upcast::<RefCounted>();
    check_convert_variant_refcount(obj)
}

#[itest]
fn object_engine_convert_variant_refcount() {
    let obj = RefCounted::new();
    check_convert_variant_refcount(obj);
}

/// Converts between Object <-> Variant and verifies the reference counter at each stage.
fn check_convert_variant_refcount(obj: Gd<RefCounted>) {
    // Freshly created -> refcount 1
    assert_eq!(obj.get_reference_count(), 1);

    {
        // Variant created from object -> increment
        let variant = obj.to_variant();
        assert_eq!(obj.get_reference_count(), 2);

        {
            // Yet another object created *from* variant -> increment
            let another_object = variant.to::<Gd<RefCounted>>();
            assert_eq!(obj.get_reference_count(), 3);
            assert_eq!(another_object.get_reference_count(), 3);
        }

        // `another_object` destroyed -> decrement
        assert_eq!(obj.get_reference_count(), 2);
    }

    // `variant` destroyed -> decrement
    assert_eq!(obj.get_reference_count(), 1);
}

#[itest]
fn object_engine_convert_variant_nil() {
    let nil = Variant::nil();

    assert_eq!(
        Gd::<Area2D>::try_from_variant(&nil),
        Err(VariantConversionError::BadType),
        "try_from_variant(&nil)"
    );

    expect_panic("from_variant(&nil)", || {
        Gd::<Area2D>::from_variant(&nil);
    });
}

#[itest]
fn object_engine_returned_refcount() {
    let Some(file) = FileAccess::open(
        "res://itest.gdextension".into(),
        file_access::ModeFlags::READ,
    ) else {
        panic!("failed to open file used to test FileAccess")
    };
    assert!(file.is_open());

    // There was a bug which incremented ref-counts of just-returned objects, keep this as regression test.
    assert_eq!(file.get_reference_count(), 1);
}

#[itest]
fn object_engine_up_deref() {
    let node3d: Gd<Node3D> = Node3D::new_alloc();
    let id = node3d.instance_id();

    // Deref chain: Gd<Node3D> -> &Node3D -> &Node -> &Object
    assert_eq!(node3d.instance_id(), id);
    assert_eq!(node3d.get_class(), GodotString::from("Node3D"));

    node3d.free();
}

#[itest]
fn object_engine_up_deref_mut() {
    let mut node3d: Gd<Node3D> = Node3D::new_alloc();

    // DerefMut chain: Gd<Node3D> -> &mut Node3D -> &mut Node -> &mut Object
    node3d.set_message_translation(true);
    assert!(node3d.can_translate_messages());

    // DerefMut chain: &mut Node3D -> ...
    let node3d_ref = &mut *node3d;
    node3d_ref.set_message_translation(false);
    assert!(!node3d_ref.can_translate_messages());

    node3d.free();
}

#[itest]
fn object_engine_upcast() {
    let node3d: Gd<Node3D> = Node3D::new_alloc();
    let id = node3d.instance_id();

    let object = node3d.upcast::<Object>();
    assert_eq!(object.instance_id(), id);
    assert_eq!(object.get_class(), GodotString::from("Node3D"));

    // Deliberate free on upcast object
    object.free();
}

#[itest]
fn object_engine_upcast_reflexive() {
    let node3d: Gd<Node3D> = Node3D::new_alloc();
    let id = node3d.instance_id();

    let object = node3d.upcast::<Node3D>();
    assert_eq!(object.instance_id(), id);
    assert_eq!(object.get_class(), GodotString::from("Node3D"));

    object.free();
}

#[itest]
fn object_engine_downcast() {
    let pos = Vector3::new(1.0, 2.0, 3.0);
    let mut node3d: Gd<Node3D> = Node3D::new_alloc();
    node3d.set_position(pos);
    let id = node3d.instance_id();

    let object = node3d.upcast::<Object>();
    let node: Gd<Node> = object.cast::<Node>();
    let node3d: Gd<Node3D> = node.try_cast::<Node3D>().expect("try_cast");

    assert_eq!(node3d.instance_id(), id);
    assert_eq!(node3d.get_position(), pos);

    node3d.free();
}

#[derive(GodotClass)]
struct CustomClassA {}

#[derive(GodotClass)]
struct CustomClassB {}

#[itest]
fn object_reject_invalid_downcast() {
    let instance = Gd::new(CustomClassA {});
    let object = instance.upcast::<Object>();

    assert!(object.try_cast::<CustomClassB>().is_none());
}

#[itest]
fn variant_reject_invalid_downcast() {
    let variant = Gd::new(CustomClassA {}).to_variant();

    assert!(variant.try_to::<Gd<CustomClassB>>().is_err());
    assert!(variant.try_to::<Gd<CustomClassA>>().is_ok());
}

#[itest]
fn object_engine_downcast_reflexive() {
    let node3d: Gd<Node3D> = Node3D::new_alloc();
    let id = node3d.instance_id();

    let node3d: Gd<Node3D> = node3d.cast::<Node3D>();
    assert_eq!(node3d.instance_id(), id);

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
fn object_engine_accept_polymorphic() {
    let mut node = Camera3D::new_alloc();
    let expected_name = StringName::from("Node name");
    let expected_class = GodotString::from("Camera3D");

    node.set_name(GodotString::from(&expected_name));

    let actual_name = accept_node(node.share());
    assert_eq!(actual_name, expected_name);

    let actual_class = accept_object(node.share());
    assert_eq!(actual_class, expected_class);

    node.free();
}

#[itest]
fn object_user_accept_polymorphic() {
    let obj = Gd::new(ObjPayload { value: 123 });
    let expected_class = GodotString::from("ObjPayload");

    let actual_class = accept_refcounted(obj.share());
    assert_eq!(actual_class, expected_class);

    let actual_class = accept_object(obj);
    assert_eq!(actual_class, expected_class);
}

fn accept_node<T>(node: Gd<T>) -> StringName
where
    T: Inherits<Node>,
{
    let up = node.upcast();
    up.get_name()
}

fn accept_refcounted<T>(node: Gd<T>) -> GodotString
where
    T: Inherits<RefCounted>,
{
    let up = node.upcast();
    up.get_class()
}

fn accept_object<T>(node: Gd<T>) -> GodotString
where
    T: Inherits<Object>,
{
    let up = node.upcast();
    up.get_class()
}

#[itest]
fn object_user_upcast() {
    let obj = user_object();
    let id = obj.instance_id();

    let object = obj.upcast::<Object>();
    assert_eq!(object.instance_id(), id);
    assert_eq!(object.get_class(), GodotString::from("ObjPayload"));
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
    assert_eq!(concrete.bind().value, 17943);
}

#[itest]
fn object_user_bad_downcast() {
    let obj = user_object();
    let object = obj.upcast::<Object>();
    let node3d: Option<Gd<Node>> = object.try_cast::<Node>();

    assert!(node3d.is_none());
}

#[itest]
fn object_engine_manual_free() {
    // Tests if no panic or memory leak

    {
        let node = Node3D::new_alloc();
        let node2 = node.share();
        node2.free();
    } // drop(node)
}

/// Tests the [`DynamicRefCount`] destructor when the underlying [`Object`] is already freed.
#[itest]
fn object_engine_shared_free() {
    {
        let node = Node::new_alloc();
        let _object = node.share().upcast::<Object>();
        node.free();
    } // drop(_object)
}

#[itest]
fn object_engine_manual_double_free() {
    expect_panic("double free()", || {
        let node = Node3D::new_alloc();
        let node2 = node.share();
        node.free();
        node2.free();
    });
}

#[itest]
fn object_engine_refcounted_free() {
    let node = RefCounted::new();
    let node2 = node.share().upcast::<Object>();

    expect_panic("calling free() on RefCounted object", || node2.free())
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

#[itest]
fn object_call_no_args() {
    let mut node = Node3D::new_alloc().upcast::<Object>();

    let static_id = node.instance_id();
    let reflect_id_variant = node.call(StringName::from("get_instance_id"), &[]);

    let reflect_id = InstanceId::from_variant(&reflect_id_variant);

    assert_eq!(static_id, reflect_id);
    node.free();
}

#[itest]
fn object_call_with_args() {
    let mut node = Node3D::new_alloc();

    let expected_pos = Vector3::new(2.5, 6.42, -1.11);

    let none = node.call(
        StringName::from("set_position"),
        &[expected_pos.to_variant()],
    );
    let actual_pos = node.call(StringName::from("get_position"), &[]);

    assert_eq!(none, Variant::nil());
    assert_eq!(actual_pos, expected_pos.to_variant());
    node.free();
}

#[itest]
fn object_get_scene_tree(ctx: &TestContext) {
    let node = Node3D::new_alloc();

    let mut tree = ctx.scene_tree.share();
    tree.add_child(node.upcast());

    let count = tree.get_child_count();
    assert_eq!(count, 1);
} // implicitly tested: node does not leak

// ----------------------------------------------------------------------------------------------------------------------------------------------

#[inline(never)] // force to move "out of scope", can trigger potential dangling pointer errors
fn user_object() -> Gd<ObjPayload> {
    let value: i16 = 17943;
    let user = ObjPayload { value };
    Gd::new(user)
}

#[derive(GodotClass, Eq, PartialEq, Debug)]
pub struct ObjPayload {
    value: i16,
}

#[godot_api]
impl RefCountedVirtual for ObjPayload {
    fn init(_base: Base<Self::Base>) -> Self {
        Self { value: 111 }
    }

    fn to_string(&self) -> GodotString {
        format!("value={}", self.value).into()
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

#[derive(GodotClass, Eq, PartialEq, Debug)]
pub struct Tracker {
    drop_count: Rc<RefCell<i32>>,
}

impl Drop for Tracker {
    fn drop(&mut self) {
        //println!("      Tracker::drop");
        *self.drop_count.borrow_mut() += 1;
    }
}

pub mod object_test_gd {
    use godot::prelude::*;

    #[derive(GodotClass)]
    #[class(init, base=Object)]
    struct MockObjRust {
        #[var]
        i: i64,
    }

    #[godot_api]
    impl MockObjRust {}

    #[derive(GodotClass)]
    #[class(init, base=RefCounted)]
    struct MockRefCountedRust {
        #[var]
        i: i64,
    }

    #[godot_api]
    impl MockRefCountedRust {}

    #[derive(GodotClass, Debug)]
    #[class(init, base=RefCounted)]
    struct ObjectTest;

    #[godot_api]
    impl ObjectTest {
        #[func]
        fn pass_object(&self, object: Gd<Object>) -> i64 {
            let i = object.get("i".into()).to();
            object.free();
            i
        }

        #[func]
        fn return_object(&self) -> Gd<Object> {
            Gd::new(MockObjRust { i: 42 }).upcast()
        }

        #[func]
        fn pass_refcounted(&self, object: Gd<RefCounted>) -> i64 {
            object.get("i".into()).to()
        }

        #[func]
        fn pass_refcounted_as_object(&self, object: Gd<Object>) -> i64 {
            object.get("i".into()).to()
        }

        #[func]
        fn return_refcounted(&self) -> Gd<RefCounted> {
            Gd::new(MockRefCountedRust { i: 42 }).upcast()
        }

        #[func]
        fn return_refcounted_as_object(&self) -> Gd<Object> {
            Gd::new(MockRefCountedRust { i: 42 }).upcast()
        }
    }

    // ----------------------------------------------------------------------------------------------------------------------------------------------

    #[derive(GodotClass)]
    #[class(base=Object)]
    pub struct CustomConstructor {
        #[base]
        base: Base<Object>,

        #[var]
        pub val: i64,
    }

    #[godot_api]
    impl CustomConstructor {
        #[func]
        pub fn construct_object(val: i64) -> Gd<CustomConstructor> {
            Gd::with_base(|base| Self { base, val })
        }
    }
}

#[itest]
fn custom_constructor_works() {
    let obj = object_test_gd::CustomConstructor::construct_object(42);
    assert_eq!(obj.bind().val, 42);
    obj.free();
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

#[derive(GodotClass)]
#[class(init, base=Object)]
struct DoubleUse {
    #[base]
    base: Base<Object>,

    used: Cell<bool>,
}

#[godot_api]
impl DoubleUse {
    #[func]
    fn use_1(&self) {
        self.used.set(true);
    }
}

#[derive(GodotClass)]
#[class(init, base=Object)]
struct SignalEmitter {
    #[base]
    base: Base<Object>,
}

#[godot_api]
impl SignalEmitter {
    #[signal]
    fn do_use();
}

#[itest]
/// Test that godot can call a method that takes `&self`, while there already exists an immutable reference
/// to that type acquired through `bind`.
///
/// This test is not signal-specific, the original bug would happen whenever godot would call a method that
/// takes `&self`. However this was the easiest way to test the bug i could find.
fn double_use_reference() {
    let double_use: Gd<DoubleUse> = Gd::new_default();
    let emitter: Gd<SignalEmitter> = Gd::new_default();

    emitter
        .share()
        .upcast::<Object>()
        .connect("do_use".into(), double_use.callable("use_1"));

    let guard = double_use.bind();

    assert!(!guard.used.get());

    emitter
        .share()
        .upcast::<Object>()
        .emit_signal("do_use".into(), &[]);

    assert!(guard.used.get(), "use_1 was not called");

    std::mem::drop(guard);

    double_use.free();
    emitter.free();
}

#[derive(GodotClass)]
#[class(init, base=Object)]
struct GodotApiTest {
    #[base]
    base: Base<Object>,
}

#[godot_api]
impl GodotApiTest {
    #[func]
    fn func_only_mut(&mut self, mut _a: Gd<Object>, mut _b: Gd<Object>) {}
    #[func]
    fn func_mut_and_not_mut(&mut self, _a: Gd<Object>, mut _b: Gd<Object>, _c: Gd<Object>) {}
    // #[func]
    // fn func_optional(&mut self, _a: Gd<Object>, mut _b: Gd<Object>, #[opt(987)] _c: i32) {}
    // #[func] // waiting https://github.com/godotengine/godot/pull/75415
    // /// Godot Docs
    // fn func_docs(&mut self, _a: Gd<Object>, mut _b: Gd<Object>) {}
    // #[func]
    // fn func_lifetime<'a>(&'a mut self) {}
}
