/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::hash::Hasher;
use std::sync::atomic::{AtomicU32, Ordering};

use godot::builtin::{
    array, varray, vdict, vslice, Array, Callable, Color, GString, NodePath, StringName, Variant,
    VariantArray, Vector2,
};
use godot::classes::{Node2D, Object, RefCounted};
use godot::init::GdextBuild;
use godot::meta::ToGodot;
use godot::obj::{Gd, NewAlloc, NewGd};
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
    fn stringify_int(&self, int: i32) -> GString {
        int.to_variant().stringify()
    }

    #[func]
    fn assign_int(&mut self, int: i32) {
        self.value = int;
    }

    #[func] // static
    fn concat_array(a: i32, b: GString, c: Array<NodePath>, d: Gd<RefCounted>) -> VariantArray {
        varray![a, b, c, d]
    }
}

#[itest]
fn callable_validity() {
    let obj = CallableTestObj::new_gd();

    // Non-null object, valid method.
    assert!(obj.callable("assign_int").is_valid());
    assert!(!obj.callable("assign_int").is_null());
    assert!(!obj.callable("assign_int").is_custom());
    assert!(obj.callable("assign_int").object().is_some());

    // Non-null object, invalid method.
    assert!(!obj.callable("doesnt_exist").is_valid());
    assert!(!obj.callable("doesnt_exist").is_null());
    assert!(!obj.callable("doesnt_exist").is_custom());
    assert!(obj.callable("doesnt_exist").object().is_some());

    // Null object.
    assert!(!Callable::invalid().is_valid());
    assert!(Callable::invalid().is_null());
    assert!(!Callable::invalid().is_custom());
    assert_eq!(Callable::invalid().object(), None);
    assert_eq!(Callable::invalid().object_id(), None);
    assert_eq!(Callable::invalid().method_name(), None);
}

#[itest]
fn callable_hash() {
    let obj = CallableTestObj::new_gd();
    assert_eq!(
        obj.callable("assign_int").hash_u32(),
        obj.callable("assign_int").hash_u32()
    );

    // Not guaranteed, but unlikely.
    assert_ne!(
        obj.callable("assign_int").hash_u32(),
        obj.callable("stringify_int").hash_u32()
    );
}

#[itest]
fn callable_object_method() {
    let object = CallableTestObj::new_gd();
    let object_id = object.instance_id();
    let callable = object.callable("assign_int");

    assert_eq!(callable.object(), Some(object.clone().upcast::<Object>()));
    assert_eq!(callable.object_id(), Some(object_id));
    assert_eq!(callable.method_name(), Some("assign_int".into()));

    // Invalidating the object still returns the old ID, however not the object.
    drop(object);
    assert_eq!(callable.object_id(), Some(object_id));
    assert_eq!(callable.object(), None);
}

#[itest]
#[cfg(since_api = "4.3")]
fn callable_variant_method() {
    // Dictionary
    let dict = vdict! { "one": 1, "value": 2 };
    let dict_get = Callable::from_variant_method(&dict.to_variant(), "get");
    assert_eq!(dict_get.call(vslice!["one"]), 1.to_variant());

    // GString
    let string = GString::from("some string").to_variant();
    let string_md5 = Callable::from_variant_method(&string, "md5_text");
    assert_eq!(
        string_md5.call(vslice![]), // use vslice![] as alternative &[] syntax.
        "5ac749fbeec93607fc28d666be85e73a".to_variant()
    );

    // Object
    let obj = CallableTestObj::new_gd().to_variant();
    let obj_stringify = Callable::from_variant_method(&obj, "stringify_int");
    assert_eq!(obj_stringify.call(vslice![10]), "10".to_variant());

    // Vector3
    let vector = Vector2::new(-1.2, 2.5).to_variant();
    let vector_round = Callable::from_variant_method(&vector, "round");
    assert_eq!(vector_round.call(&[]), Vector2::new(-1.0, 3.0).to_variant());

    // Color
    let color = Color::from_rgba8(255, 0, 127, 255).to_variant();
    let color_to_html = Callable::from_variant_method(&color, "to_html");
    assert_eq!(color_to_html.call(&[]), "ff007fff".to_variant());

    // Color - invalid method.
    let color = Color::from_rgba8(255, 0, 127, 255).to_variant();
    let color_to_html = Callable::from_variant_method(&color, "to_htmI");
    assert!(!color_to_html.is_valid());
}

