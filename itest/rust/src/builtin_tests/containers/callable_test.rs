/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use godot::bind::{godot_api, GodotClass};
use godot::builtin::inner::InnerCallable;
use godot::builtin::{
    varray, Callable, GodotString, StringName, ToVariant, Variant, VariantOperator,
};
use godot::engine::{Node2D, Object};
use godot::obj::Gd;

use crate::framework::itest;

#[derive(GodotClass)]
#[class(init, base=RefCounted)]
struct CallableTestObj {
    value: i32,
}

#[godot_api]
impl CallableTestObj {
    #[func]
    fn foo(&mut self, a: i32) {
        self.value = a;
    }

    #[func]
    fn bar(&self, b: i32) -> GodotString {
        b.to_variant().stringify()
    }
}

#[itest]
fn callable_validity() {
    let obj = Gd::<CallableTestObj>::new_default();

    // non-null object, valid method
    assert!(obj.callable("foo").is_valid());
    assert!(!obj.callable("foo").is_null());
    assert!(!obj.callable("foo").is_custom());
    assert!(obj.callable("foo").object().is_some());

    // non-null object, invalid method
    assert!(!obj.callable("doesn't_exist").is_valid());
    assert!(!obj.callable("doesn't_exist").is_null());
    assert!(!obj.callable("doesn't_exist").is_custom());
    assert!(obj.callable("doesn't_exist").object().is_some());

    // null object
    assert!(!Callable::invalid().is_valid());
    assert!(Callable::invalid().is_null());
    assert!(!Callable::invalid().is_custom());
    assert!(Callable::invalid().object().is_none());
}

#[itest]
fn callable_hash() {
    let obj = Gd::<CallableTestObj>::new_default();
    assert_eq!(obj.callable("foo").hash(), obj.callable("foo").hash());
    assert_ne!(obj.callable("foo").hash(), obj.callable("bar").hash());
}

#[itest]
fn callable_object_method() {
    let obj = Gd::<CallableTestObj>::new_default();
    let callable = obj.callable("foo");

    assert_eq!(callable.object(), Some(obj.clone().upcast::<Object>()));
    assert_eq!(callable.object_id(), Some(obj.instance_id()));
    assert_eq!(callable.method_name(), Some("foo".into()));

    assert_eq!(Callable::invalid().object(), None);
    assert_eq!(Callable::invalid().object_id(), None);
    assert_eq!(Callable::invalid().method_name(), None);
}

#[itest]
fn callable_call() {
    let obj = Gd::<CallableTestObj>::new_default();
    let callable = obj.callable("foo");

    assert_eq!(obj.bind().value, 0);
    callable.callv(varray![10]);
    assert_eq!(obj.bind().value, 10);
    callable.callv(varray![20, 30]);
    assert_eq!(obj.bind().value, 20);

    // TODO(bromeon): this causes a Rust panic, but since call() is routed to Godot, the panic is handled at the FFI boundary.
    // Can there be a way to notify the caller about failed calls like that?
    assert_eq!(callable.callv(varray!["string"]), Variant::nil());

    assert_eq!(Callable::invalid().callv(varray![1, 2, 3]), Variant::nil());
}

#[itest]
fn callable_call_return() {
    let obj = Gd::<CallableTestObj>::new_default();
    let callable = obj.callable("bar");

    assert_eq!(
        callable.callv(varray![10]),
        10.to_variant().stringify().to_variant()
    );
    // errors in godot but does not crash
    assert_eq!(callable.callv(varray!["string"]), Variant::nil());
}

#[itest]
fn callable_call_engine() {
    let obj = Node2D::new_alloc();
    let cb = Callable::from_object_method(obj.clone(), "set_position");
    let inner: InnerCallable = cb.as_inner();

    assert!(!inner.is_null());
    assert_eq!(inner.get_object_id(), obj.instance_id().to_i64());
    assert_eq!(inner.get_method(), StringName::from("set_position"));

    // TODO once varargs is available
    // let pos = Vector2::new(5.0, 7.0);
    // inner.call(&[pos.to_variant()]);
    // assert_eq!(obj.get_position(), pos);
    //
    // inner.bindv(array);

    obj.free();
}

// #[cfg(since_api = "4.2")]
mod custom_callable {
    use super::*;
    use godot::builtin::Dictionary;

    #[itest]
    fn callable_custom_invoke() {
        let my_rust_callable = Adder { sum: 0 };
        let callable = Callable::from_custom(my_rust_callable);

        assert!(callable.is_valid());
        assert!(!callable.is_null());
        assert!(callable.is_custom());
        assert!(callable.object().is_none());

        let sum1 = callable.callv(varray![3, 9, 2, 1]);
        assert_eq!(sum1, 15.to_variant());

        let sum2 = callable.callv(varray![4]);
        assert_eq!(sum2, 19.to_variant());
    }

    #[itest]
    fn callable_custom_to_string() {
        let my_rust_callable = Adder { sum: 0 };
        let callable = Callable::from_custom(my_rust_callable);

        println!("to_string: {}", callable);
        println!("equal: {}", callable == callable);
        println!("equal2: {}", callable.to_variant() == callable.to_variant());
        println!("hash: {}", callable.hash());
    }

    #[itest]
    fn callable_custom_equal() {
        let a = Callable::from_custom(Adder { sum: 0 });
        let b = Callable::from_custom(Adder { sum: 0 });
        println!("equal: {}", a == b);
        println!("equal2: {}", a.to_variant() == b.to_variant());

        let mut dict = Dictionary::new();

        dict.insert(a, "hello");
        dict.insert(b, "hi");
    }


    struct Adder {
        sum: i32,
    }

    impl godot::builtin::RustCallable for Adder {
        fn invoke(&mut self, args: &[&Variant]) -> Result<Variant, ()> {
            for arg in args {
                self.sum += arg.to::<i32>();
            }

            Ok(self.sum.to_variant())
        }
    }
}
