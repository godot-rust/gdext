/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! Objects that are destroyed while a Godot -> Rust call on them is still running.
//!
//! The storage must outlive such a call, since the trampoline and any instance guard keep using it after the method returns.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use godot::builtin::vslice;
use godot::classes::notify::ObjectNotification;
use godot::classes::{IRefCounted, Object, RefCounted};
use godot::meta::ToGodot;
use godot::obj::{Base, Gd, InstanceId, NewAlloc, NewGd, WithBaseField, WithUserSignals};
use godot::register::{GodotClass, godot_api};

use crate::framework::{itest, suppress_godot_print};

// A signal handler may drop the last reference to the emitter, mid-call. The object must survive until the method returns, otherwise the
// active instance guard would dangle. Regression test for https://github.com/godot-rust/gdext/issues/1666.
#[itest]
fn signal_emitter_destroyed_during_own_call() {
    let instance_id = call_self_destroyer("emit_and_die");

    assert!(!instance_id.lookup_validity(), "object must be destroyed");
    assert_eq!(SELF_DESTROYER_DROPS.get(), 1, "storage must not leak");
}

// Same, but the destruction happens one call deeper, so two claims are outstanding and only the outermost may free.
#[itest]
fn signal_emitter_destroyed_during_nested_call() {
    let instance_id = call_self_destroyer("nested_emit_and_die");

    assert!(!instance_id.lookup_validity());
    assert_eq!(SELF_DESTROYER_DROPS.get(), 1);
}

// The workaround named in the destruction report: a deferred callable holds a reference past the call.
#[itest]
fn signal_emitter_kept_alive_by_deferred_call() {
    let instance_id = call_self_destroyer("emit_and_survive");

    assert!(
        instance_id.lookup_validity(),
        "object must survive the call"
    );
}

// `gd_self` holds no instance guard, so free() succeeds and destroys the object mid-call. The storage must outlive the call, whose trampoline
// still uses it after the method returns. `&mut self` receivers cannot reach this: free() panics on an active bind.
// The destruction is reported as an error, which is expected here.
#[itest]
fn object_free_during_own_gd_self_call() {
    SELF_FREER_DROPS.set(0);

    let mut object = SelfFreer::new_alloc();
    let id = object.instance_id();

    let result = suppress_godot_print(|| object.call("free_self", &[]));
    assert_eq!(result, 1.to_variant());
    assert!(!id.lookup_validity());
    assert_eq!(SELF_FREER_DROPS.get(), 1, "storage must not leak");
}