#[itest]
#[cfg(since_api = "4.4")]
fn callable_static() {
    let callable = Callable::from_class_static("CallableTestObj", "concat_array");

    assert_eq!(callable.object(), None);
    assert_eq!(callable.object_id(), None);
    assert_eq!(callable.method_name(), None);
    assert!(callable.is_custom());
    assert!(callable.is_valid());

    assert!(!callable.is_null());

    // Calling works consistently everywhere.
    let result = callable.callv(&varray![
        10,
        "hello",
        &array![&NodePath::from("my/node/path")],
        RefCounted::new_gd()
    ]);

    let result = result.to::<VariantArray>();
    assert_eq!(result.len(), 4);
    assert_eq!(result.at(0), 10.to_variant());

    #[cfg(since_api = "4.3")]
    assert_eq!(callable.get_argument_count(), 0); // Consistently doesn't work :)
}

// Regression test, see https://github.com/godot-rust/gdext/pull/1029.
#[itest]
#[cfg(since_api = "4.4")]
fn callable_static_bind() {
    let callable = Callable::from_class_static("CallableTestObj", "concat_array");
    assert!(callable.is_valid());

    // Test varying binds to static callables.
    // Last 3 of 4 arguments. Within Godot, bound arguments are used in-order AFTER call arguments.
    let bindv = callable.bindv(&varray![
        "two",
        array![&NodePath::from("three/four")],
        &RefCounted::new_gd(),
    ]);
    assert!(bindv.is_valid());

    assert!(!bindv.to_variant().is_nil());
    let args = varray![1];
    let bindv_result = bindv.callv(&args);

    assert!(!bindv_result.is_nil());

    let bind_result_data: VariantArray = bindv_result.to();
    assert_eq!(4, bind_result_data.len());
}

#[itest]
fn callable_callv() {
    let obj = CallableTestObj::new_gd();
    let callable = obj.callable("assign_int");

    assert_eq!(obj.bind().value, 0);
    callable.callv(&varray![10]);
    assert_eq!(obj.bind().value, 10);

    // Too many arguments: this call fails, its logic is not applied.
    // In the future, panic should be propagated to caller.
    callable.callv(&varray![20, 30]);
    assert_eq!(obj.bind().value, 10);

    // TODO(bromeon): this causes a Rust panic, but since call() is routed to Godot, the panic is handled at the FFI boundary.
    // Can there be a way to notify the caller about failed calls like that?
    assert_eq!(callable.callv(&varray!["string"]), Variant::nil());

    assert_eq!(Callable::invalid().callv(&varray![1, 2, 3]), Variant::nil());
}

#[itest]
fn callable_call() {
    // See callable_callv() for future improvements.

    let obj = CallableTestObj::new_gd();
    let callable = obj.callable("assign_int");

    assert_eq!(obj.bind().value, 0);
    callable.call(vslice![10]);
    assert_eq!(obj.bind().value, 10);

    callable.call(vslice![20, 30]);
    assert_eq!(obj.bind().value, 10);

    assert_eq!(callable.call(vslice!["string"]), Variant::nil());

    assert_eq!(Callable::invalid().call(vslice![1, 2, 3]), Variant::nil());
}

#[itest]
fn callable_call_return() {
    let obj = CallableTestObj::new_gd();
    let callable = obj.callable("stringify_int");

    assert_eq!(
        callable.callv(&varray![10]),
        10.to_variant().stringify().to_variant()
    );

    // Causes error in Godot, but should not crash.
    assert_eq!(callable.callv(&varray!["string"]), Variant::nil());
}

