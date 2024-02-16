/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use godot::builtin::inner::InnerCallable;
use godot::builtin::meta::ToGodot;
use godot::builtin::{varray, Callable, GString, StringName, Variant};
use godot::engine::{Node2D, Object};
use godot::obj::{NewAlloc, NewGd};
use godot::register::{godot_api, GodotClass};

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
    fn bar(&self, b: i32) -> GString {
        b.to_variant().stringify()
    }
}

#[itest]
fn callable_validity() {
    let obj = CallableTestObj::new_gd();

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
    let obj = CallableTestObj::new_gd();
    assert_eq!(obj.callable("foo").hash(), obj.callable("foo").hash());
    assert_ne!(obj.callable("foo").hash(), obj.callable("bar").hash());
}

#[itest]
fn callable_object_method() {
    let obj = CallableTestObj::new_gd();
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
    let obj = CallableTestObj::new_gd();
    let callable = obj.callable("foo");

    assert_eq!(obj.bind().value, 0);
    callable.callv(varray![10]);
    assert_eq!(obj.bind().value, 10);

    // Too many arguments: this call fails, its logic is not applied.
    // In the future, panic should be propagated to caller.
    callable.callv(varray![20, 30]);
    assert_eq!(obj.bind().value, 10);

    // TODO(bromeon): this causes a Rust panic, but since call() is routed to Godot, the panic is handled at the FFI boundary.
    // Can there be a way to notify the caller about failed calls like that?
    assert_eq!(callable.callv(varray!["string"]), Variant::nil());

    assert_eq!(Callable::invalid().callv(varray![1, 2, 3]), Variant::nil());
}

