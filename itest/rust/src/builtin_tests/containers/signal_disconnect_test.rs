/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use godot::builtin::{Callable, Signal};
use godot::classes::Object;
use godot::obj::{Base, Gd, NewAlloc};
use godot::register::{godot_api, ConnectHandle, GodotClass};

use crate::framework::{expect_debug_panic_or_release_ok, expect_panic, itest};

#[derive(GodotClass)]
#[class(init, base=Object)]
struct SignalDisc {
    counter: i32,

    _base: Base<Object>,
}

#[godot_api]
impl SignalDisc {
    #[signal]
    fn my_signal();

    fn increment_self(&mut self) {
        self.counter += 1;
    }
}

#[itest]
fn disconnect_static() {
    let obj = SignalDisc::new_alloc();
    let mut closure_obj = obj.clone();
    let handle = obj.signals().my_signal().connect(move || {
        closure_obj.bind_mut().increment_self();
    });

    // Emit signal.
    assert_eq!(obj.bind().counter, 0);
    obj.signals().my_signal().emit();
    assert_eq!(obj.bind().counter, 1);

    handle.disconnect();

    // Emit again - this time no change should happen.
    obj.signals().my_signal().emit();
    assert_eq!(obj.bind().counter, 1);

    obj.free();
}

#[itest]
fn disconnect_self() {
    test_disconnect(true, |signal_object, _| {
        signal_object
            .signals()
            .my_signal()
            .connect_self(SignalDisc::increment_self)
    })
}

#[itest]
fn disconnect_self_mut() {
    test_disconnect(true, |signal_object, _| {
        signal_object
            .signals()
            .my_signal()
            .builder()
            .connect_self_mut(|self_mut| {
                self_mut.increment_self();
            })
    });
}

#[itest]
fn disconnect_self_gd() {
    test_disconnect(true, |signal_object, _| {
        signal_object
            .signals()
            .my_signal()
            .builder()
            .connect_self_gd(|mut self_gd| {
                self_gd.bind_mut().increment_self();
            })
    });
}

#[itest]
fn disconnect_other() {
    test_disconnect(false, |broadcaster, receiver| {
        broadcaster
            .signals()
            .my_signal()
            .connect_other(receiver, SignalDisc::increment_self)
    });
}

#[itest]
fn disconnect_other_mut() {
    test_disconnect(false, |broadcaster, receiver| {
        broadcaster
            .signals()
            .my_signal()
            .builder()
            .connect_other_mut(receiver, |other_mut| {
                other_mut.increment_self();
            })
    });
}

#[itest]
fn disconnect_other_gd() {
    test_disconnect(false, |broadcaster, receiver| {
        broadcaster
            .signals()
            .my_signal()
            .builder()
            .connect_other_gd(receiver, |mut other_gd| {
                other_gd.bind_mut().increment_self();
            })
    });
}

#[itest]
fn handle_recognizes_direct_signal_disconnect() {
    test_handle_recognizes_non_valid_state(|obj| {
        let signal_dict = obj.get_signal_connection_list("my_signal").at(0);
        let godot_signal = signal_dict.at("signal").to::<Signal>();
        let godot_callable = signal_dict.at("callable").to::<Callable>();

        // Disconnect using signal.disconnet(callable).
        assert!(godot_signal.is_connected(&godot_callable));
        godot_signal.disconnect(&godot_callable);
    });
}

#[itest]
fn handle_recognizes_direct_object_disconnect() {
    test_handle_recognizes_non_valid_state(|obj| {
        let signal_dict = obj.get_signal_connection_list("my_signal").at(0);
        let godot_callable = signal_dict.at("callable").to::<Callable>();

        // Disconnect using obj.disconnect(signal, callable)
        assert!(obj.is_connected("my_signal", &godot_callable));
        obj.disconnect("my_signal", &godot_callable);
    })
}

#[itest]
fn test_handle_after_freeing_broadcaster() {
    test_freed_nodes_handles(true);
}

#[itest]
fn test_handle_after_freeing_receiver() {
    test_freed_nodes_handles(false);
}

// Helper functions:

fn test_disconnect(
    connect_to_self: bool,
    handle_function: impl FnOnce(&Gd<SignalDisc>, &Gd<SignalDisc>) -> ConnectHandle,
) {
    // If we mean to connect to self, broadcaster and receiver is the same object.
    let broadcaster = SignalDisc::new_alloc();
    let receiver = if connect_to_self {
        broadcaster.clone()
    } else {
        SignalDisc::new_alloc()
    };

    // Connection handle created by the handle_function.
    let handle = handle_function(&broadcaster, &receiver);
    assert!(has_connections(&broadcaster));
    assert_eq!(broadcaster.bind().counter, 0);
    assert_eq!(receiver.bind().counter, 0);

    // Emit signal - either to self or another receiver.
    broadcaster.signals().my_signal().emit();
    assert_eq!(receiver.bind().counter, 1);

    // Disconnect.
    handle.disconnect();
    assert!(!has_connections(&broadcaster));

    // Emit signal again - this time with no effect as signal is not connected to anything.
    broadcaster.signals().my_signal().emit();
    assert_eq!(receiver.bind().counter, 1);

    // Broadcaster should have a non-zero counter iff it was connected to itself.
    // Otherwise, emitting the signal should not change broadcaster's inital value of 0.
    let expected_counter_for_broadcaster = if connect_to_self { 1 } else { 0 };
    assert_eq!(broadcaster.bind().counter, expected_counter_for_broadcaster);

    // Freeing both objects.
    broadcaster.free();
    // While free() prevents double-free, it will still panic when trying to free something that has already been freed.
    if !connect_to_self {
        receiver.free();
    }
}

fn test_handle_recognizes_non_valid_state(disconnect_function: impl FnOnce(&mut Gd<SignalDisc>)) {
    let mut obj = SignalDisc::new_alloc();

    let handle = obj
        .signals()
        .my_signal()
        .connect_self(SignalDisc::increment_self);

    // We do not need to emit here, but done just to demonstrate that it works.
    assert_eq!(obj.bind().counter, 0);
    obj.signals().my_signal().emit();
    assert_eq!(obj.bind().counter, 1);

    // Circumventing the handle and "disconnecting manually".
    disconnect_function(&mut obj);

    let is_valid = handle.is_connected();
    assert!(!is_valid);

    expect_debug_panic_or_release_ok("disconnect invalid handle", || {
        handle.disconnect();
    });

    obj.free();
}

fn test_freed_nodes_handles(free_broadcaster_first: bool) {
    let broadcaster = SignalDisc::new_alloc();
    let receiver = SignalDisc::new_alloc();

    let handle = broadcaster
        .signals()
        .my_signal()
        .connect_other(&receiver, |r| {
            r.increment_self();
        });

    let (to_free, other) = if free_broadcaster_first {
        (broadcaster, receiver)
    } else {
        (receiver, broadcaster)
    };

    // Free one of the nodes, and check if the handle thinks the objects are connected.
    // In both cases godot runtime should handle disconnecting the signals.
    to_free.free();
    assert!(!handle.is_connected());

    // Calling disconnect() on already disconnected handle should panic in the Debug mode.
    // Otherwise, in release mode, the error will happen in Godot runtime.
    if cfg!(debug_assertions) {
        expect_panic("Disconnected invalid handle!", || {
            handle.disconnect();
        });
    }

    other.free();
}

fn has_connections(obj: &Gd<SignalDisc>) -> bool {
    !obj.get_signal_connection_list("my_signal").is_empty()
}