#[itest]
fn callable_call_engine() {
    let obj = Node2D::new_alloc();
    let cb = Callable::from_object_method(&obj, "set_position");

    assert!(!cb.is_null());
    assert_eq!(cb.object_id(), Some(obj.instance_id()));
    assert_eq!(cb.method_name(), Some(StringName::from("set_position")));

    let pos = Vector2::new(5.0, 7.0);
    cb.call(vslice![pos]);
    assert_eq!(obj.get_position(), pos);

    let pos = Vector2::new(1.0, 23.0);
    let bound = cb.bind(vslice![pos]);
    bound.call(&[]);
    assert_eq!(obj.get_position(), pos);

    obj.free();
}

#[itest]
fn callable_bindv() {
    let obj = CallableTestObj::new_gd();
    let callable = obj.callable("stringify_int");
    let callable_bound = callable.bindv(&varray![10]);

    assert_eq!(
        callable_bound.callv(&varray![]),
        10.to_variant().stringify().to_variant()
    );
}

#[itest]
fn callable_bind() {
    let obj = CallableTestObj::new_gd();
    let callable = obj.callable("stringify_int");
    let callable_bound = callable.bind(vslice![10]);

    assert_eq!(
        callable_bound.call(&[]),
        10.to_variant().stringify().to_variant()
    );
}

#[itest]
fn callable_unbind() {
    let obj = CallableTestObj::new_gd();
    let callable = obj.callable("stringify_int");
    let callable_unbound = callable.unbind(3);

    assert_eq!(
        callable_unbound.call(vslice![121, 20, 30, 40]),
        121.to_variant().stringify().to_variant()
    );
}

#[cfg(since_api = "4.3")]
#[itest]
fn callable_get_argument_count() {
    let obj = CallableTestObj::new_gd();

    let assign_int = obj.callable("assign_int");
    assert_eq!(assign_int.get_argument_count(), 1);
    assert_eq!(assign_int.unbind(10).get_argument_count(), 11);

    assert_eq!(obj.callable("stringify_int").get_argument_count(), 1);

    let concat_array = obj.callable("concat_array");
    assert_eq!(concat_array.get_argument_count(), 4);
    assert_eq!(
        concat_array.bind(vslice![10, "hello"]).get_argument_count(),
        2
    );
}

#[itest]
fn callable_get_bound_arguments_count() {
    let obj = CallableTestObj::new_gd();
    let original = obj.callable("assign_int");

    assert_eq!(original.get_bound_arguments_count(), 0);
    assert_eq!(original.unbind(28).get_bound_arguments_count(), 0);

    let with_1_arg = original.bindv(&varray![10]);
    assert_eq!(with_1_arg.get_bound_arguments_count(), 1);

    // Note: bug regarding get_bound_arguments_count() before 4.4; godot-rust caps at 0.
    let expected = if GdextBuild::since_api("4.4") { 1 } else { 0 };
    assert_eq!(with_1_arg.unbind(5).get_bound_arguments_count(), expected);
}

#[itest]
fn callable_get_bound_arguments() {
    let obj = CallableTestObj::new_gd();

    let a: i32 = 10;
    let b: &str = "hello!";
    let c: Array<NodePath> = array!["my/node/path"];
    let d: Gd<RefCounted> = RefCounted::new_gd();

    let callable = obj.callable("baz");
    let callable_bound = callable.bindv(&varray![a, b, c, d]);

    assert_eq!(callable_bound.get_bound_arguments(), varray![a, b, c, d]);
}

// Regression test for https://github.com/godot-rust/gdext/issues/410.
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

// Used to be #[cfg(since_api = "4.2")], could maybe be moved to own file.
pub mod custom_callable {
    use std::fmt;
    use std::hash::Hash;
    use std::sync::{Arc, Mutex};

    use godot::builtin::{Dictionary, RustCallable};
    use godot::prelude::Signal;
    use godot::sys;
    use godot::sys::GdextBuild;

    use super::*;
    use crate::framework::{assert_eq_self, quick_thread, suppress_panic_log, ThreadCrosser};

    #[itest]
    fn callable_from_fn() {
        let callable = Callable::from_fn("sum", sum);

        assert!(callable.is_valid());
        assert!(!callable.is_null());
        assert!(callable.is_custom());
        assert!(callable.object().is_none());

        let sum1 = callable.callv(&varray![1, 2, 4, 8]);
        assert_eq!(sum1, 15.to_variant());

        // Important to test 0 arguments, as the FFI call passes a null pointer for the argument array.
        let sum2 = callable.callv(&varray![]);
        assert_eq!(sum2, 0.to_variant());
    }

