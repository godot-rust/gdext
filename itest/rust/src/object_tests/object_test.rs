/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

// Needed for Clippy to accept #[cfg(all())]
#![allow(clippy::non_minimal_cfg)]

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use godot::builtin::{Array, GString, StringName, Variant, Vector3};
use godot::classes::{
    file_access, Engine, FileAccess, IRefCounted, Node, Node2D, Node3D, Object, RefCounted,
};
use godot::global::godot_str;
use godot::meta::{FromGodot, GodotType, ToGodot};
use godot::obj::{Base, Gd, Inherits, InstanceId, NewAlloc, NewGd, RawGd, Singleton};
use godot::register::{godot_api, GodotClass};
use godot::sys::{self, interface_fn, GodotFfi};

use crate::framework::{expect_panic, expect_panic_or_ub, itest, TestContext};

// TODO:
// * make sure that ptrcalls are used when possible (i.e. when type info available; maybe GDScript integration test)
// * Deref impl for user-defined types

#[itest]
fn object_construct_default() {
    let obj = Gd::<RefcPayload>::default();
    assert_eq!(obj.bind().value, 111);
}

#[itest]
fn object_construct_new_gd() {
    let obj = RefcPayload::new_gd();
    assert_eq!(obj.bind().value, 111);
}

#[itest]
fn object_construct_value() {
    let obj = Gd::from_object(RefcPayload { value: 222 });
    assert_eq!(obj.bind().value, 222);
}

#[itest]
fn object_user_roundtrip_return() {
    let value: i16 = 17943;
    let user = RefcPayload { value };

    let obj: Gd<RefcPayload> = Gd::from_object(user);
    assert_eq!(obj.bind().value, value);

    let raw = obj.to_ffi();
    let ptr = raw.sys();
    std::mem::forget(obj);

    let raw2 = unsafe { RawGd::<RefcPayload>::new_from_sys(ptr) };
    let obj2 = Gd::from_ffi(raw2);
    assert_eq!(obj2.bind().value, value);
} // drop

#[itest]
fn object_user_roundtrip_write() {
    let value: i16 = 17943;
    let user = RefcPayload { value };

    let obj: Gd<RefcPayload> = Gd::from_object(user);
    assert_eq!(obj.bind().value, value);

    // Use into_ffi() instead of to_ffi(), as the latter returns a reference and isn't used for returns anymore.
    let raw = obj.into_ffi();

    let raw2 = unsafe {
        RawGd::<RefcPayload>::new_with_uninit(|ptr| {
            raw.move_return_ptr(sys::SysPtr::force_init(ptr), sys::PtrcallType::Standard)
        })
    };
    let obj2 = Gd::from_ffi(raw2);
    assert_eq!(obj2.bind().value, value);
} // drop

#[itest]
fn object_engine_roundtrip() {
    let pos = Vector3::new(1.0, 2.0, 3.0);

    let mut obj: Gd<Node3D> = Node3D::new_alloc();
    obj.set_position(pos);
    assert_eq!(obj.get_position(), pos);

    let raw = obj.to_ffi();
    let ptr = raw.sys();

    let raw2 = unsafe { RawGd::<Node3D>::new_from_sys(ptr) };
    let obj2 = Gd::from_ffi(raw2);
    assert_eq!(obj2.get_position(), pos);
    obj.free();
}

#[itest]
fn object_option_argument() {
    // Tests following things:
    // - to_godot() returns Option<&T>
    // - None maps to None
    // - Some(gd) maps to Some(&gd)

    let null_obj = None::<Gd<Node>>;
    let via: Option<&Gd<Node>> = null_obj.to_godot();
    assert_eq!(via, None);

    let refc = RefCounted::new_gd();
    let some_obj = Some(refc.clone());
    let via: Option<&Gd<RefCounted>> = some_obj.to_godot();
    assert_eq!(via, Some(&refc));
}

#[itest]
fn object_user_display() {
    let obj = Gd::from_object(RefcPayload { value: 774 });

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
    let user = RefcPayload { value };

    let obj: Gd<RefcPayload> = Gd::from_object(user);
    let id = obj.instance_id();

    let obj2 = Gd::<RefcPayload>::from_instance_id(id);
    assert_eq!(obj2.bind().value, value);
}

#[itest]
fn object_instance_id_when_freed() {
    let node: Gd<Node3D> = Node3D::new_alloc();
    assert!(node.is_instance_valid());

    node.clone().free(); // destroys object without moving out of reference
    assert!(!node.is_instance_valid());

    expect_panic_or_ub("instance_id() on dead object", move || {
        node.instance_id();
    });
}