// Like `signal_emitter_destroyed_during_own_call`, but through the notification callback. The reference dropped mid-call is not the last one:
// Godot reads `this` after the callback returns, so the object must outlive it -- see the keepalive below.
#[itest]
fn notification_drops_own_reference() {
    DESTROYER_DROPS.set(0);

    let object = NotificationSelfDestroyer::new_gd();
    let instance_id = object.instance_id();

    // Callable holds no strong reference, so the handler's reference and this one are the only two.
    let callable = object.callable("notification");
    DESTROYER_REF.with(|cell| *cell.borrow_mut() = Some(object.clone()));

    // Keepalive: Godot reads `this` after the callback returns (Object::_notification_forward()), so the last reference must not drop here.
    let keepalive = object;

    // Arbitrary user notification ID; the handler ignores it.
    callable.call(vslice![9999]);

    assert!(DESTROYER_REF.with(|cell| cell.borrow().is_none()));
    assert!(instance_id.lookup_validity(), "keepalive outlives the call");

    drop(keepalive);
    assert!(!instance_id.lookup_validity());
    assert_eq!(DESTROYER_DROPS.get(), 1, "storage must not leak");
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Helper types

thread_local! {
    // The `*_DROPS` counters are incremented per destroyed storage, to detect a claimed storage that is never freed.
    static SELF_DESTROYER_DROPS: Cell<u32> = const { Cell::new(0) };
    static SELF_FREER_DROPS: Cell<u32> = const { Cell::new(0) };
    static DESTROYER_DROPS: Cell<u32> = const { Cell::new(0) };

    /// Lets the notification handler drop a reference to its own object, without access to the test's local variables.
    static DESTROYER_REF: RefCell<Option<Gd<NotificationSelfDestroyer>>> = const { RefCell::new(None) };
}

/// Invokes `method` on a fresh [`SelfDestroyer`], whose signal handler drops the last reference to it mid-call.
fn call_self_destroyer(method: &str) -> InstanceId {
    SELF_DESTROYER_DROPS.set(0);

    let object = SelfDestroyer::new_gd();
    let instance_id = object.instance_id();

    // Callable holds no strong reference, so the only one remains inside `last_ref`.
    let callable = object.callable(method);
    let last_ref = Rc::new(RefCell::new(None));

    let cell = last_ref.clone();
    object.signals().about_to_die().connect(move || {
        *cell.borrow_mut() = None; // Drops the last reference to the emitter, mid-call.
    });

    *last_ref.borrow_mut() = Some(object);

    // Goes through the Godot -> Rust method trampoline, which must keep the receiver alive for the duration of the call.
    // The destruction is reported as an error, which is expected here.
    suppress_godot_print(|| callable.call(&[]));
    assert!(last_ref.borrow().is_none(), "handler must have run");

    instance_id
}

/// Emits a signal whose handler drops the last reference to this very object.
#[derive(GodotClass)]
#[class(base = RefCounted, init)]
struct SelfDestroyer {
    base: Base<RefCounted>,
}

impl Drop for SelfDestroyer {
    fn drop(&mut self) {
        SELF_DESTROYER_DROPS.set(SELF_DESTROYER_DROPS.get() + 1);
    }
}

#[godot_api]
impl SelfDestroyer {
    #[signal]
    fn about_to_die();

    #[func]
    fn emit_and_die(&mut self) {
        self.signals().about_to_die().emit();
    }

    #[func]
    fn emit_and_survive(&mut self) {
        self.to_gd().run_deferred_gd(|_| {}); // Keeps a reference until idle time.
        self.signals().about_to_die().emit();
    }

    /// Re-enters through Godot, so two calls are ongoing. Uses `&self` receivers, since a nested `&mut self` bind would conflict.
    #[func]
    fn nested_emit_and_die(&self) {
        // Callable holds no strong reference, so the object can die inside the inner frame.
        let callable = self.to_gd().callable("emit_shared");
        callable.call(&[]);
    }

    #[func]
    fn emit_shared(&self) {
        // Engine-side emit, which needs no bind of the user instance.
        self.to_gd().emit_signal("about_to_die", &[]);
    }
}

/// Frees itself from within a call on it, which its `gd_self` receiver permits.
#[derive(GodotClass)]
#[class(base = Object, init)]
struct SelfFreer {
    base: Base<Object>,
}

impl Drop for SelfFreer {
    fn drop(&mut self) {
        SELF_FREER_DROPS.set(SELF_FREER_DROPS.get() + 1);
    }
}

#[godot_api]
impl SelfFreer {
    #[func(gd_self)]
    fn free_self(this: Gd<Self>) -> i32 {
        this.free();
        1
    }
}

/// Drops a reference to itself when notified.
#[derive(GodotClass)]
#[class(base = RefCounted, init)]
struct NotificationSelfDestroyer {}

impl Drop for NotificationSelfDestroyer {
    fn drop(&mut self) {
        DESTROYER_DROPS.set(DESTROYER_DROPS.get() + 1);
    }
}

#[godot_api]
impl IRefCounted for NotificationSelfDestroyer {
    fn on_notification(&mut self, _what: ObjectNotification) {
        // Drops a reference to this object, mid-call. Still empty during POSTINITIALIZE.
        DESTROYER_REF.with(|cell| *cell.borrow_mut() = None);
    }
}