#[itest]
fn callable_call_return() {
    let obj = CallableTestObj::new_gd();
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
    let cb = Callable::from_object_method(&obj, "set_position");
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

// Testing https://github.com/godot-rust/gdext/issues/410

#[derive(GodotClass)]
#[class(init, base = Node)]
pub struct CallableRefcountTest {}

#[godot_api]
impl CallableRefcountTest {
    #[func]
    fn accept_callable(&self, _call: Callable) {}
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Tests and infrastructure for custom callables

#[cfg(since_api = "4.2")]
mod custom_callable {
    use super::*;
    use crate::framework::assert_eq_self;
    use godot::builtin::Dictionary;
    use std::fmt;
    use std::hash::Hash;
    use std::sync::{Arc, Mutex};

    #[itest]
    fn callable_from_fn() {
        let callable = Callable::from_fn("sum", sum);

        assert!(callable.is_valid());
        assert!(!callable.is_null());
        assert!(callable.is_custom());
        assert!(callable.object().is_none());

        let sum1 = callable.callv(varray![1, 2, 4, 8]);
        assert_eq!(sum1, 15.to_variant());

        let sum2 = callable.callv(varray![5]);
        assert_eq!(sum2, 5.to_variant());

        // Important to test 0 arguments, as the FFI call passes a null pointer for the argument array.
        let sum3 = callable.callv(varray![]);
        assert_eq!(sum3, 0.to_variant());
    }

    #[itest]
    fn callable_from_fn_eq() {
        let a = Callable::from_fn("sum", sum);
        let b = a.clone();
        let c = Callable::from_fn("sum", sum);

        assert_eq!(a, b, "same function, same instance -> equal");
        assert_ne!(a, c, "same function, different instance -> not equal");
    }

    fn sum(args: &[&Variant]) -> Result<Variant, ()> {
        let sum: i32 = args.iter().map(|arg| arg.to::<i32>()).sum();
        Ok(sum.to_variant())
    }

    #[itest]
    fn callable_custom_invoke() {
        let my_rust_callable = Adder::new(0);
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
        let my_rust_callable = Adder::new(-2);
        let callable = Callable::from_custom(my_rust_callable);

        let variant = callable.to_variant();
        assert_eq!(variant.stringify(), GString::from("Adder(sum=-2)"));
    }

    #[itest]
    fn callable_custom_eq() {
        // Godot only invokes custom equality function if the operands are not the same instance of the Callable.

        let at = Tracker::new();
        let bt = Tracker::new();
        let ct = Tracker::new();

        let a = Callable::from_custom(Adder::new_tracked(3, at.clone()));
        let b = Callable::from_custom(Adder::new_tracked(3, bt.clone()));
        let c = Callable::from_custom(Adder::new_tracked(4, ct.clone()));

        assert_eq_self!(a);
        assert_eq!(
            eq_count(&at),
            0,
            "if it's the same Callable, Godot does not invoke custom eq"
        );

        assert_eq!(a, b);
        assert_eq!(eq_count(&at), 1);
        assert_eq!(eq_count(&bt), 1);

        assert_ne!(a, c);
        assert_eq!(eq_count(&at), 2);
        assert_eq!(eq_count(&ct), 1);

        assert_eq!(a.to_variant(), b.to_variant(), "equality inside Variant");
        assert_eq!(eq_count(&at), 3);
        assert_eq!(eq_count(&bt), 2);

        assert_ne!(a.to_variant(), c.to_variant(), "inequality inside Variant");
        assert_eq!(eq_count(&at), 4);
        assert_eq!(eq_count(&ct), 2);
    }

    #[itest]
    fn callable_custom_eq_hash() {
        // Godot only invokes custom equality function if the operands are not the same instance of the Callable.

        let at = Tracker::new();
        let bt = Tracker::new();

        let a = Callable::from_custom(Adder::new_tracked(3, at.clone()));
        let b = Callable::from_custom(Adder::new_tracked(3, bt.clone()));

        let mut dict = Dictionary::new();

        dict.set(a, "hello");
        assert_eq!(hash_count(&at), 1, "hash needed for a dict key");
        assert_eq!(eq_count(&at), 0, "eq not needed if dict bucket is empty");

        dict.set(b, "hi");
        assert_eq!(hash_count(&at), 1, "hash for a untouched if b is inserted");
        assert_eq!(hash_count(&bt), 1, "hash needed for b dict key");
        assert_eq!(eq_count(&at), 1, "hash collision, eq for a needed");
        assert_eq!(eq_count(&bt), 1, "hash collision, eq for b needed");
    }

    struct Adder {
        sum: i32,

        // Track usage of PartialEq and Hash
        tracker: Arc<Mutex<Tracker>>,
    }

    impl Adder {
        fn new(sum: i32) -> Self {
            Self {
                sum,
                tracker: Tracker::new(),
            }
        }

        fn new_tracked(sum: i32, tracker: Arc<Mutex<Tracker>>) -> Self {
            Self { sum, tracker }
        }
    }

    impl PartialEq for Adder {
        fn eq(&self, other: &Self) -> bool {
            let mut guard = self.tracker.lock().unwrap();
            guard.eq_counter += 1;

            let mut guard = other.tracker.lock().unwrap();
            guard.eq_counter += 1;

            self.sum == other.sum
        }
    }

    impl Hash for Adder {
        fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
            let mut guard = self.tracker.lock().unwrap();
            guard.hash_counter += 1;

            self.sum.hash(state);
        }
    }

    impl fmt::Display for Adder {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "Adder(sum={})", self.sum)
        }
    }

    impl godot::builtin::RustCallable for Adder {
        fn invoke(&mut self, args: &[&Variant]) -> Result<Variant, ()> {
            for arg in args {
                self.sum += arg.to::<i32>();
            }

            Ok(self.sum.to_variant())
        }
    }

    struct Tracker {
        eq_counter: usize,
        hash_counter: usize,
    }

    impl Tracker {
        fn new() -> Arc<Mutex<Self>> {
            Arc::new(Mutex::new(Self {
                eq_counter: 0,
                hash_counter: 0,
            }))
        }
    }

    fn eq_count(tracker: &Arc<Mutex<Tracker>>) -> usize {
        tracker.lock().unwrap().eq_counter
    }

    fn hash_count(tracker: &Arc<Mutex<Tracker>>) -> usize {
        tracker.lock().unwrap().hash_counter
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