#[itest]
fn object_from_invalid_instance_id() {
    let id = InstanceId::try_from_i64(0xDEADBEEF).unwrap();

    Gd::<RefcPayload>::try_from_instance_id(id)
        .expect_err("invalid instance id should not return a valid object");
}

#[itest]
fn object_from_instance_id_inherits_type() {
    let descr = GString::from("some very long description");

    let mut node: Gd<Node3D> = Node3D::new_alloc();
    node.set_editor_description(&descr);

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

    Gd::<RefCounted>::try_from_instance_id(id)
        .expect_err("try_from_instance_id() with bad type should fail");

    node.free();
}

#[itest]
fn object_new_has_instance_id() {
    let obj = ObjPayload::new_alloc();
    let _id = obj.instance_id();
    obj.free();
}

#[itest]
fn object_dynamic_free() {
    let mut obj = ObjPayload::new_alloc();
    let id = obj.instance_id();

    obj.call("free", &[]);

    Gd::<ObjPayload>::try_from_instance_id(id)
        .expect_err("dynamic free() call must destroy object");
}

#[itest]
fn object_user_bind_after_free() {
    let obj = Gd::from_object(ObjPayload {});
    let copy = obj.clone();
    obj.free();

    expect_panic_or_ub("bind() on dead user object", move || {
        let _ = copy.bind();
    });
}

#[itest]
fn object_user_free_during_bind() {
    let obj = Gd::from_object(ObjPayload {});
    let guard = obj.bind();

    let copy = obj.clone(); // TODO clone allowed while bound?

    expect_panic_or_ub("direct free() on user while it's bound", move || {
        copy.free();
    });

    drop(guard);
    assert!(
        obj.is_instance_valid(),
        "object lives on after failed free()"
    );

    let copy = obj.clone();
    obj.free(); // now succeeds

    assert!(
        !copy.is_instance_valid(),
        "object is finally destroyed after successful free()"
    );
}

#[itest]
fn object_engine_freed_argument_passing(ctx: &TestContext) {
    let node: Gd<Node> = Node::new_alloc();

    let mut tree = ctx.scene_tree.clone();
    let node2 = node.clone();

    // Destroy object and then pass it to a Godot engine API.
    node.free();
    expect_panic_or_ub("pass freed Gd<T> to Godot engine API (T=Node)", || {
        tree.add_child(&node2);
    });
}

#[itest]
fn object_user_freed_casts() {
    let obj = Gd::from_object(ObjPayload {});
    let obj2 = obj.clone();
    let base_obj = obj.clone().upcast::<Object>();

    // Destroy object and then pass it to a Godot engine API (upcast itself works, see other tests).
    obj.free();
    expect_panic_or_ub("Gd<T>::upcast() on dead object (T=user)", || {
        let _ = obj2.upcast::<Object>();
    });
    expect_panic_or_ub("Gd<T>::cast() on dead object (T=user)", || {
        let _ = base_obj.cast::<ObjPayload>();
    });
}

#[itest]
fn object_user_freed_argument_passing() {
    let obj = Gd::from_object(ObjPayload {});
    let obj = obj.upcast::<Object>();
    let obj2 = obj.clone();

    let mut engine = Engine::singleton();

    // Destroy object and then pass it to a Godot engine API (upcast itself works, see other tests).
    obj.free();
    expect_panic_or_ub("pass freed Gd<T> to Godot engine API (T=user)", || {
        engine.register_singleton("NeverRegistered", &obj2);
    });
}

#[itest(skip)] // This deliberately crashes the engine. Un-skip to manually test this.
fn object_user_dynamic_free_during_bind() {
    // Note: we could also test if GDScript can access free() when an object is bound, to check whether the panic is handled or crashes
    // the engine. However, that is only possible under the following scenarios:
    // 1. Multithreading -- needs to be outlawed on Gd<T> in general, anyway. If we allow a thread-safe Gd<T>, we however need to handle that.
    // 2. Re-entrant calls -- Rust binds a Gd<T>, calls GDScript, which frees the same Gd. This is the same as the test here.
    // 3. Holding a guard (GdRef/GdMut) across function calls -- not possible, guard's lifetime is coupled to a Gd and cannot be stored in
    //    fields or global variables due to that.

    let obj = Gd::from_object(ObjPayload {});
    let guard = obj.bind();

    let mut copy = obj.clone(); // TODO clone allowed while bound?

    // This technically triggers UB, but in practice no one accesses the references.
    // There is no alternative to test this, see destroy_storage() comments.
    copy.call("free", &[]);

    drop(guard);
    assert!(
        !obj.is_instance_valid(),
        "dynamic free() destroys object even if it's bound"
    );
}

