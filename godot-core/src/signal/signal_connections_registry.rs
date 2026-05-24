/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! Signal connection registry
//!
//! Interacting with custom callables (used by typed signals) after hot reload in any way is instant UB.
//! We prevent unsoundness by disconnecting all the signals before the hot reload.
//!
//! To achieve this, we store all connections in a global registry as long as the library remains loaded and given receiver object is alive.
//!
//! For upstream issue, see: <https://github.com/godotengine/godot/issues/105802>.

use std::cell::RefCell;

use crate::builtin::{Callable, StringName};
use crate::classes::Object;
use crate::obj::Gd;
use crate::{godot_warn, sys};

thread_local! {
    static SIGNAL_CONNECTIONS_REGISTRY: RefCell<Vec<CachedSignalConnection >> = RefCell::default();
}

struct CachedSignalConnection {
    // `Option`, so we can mark objects for removal by setting receiver to None.
    receiver_object: Option<Gd<Object>>,
    signal_name: String,
    callable: Callable,
}

/// Prunes stale connections to objects that are no longer valid.
///
/// This method does not check the validity of the signals themselves due to the overhead required.
/// Additionally, the number of connections to alive objects is finite -- unlike connections to freed objects,
/// which can accumulate to a critical mass simply by opening and closing tabs in the editor.
fn prune_stale_connections(registry: &mut Vec<CachedSignalConnection>) {
    registry.retain_mut(|connection| {
        if let Some(obj) = connection
            .receiver_object
            .take_if(|obj| !obj.is_instance_valid())
        {
            obj.drop_weak();
            false
        } else {
            true
        }
    });
}

/// Stores a custom-callable connection so it can be disconnected before hot reload, and prunes connections to invalid objects.
pub(crate) fn store_custom_callable_connection(
    receiver_object: &Gd<Object>,
    signal_name: &StringName,
    callable: &Callable,
) {
    // We register only callables created by *this* extension, identified through `is_rust_callable()` relying on `rust_callable_token()`.
    // Behavior per callable kind:
    //
    //   Callable kind                     | is_custom | is_rust_callable | stale on reload
    //   ----------------------------------+-----------+------------------+------------------------------------------------------------
    //   Rust callable                     |   yes     |     yes          |    yes  <- registered, disconnected before reload
    //   Rust callable + bind/bindv        |   yes     |     no           |    yes  <- MISSED, see below
    //   Rust callable, other extension    |   yes     |     no           |    no   <- other extension takes care of it
    //   Object method                     |   no      |     no           |    no
    //   Engine+bind, GDScript lambda, ... |   yes     |     no           |    no
    //
    // `is_rust_callable()` avoids touching connections we shouldn't (bound engine methods, other extensions' callables, GDScript lambdas
    // routed through our `connect`) -- all of which the broad `is_custom()` would wrongly disconnect on every reload.
    //
    // Limitation: `bind/bindv/unbind` re-wrap their argument engine-side, so a bound Rust callable is no longer detected as Rust callable.
    // However, it still becomes stale after reload (UAF). We accept this: binding a Rust callable is rare (closures are more powerful than
    // `bind`), and a GDScript lambda calling a Rust callable would suffer from the same anyway. If we auto-disconnected these, we'd lose
    // a lot Callables that would perfectly survive hot-reload. In other words, we trade many false positives for very few false negatives.

    if !sys::is_editor() || !callable.is_rust_callable() {
        return;
    }

    SIGNAL_CONNECTIONS_REGISTRY.with_borrow_mut(|connection_registry| {
        prune_stale_connections(connection_registry);

        // SAFETY: Given weak pointer to the Object is accessed only once in `prune_stored_signal_connections` or `prune_stale_connections`,
        // inaccessible outside this module, validated before use and properly disposed of by using `drop_weak`.
        let weak_object_ptr = unsafe { receiver_object.clone_weak() };
        connection_registry.push(CachedSignalConnection {
            receiver_object: Some(weak_object_ptr),
            signal_name: signal_name.to_string(),
            callable: callable.clone(),
        });
    });
}

/// Disconnects all the registered signals.
///
/// Should be run only once during initialization of the library on [`InitLevel::Editor`].
/// (Running it multiple times is safe, but has no effect.)
pub(crate) fn prune_stored_signal_connections() {
    SIGNAL_CONNECTIONS_REGISTRY.with_borrow_mut(|connection_registry| {
        if connection_registry.is_empty() {
            return;
        }

        godot_warn!(
            "godot-rust: TypedSignal connections are now auto-disconnected.\n\
            Custom callables used in signals would otherwise become invalid after hot-reload.\n\
            They must be recreated by listening to `ObjectNotification::EXTENSION_RELOADED`.\n\
            See: https://godot-rust.github.io/book/register/signals.html#signals-in-the-editor--hot-reload-interaction."
        );

        for connection in connection_registry.drain(..) {
            let CachedSignalConnection {
                receiver_object: Some(mut receiver_object),
                signal_name,
                callable,
            } = connection
            else {
                continue;
            };

            // Bail if object has been freed in a meanwhile -- Godot handled disconnecting by itself.
            if !receiver_object.is_instance_valid() {
                receiver_object.drop_weak();
                continue;
            }

            if receiver_object.is_connected(&*signal_name, &callable) {
                receiver_object.disconnect(&*signal_name, &callable);
            }

            receiver_object.drop_weak();
        }
    });
}