    // Without this feature, any access to the global binding from another thread fails; so the from_fn() cannot be tested in isolation.
    #[itest]
    fn callable_from_fn_crossthread() {
        // This static is a workaround for not being able to propagate failed `Callable` invocations as panics.
        // See note in itest callable_call() for further info.
        static GLOBAL: sys::Global<i32> = sys::Global::default();

        let callable = Callable::from_fn("change_global", |_args| {
            *GLOBAL.lock() = 777;
        });

        // Note that Callable itself isn't Sync/Send, so we have to transfer it unsafely.
        // Godot may pass it to another thread though without `unsafe`.
        let crosser = ThreadCrosser::new(callable);

        // Create separate thread and ensure calling fails.
        // Why expect_panic for (safeguards_balanced && single-threaded) but not otherwise:
        // - In single-threaded mode with balanced safeguards, there's an FFI access check which panics when another thread is invoked.
        // - In multi-threaded mode OR with safeguards disengaged, the callable may or may not execute, but won't panic at the FFI level.
        // - We can't catch panics from Callable invocations yet (see above), only the FFI access panics.
        if cfg!(safeguards_balanced) && !cfg!(feature = "experimental-threads") {
            // Single-threaded with balanced safeguards: FFI access check will panic.
            crate::framework::expect_panic(
                "Callable created with from_fn() must panic when invoked on other thread",
                || {
                    quick_thread(|| {
                        let callable = unsafe { crosser.extract() };
                        callable.callv(&varray![5]);
                    });
                },
            );
        } else {
            // Multi-threaded OR safeguards disengaged: No FFI panic, but callable may or may not execute.
            quick_thread(|| {
                let callable = unsafe { crosser.extract() };
                callable.callv(&varray![5]);
            });
        }

        // Expected value depends on whether thread checks are enforced.
        // 777: callable *is* executed on other thread.
        let expected = if cfg!(safeguards_balanced) { 0 } else { 777 };
        assert_eq!(*GLOBAL.lock(), expected);
    }

    #[itest]
    #[cfg(feature = "experimental-threads")]
    fn callable_from_sync_fn() {
        let callable = Callable::from_sync_fn("sum", sum);

        assert!(callable.is_valid());
        assert!(!callable.is_null());
        assert!(callable.is_custom());
        assert!(callable.object().is_none());

        let sum1 = callable.callv(&varray![1, 2, 4, 8]);
        assert_eq!(sum1, 15.to_variant());

        let sum2 = callable.callv(&varray![5]);
        assert_eq!(sum2, 5.to_variant());

        // Important to test 0 arguments, as the FFI call passes a null pointer for the argument array.
        let sum3 = callable.callv(&varray![]);
        assert_eq!(sum3, 0.to_variant());
    }

    #[itest]
    fn callable_from_fn_nil() {
        let callable_with_err = Callable::from_fn("returns_nil", |_args: &[&Variant]| {});

        assert_eq!(callable_with_err.callv(&varray![]), Variant::nil());
    }

    #[itest]
    fn callable_from_fn_eq() {
        let a = Callable::from_fn("sum", sum);
        let b = a.clone();
        let c = Callable::from_fn("sum", sum);

        assert_eq!(a, b, "same function, same instance -> equal");
        assert_ne!(a, c, "same function, different instance -> not equal");
    }

    // Now non-Variant return type.
    fn sum(args: &[&Variant]) -> i32 {
        args.iter().map(|arg| arg.to::<i32>()).sum()
    }