// TODO test if engine destroys it, eg. call()

#[itest]
fn object_user_call_after_free() {
    let obj = Gd::from_object(ObjPayload {});
    let mut copy = obj.clone();
    obj.free();

    expect_panic_or_ub("call() on dead user object", move || {
        let _ = copy.call("get_instance_id", &[]);
    });
}

#[itest]
fn object_engine_use_after_free() {
    let node: Gd<Node3D> = Node3D::new_alloc();
    let copy = node.clone();
    node.free();

    expect_panic_or_ub("call method on dead engine object", move || {
        copy.get_position();
    });
}

#[itest]
fn object_engine_use_after_free_varcall() {
    let node: Gd<Node3D> = Node3D::new_alloc();
    let mut copy = node.clone();
    node.free();

    expect_panic_or_ub("call method on dead engine object", move || {
        copy.call_deferred("get_position", &[]);
    });
}

#[itest]
fn object_user_eq() {
    let value: i16 = 17943;
    let a = RefcPayload { value };
    let b = RefcPayload { value };

    let a1 = Gd::from_object(a);
    let a2 = a1.clone();
    let b1 = Gd::from_object(b);

    assert_eq!(a1, a2);
    assert_ne!(a1, b1);
    assert_ne!(a2, b1);
}

#[itest]
fn object_engine_eq() {
    let a1 = Node3D::new_alloc();
    let a2 = a1.clone();
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
    let b2 = b.clone();

    // Destroy b1 without consuming it
    b.clone().free();

    {
        let lhs = a.clone();
        expect_panic_or_ub("Gd::eq() panics when one operand is dead", move || {
            let _ = lhs == b;
        });
    }
    {
        let rhs = a.clone();
        expect_panic_or_ub("Gd::ne() panics when one operand is dead", move || {
            let _ = b2 != rhs;
        });
    }

    a.free();
}

#[itest]
fn object_user_convert_variant() {
    let value: i16 = 17943;
    let user = RefcPayload { value };

    let obj: Gd<RefcPayload> = Gd::from_object(user);
    let variant = obj.to_variant();
    let obj2 = Gd::<RefcPayload>::from_variant(&variant);

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
    let obj: Gd<RefcPayload> = Gd::from_object(RefcPayload { value: -22222 });
    let obj = obj.upcast::<RefCounted>();
    check_convert_variant_refcount(obj)
}

#[itest]
fn object_engine_convert_variant_refcount() {
    let obj = RefCounted::new_gd();
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

    Gd::<Node2D>::try_from_variant(&nil).expect_err("`nil` should not convert to `Gd<Node2D>`");

    expect_panic("from_variant(&nil)", || {
        Gd::<Node2D>::from_variant(&nil);
    });
}

#[itest]
fn object_engine_convert_variant_error() {
    let refc = RefCounted::new_gd();
    let variant = refc.to_variant();
    assert_eq!(refc.test_refcount(), Some(2));

    let err = Gd::<Node2D>::try_from_variant(&variant)
        .expect_err("`Gd<RefCounted>` should not convert to `Gd<Node2D>`");

    // ConvertError::Err holds a copy of the value, i.e. refcount is +1.
    assert_eq!(refc.test_refcount(), Some(3));

    let expected_debug = format!(
        "cannot convert to class Node2D: VariantGd {{ id: {}, class: RefCounted, refc: 3 }}",
        refc.instance_id().to_i64()
    );
    assert_eq!(err.to_string(), expected_debug);
}

#[itest]
fn object_convert_variant_option() {
    let refc = RefCounted::new_gd();
    let variant = refc.to_variant();

    // Variant -> Option<Gd>.
    let gd = Option::<Gd<RefCounted>>::from_variant(&variant);
    assert_eq!(gd, Some(refc.clone()));

    let nil = Variant::nil();
    let gd = Option::<Gd<RefCounted>>::from_variant(&nil);
    assert_eq!(gd, None);

    // Option<Gd> -> Variant.
    let back = Some(refc).to_variant();
    assert_eq!(back, variant);

    let back = None::<Gd<RefCounted>>.to_variant();
    assert_eq!(back, Variant::nil());
}