    #[itest]
    fn callable_custom_invoke() {
        let my_rust_callable = Adder::new(0);
        let callable = Callable::from_custom(my_rust_callable);

        assert!(callable.is_valid());
        assert!(!callable.is_null());
        assert!(callable.is_custom());
        assert!(callable.object().is_none());

        let sum1 = callable.callv(&varray![3, 9, 2, 1]);
        assert_eq!(sum1, 15.to_variant());

        let sum2 = callable.callv(&varray![4]);
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

        let eq = match GdextBuild::godot_runtime_version_triple() {
            (4, 1..=3, _) => 1,
            (4, 4, 0..=1) => 2, // changed in https://github.com/godotengine/godot/pull/96797.
            _ => 1,             // changed in https://github.com/godotengine/godot/pull/103647.
        };

        assert_eq!(eq_count(&at), eq, "hash collision, eq for a needed");
        assert_eq!(eq_count(&bt), eq, "hash collision, eq for b needed");
    }

    #[itest]
    fn callable_callv_panic_from_fn() {
        let received = Arc::new(AtomicU32::new(0));
        let received_callable = received.clone();
        let callable = Callable::from_fn("test", move |_args| {
            suppress_panic_log(|| {
                panic!("TEST: {}", received_callable.fetch_add(1, Ordering::SeqCst))
            });
        });

        assert_eq!(Variant::nil(), callable.callv(&varray![]));

        assert_eq!(1, received.load(Ordering::SeqCst));
    }

    #[itest]
    fn callable_callv_panic_from_custom() {
        let received = Arc::new(AtomicU32::new(0));
        let callable = Callable::from_custom(PanicCallable(received.clone()));

        assert_eq!(Variant::nil(), callable.callv(&varray![]));

        assert_eq!(1, received.load(Ordering::SeqCst));
    }

    #[itest]
    fn callable_is_connected() {
        let tracker = Tracker::new();
        let tracker2 = Tracker::new();

        // Adder hash depends on its sum.
        let some_callable = Callable::from_custom(Adder::new_tracked(3, tracker));
        let identical_callable = Callable::from_custom(Adder::new_tracked(3, tracker2));

        let obj = RefCounted::new_gd();
        let signal = Signal::from_object_signal(&obj, "script_changed");
        signal.connect(&some_callable);

        // Given Custom Callable is connected to signal
        // if callable with the very same hash is already connected.
        assert!(signal.is_connected(&some_callable));
        assert!(signal.is_connected(&identical_callable));

        let change = [2.to_variant()];

        // Change the hash.
        signal.emit(&change);

        // The hash, dependent on `Adder.sum` has been changed.
        // `identical_callable` is considered NOT connected.
        assert!(signal.is_connected(&some_callable));
        assert!(!signal.is_connected(&identical_callable));

        identical_callable.call(&change);

        // The hashes are, once again, identical.
        assert!(signal.is_connected(&some_callable));
        assert!(signal.is_connected(&identical_callable));
    }

    #[itest]
    fn callable_from_once_fn() {
        let callable = Callable::__once_fn("once_test", move |_| 42.to_variant());

        // First call should succeed.
        let result = callable.call(&[]);
        assert_eq!(result.to::<i32>(), 42);

        // Second call should fail (panic currently isn't propagated, see other tests).
        let result = callable.call(&[]);
        assert!(result.is_nil());
    }

    // ------------------------------------------------------------------------------------------------------------------------------------------
    // Helper structs and functions for custom callables

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
        fn hash<H: Hasher>(&self, state: &mut H) {
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

    impl RustCallable for Adder {
        fn invoke(&mut self, args: &[&Variant]) -> Variant {
            for arg in args {
                self.sum += arg.to::<i32>();
            }

            self.sum.to_variant()
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

    // Also used in signal_test.
    pub struct PanicCallable(pub Arc<AtomicU32>);

    impl PartialEq for PanicCallable {
        fn eq(&self, other: &Self) -> bool {
            Arc::ptr_eq(&self.0, &other.0)
        }
    }

    impl Hash for PanicCallable {
        fn hash<H: Hasher>(&self, state: &mut H) {
            state.write_usize(Arc::as_ptr(&self.0) as usize)
        }
    }

    impl fmt::Display for PanicCallable {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "test")
        }
    }

    impl RustCallable for PanicCallable {
        fn invoke(&mut self, _args: &[&Variant]) -> Variant {
            panic!("TEST: {}", self.0.fetch_add(1, Ordering::SeqCst))
        }
    }
}