#[itest]
fn object_engine_returned_refcount() {
    let Some(file) = FileAccess::open("res://itest.gdextension", file_access::ModeFlags::READ)
    else {
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
    assert_eq!(node3d.get_class(), GString::from("Node3D"));

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
    assert_eq!(object.get_class(), GString::from("Node3D"));

    // Deliberate free on upcast object.
    object.free();
}

fn ref_instance_id(obj: &Object) -> InstanceId {
    let obj_ptr = obj.__object_ptr();
    // SAFETY: raw FFI call since we can't access get_instance_id() of a raw Object anymore, and call() needs &mut.
    let raw_id = unsafe { interface_fn!(object_get_instance_id)(obj_ptr) };
    InstanceId::try_from_i64(raw_id as i64).unwrap()
}

#[itest]
fn object_engine_upcast_ref() {
    let node3d: Gd<Node3D> = Node3D::new_alloc();
    let id = node3d.instance_id();

    let object = node3d.upcast_ref::<Object>();
    assert_eq!(ref_instance_id(object), id);
    assert_eq!(object.get_class(), GString::from("Node3D"));

    node3d.free();
}

#[itest]
fn object_engine_upcast_reflexive() {
    let node3d: Gd<Node3D> = Node3D::new_alloc();
    let id = node3d.instance_id();

    let object = node3d.upcast::<Node3D>();
    assert_eq!(object.instance_id(), id);
    assert_eq!(object.get_class(), GString::from("Node3D"));

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
#[class(no_init)]
struct CustomClassA {}

#[derive(GodotClass)]
#[class(no_init)]
struct CustomClassB {}

#[itest]
fn object_reject_invalid_downcast() {
    let instance = Gd::from_object(CustomClassA {});
    let object = instance.upcast::<Object>();

    assert!(object.try_cast::<CustomClassB>().is_err());
}

#[itest]
fn variant_reject_invalid_downcast() {
    let variant = Gd::from_object(CustomClassA {}).to_variant();

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
    let object2 = object.clone();

    let node3d: Result<Gd<Node3D>, Gd<Object>> = object.try_cast::<Node3D>();

    assert_eq!(node3d, Err(object2.clone()));
    object2.free();
}

#[itest]
fn object_engine_accept_polymorphic() {
    let mut node = Node3D::new_alloc();
    let expected_name = StringName::from("Node name");
    let expected_class = GString::from("Node3D");

    // Node::set_name() changed to accept StringName, in https://github.com/godotengine/godot/pull/76560.
    #[cfg(before_api = "4.5")]
    node.set_name(expected_name.arg());
    #[cfg(since_api = "4.5")]
    node.set_name(&expected_name);

    let actual_name = accept_node(node.clone());
    assert_eq!(actual_name, expected_name);

    let actual_class = accept_object(node.clone());
    assert_eq!(actual_class, expected_class);

    node.free();
}

#[itest]
fn object_user_accept_polymorphic() {
    let obj = Gd::from_object(RefcPayload { value: 123 });
    let expected_class = GString::from("RefcPayload");

    let actual_class = accept_refcounted(obj.clone());
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

fn accept_refcounted<T>(node: Gd<T>) -> GString
where
    T: Inherits<RefCounted>,
{
    let up = node.upcast();
    up.get_class()
}

fn accept_object<T>(node: Gd<T>) -> GString
where
    T: Inherits<Object>,
{
    let up = node.upcast();
    up.get_class()
}

#[itest]
fn object_user_upcast() {
    let obj = user_refc_instance();
    let id = obj.instance_id();

    let object = obj.upcast::<Object>();
    assert_eq!(object.instance_id(), id);
    assert_eq!(object.get_class(), GString::from("RefcPayload"));
}

#[itest]
fn object_user_upcast_ref() {
    let obj = user_refc_instance();
    let id = obj.instance_id();

    let object = obj.upcast_ref::<Object>();
    assert_eq!(ref_instance_id(object), id);
    assert_eq!(object.get_class(), GString::from("RefcPayload"));
}

#[itest]
fn object_user_upcast_mut() {
    let mut obj = user_refc_instance();
    let id = obj.instance_id();

    let object = obj.upcast_mut::<Object>();
    assert_eq!(ref_instance_id(object), id);
    assert_eq!(object.get_class(), GString::from("RefcPayload"));
    assert_eq!(object.call("to_string", &[]), "value=17943".to_variant());
}

#[itest]
fn object_user_downcast() {
    let obj = user_refc_instance();
    let id = obj.instance_id();

    let object = obj.upcast::<Object>();
    let intermediate: Gd<RefCounted> = object.cast::<RefCounted>();
    assert_eq!(intermediate.instance_id(), id);

    let concrete: Gd<RefcPayload> = intermediate.try_cast::<RefcPayload>().expect("try_cast");
    assert_eq!(concrete.instance_id(), id);
    assert_eq!(concrete.bind().value, 17943);
}

#[itest]
fn object_user_bad_downcast() {
    let obj = user_refc_instance();
    let object = obj.upcast::<Object>();
    let object2 = object.clone();

    let node3d: Result<Gd<Node>, Gd<Object>> = object.try_cast::<Node>();

    assert_eq!(node3d, Err(object2));
}

#[itest]
fn object_engine_manual_free() {
    // Tests if no panic or memory leak
    {
        let node = Node3D::new_alloc();
        let node2 = node.clone();
        node2.free();
    } // drop(node)
}

/// Tests the [`DynamicRefCount`] destructor when the underlying [`Object`] is already freed.
#[itest]
fn object_engine_shared_free() {
    {
        let node = Node::new_alloc();
        let _object = node.clone().upcast::<Object>();
        node.free();
    } // drop(_object)
}

#[itest]
fn object_engine_manual_double_free() {
    let node = Node3D::new_alloc();
    let node2 = node.clone();
    node.free();

    expect_panic_or_ub("double free()", move || {
        node2.free();
    });
}

#[itest]
fn object_engine_refcounted_free() {
    let node = RefCounted::new_gd();
    let node2 = node.clone().upcast::<Object>();

    expect_panic("calling free() on RefCounted object", || node2.free())
}

#[itest]
fn object_user_double_free() {
    let mut obj = ObjPayload::new_alloc();
    let obj2 = obj.clone();
    obj.call("free", &[]);

    expect_panic_or_ub("double free()", move || {
        obj2.free();
    });
}

#[itest]
fn object_user_share_drop() {
    let drop_count = Rc::new(RefCell::new(0));

    let object: Gd<Tracker> = Gd::from_object(Tracker {
        drop_count: Rc::clone(&drop_count),
    });
    assert_eq!(*drop_count.borrow(), 0);

    let shared = object.clone();
    assert_eq!(*drop_count.borrow(), 0);

    drop(shared);
    assert_eq!(*drop_count.borrow(), 0);

    drop(object);
    assert_eq!(*drop_count.borrow(), 1);
}

#[itest]
fn object_get_scene_tree(ctx: &TestContext) {
    let node = Node3D::new_alloc();

    let mut tree = ctx.scene_tree.clone();
    tree.add_child(&node);

    let count = tree.get_child_count();
    assert_eq!(count, 1);

    // Explicit type as regression test: https://github.com/godot-rust/gdext/pull/1385
    let nodes: Array<Gd<Node>> = tree.get_children();
    assert_eq!(nodes.len(), 1);
} // implicitly tested: node does not leak

#[itest]
fn object_try_to_unique() {
    let a = RefCounted::new_gd();
    let id = a.instance_id();
    let a = a.try_to_unique().expect("a.try_to_unique()");
    assert_eq!(a.instance_id(), id);

    let b = a.clone();
    let (b, ref_count) = b.try_to_unique().expect_err("b.try_to_unique()");
    assert_eq!(b.instance_id(), id);
    assert_eq!(ref_count, 2);
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

#[derive(GodotClass)]
#[class(init, base=Object)]
pub(super) struct ObjPayload {}

#[godot_api]
impl ObjPayload {
    #[signal(__no_builder)]
    fn do_use();

    #[func]
    fn take_1_int(&self, value: i64) -> i64 {
        value
    }

    #[func]
    fn do_panic(&self) {
        // Unicode character as regression test for https://github.com/godot-rust/gdext/issues/384.
        panic!("do_panic exploded 💥");
    }

    // Obtain the line number of the panic!() call above; keep equidistant to do_panic() method.
    pub fn get_panic_line() -> u32 {
        line!() - 5
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

#[inline(never)] // force to move "out of scope", can trigger potential dangling pointer errors
pub(super) fn user_refc_instance() -> Gd<RefcPayload> {
    let value: i16 = 17943;
    let user = RefcPayload { value };
    Gd::from_object(user)
}

#[derive(GodotClass, Eq, PartialEq, Debug)]
pub struct RefcPayload {
    #[var]
    pub(super) value: i16,
}

#[godot_api]
impl IRefCounted for RefcPayload {
    fn init(_base: Base<Self::Base>) -> Self {
        Self { value: 111 }
    }

    fn to_string(&self) -> GString {
        godot_str!("value={}", self.value)
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

#[derive(GodotClass, Eq, PartialEq, Debug)]
#[class(no_init)]
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

    #[derive(GodotClass)]
    #[class(init, base=RefCounted)]
    struct MockRefCountedRust {
        #[var]
        i: i64,
    }

    mod nested {
        use godot::prelude::*;
        #[derive(GodotClass, Debug)]
        #[class(init, base=RefCounted)]
        pub(super) struct ObjectTest;
    }
    use nested::ObjectTest;

    // Disabling signals allows nested::ObjectTest, which would fail otherwise due to generated decl-macro being out-of-scope.
    #[godot_api(no_typed_signals)]
    // #[hint(has_base_field = false)] // if we allow more fine-grained control in the future
    impl nested::ObjectTest {
        #[func]
        fn pass_object(&self, object: Gd<Object>) -> i64 {
            let i = object.get("i").to();
            object.free();
            i
        }

        #[func]
        fn return_object(&self) -> Gd<Object> {
            Gd::from_object(MockObjRust { i: 42 }).upcast()
        }

        #[func]
        fn pass_refcounted(&self, object: Gd<RefCounted>) -> i64 {
            object.get("i").to()
        }

        #[func]
        fn pass_refcounted_as_object(&self, object: Gd<Object>) -> i64 {
            object.get("i").to()
        }

        #[func]
        fn return_refcounted(&self) -> Gd<RefCounted> {
            Gd::from_object(MockRefCountedRust { i: 42 }).upcast()
        }

        #[func]
        fn return_refcounted_as_object(&self) -> Gd<Object> {
            Gd::from_object(MockRefCountedRust { i: 42 }).upcast()
        }

        #[func]
        fn return_self() -> Gd<Self> {
            Gd::from_object(Self)
        }

        #[func]
        fn return_nested_self() -> Array<Gd<<Self as GodotClass>::Base>> {
            array![&Self::return_self()] // implicit upcast
        }

        #[func]
        fn pass_i32(&self, _i: i32) {}

        #[func]
        fn cause_panic(&self) -> Vector3 {
            panic!("Rust panics")
        }
    }

    // ----------------------------------------------------------------------------------------------------------------------------------------------

    #[derive(GodotClass)]
    #[class(base=Object, no_init)]
    pub struct CustomConstructor {
        #[var]
        pub val: i64,
    }

    #[godot_api]
    impl CustomConstructor {
        #[func]
        pub fn construct_object(val: i64) -> Gd<CustomConstructor> {
            Gd::from_init_fn(|_base| Self { val })
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
    used: Cell<bool>,
}

#[godot_api]
impl DoubleUse {
    #[func]
    fn use_1(&self) {
        self.used.set(true);
    }
}

/// Test that Godot can call a method that takes `&self`, while there already exists an immutable reference
/// to that type acquired through `bind`.
///
/// This test is not signal-specific, the original bug would happen whenever Godot would call a method that takes `&self`.
#[itest]
fn double_use_reference() {
    let double_use: Gd<DoubleUse> = DoubleUse::new_alloc();
    let emitter: Gd<ObjPayload> = ObjPayload::new_alloc();

    emitter
        .clone()
        .upcast::<Object>()
        .connect("do_use", &double_use.callable("use_1"));

    let guard = double_use.bind();

    assert!(!guard.used.get());

    emitter
        .clone()
        .upcast::<Object>()
        .emit_signal("do_use", &[]);

    assert!(guard.used.get(), "use_1 was not called");

    drop(guard);

    double_use.free();
    emitter.free();
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

// Test that one class can be declared multiple times (using #[cfg]) without conflicts

#[derive(GodotClass)]
#[class(init, base=Object)]
struct MultipleStructsCfg {}

#[derive(GodotClass)]
#[class(init, base=Object)]
#[cfg(any())]
struct MultipleStructsCfg {}

#[cfg(any())]
#[derive(GodotClass)]
#[class(init, base=Object)]
struct MultipleStructsCfg {}
