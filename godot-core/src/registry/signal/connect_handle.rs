/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::borrow::Cow;

use crate::builtin::Callable;
use crate::classes::Object;
use crate::obj::Gd;
use crate::sys;

/// Handle representing a typed signal connection to a receiver.
///
/// Returned by connections made by the `connect_*` methods of
/// [`TypedSignal`][crate::registry::signal::TypedSignal] and [`ConnectBuilder`][crate::registry::signal::ConnectBuilder].
///
/// Connections managed by a handle can be disconnected using [`disconnect()`][Self::disconnect].
pub struct ConnectHandle {
    receiver_object: Gd<Object>,
    signal_name: Cow<'static, str>,
    callable: Callable,
}

impl ConnectHandle {
    // Should only be invoked by connect_* methods.
    pub(super) fn new(
        receiver_object: Gd<Object>,
        signal_name: Cow<'static, str>,
        callable: Callable,
    ) -> Self {
        Self {
            receiver_object,
            signal_name,
            callable,
        }
    }

    /// Disconnects the signal from the connected callable.
    ///
    /// # Panics (safeguards-balanced)
    /// If the connection does not exist. Use [`is_connected()`][Self::is_connected] to make sure the connection exists.
    pub fn disconnect(mut self) {
        sys::balanced_assert!(self.is_connected());

        self.receiver_object
            .disconnect(&*self.signal_name, &self.callable);
    }

    /// Whether the handle represents a valid connection.
    ///
    /// Returns false if:
    /// - ... the signals and callables managed by this handle have been disconnected in any other way than by using
    ///   [`disconnect()`][Self::disconnect] -- e.g. through [`Signal::disconnect()`][crate::builtin::Signal::disconnect] or
    ///   [`Object::disconnect()`][crate::classes::Object::disconnect].
    /// - ... the broadcasting object managed by this handle is not valid -- e.g. if the object has been freed.
    pub fn is_connected(&self) -> bool {
        self.receiver_object.is_instance_valid()
            && self
                .receiver_object
                .is_connected(&*self.signal_name, &self.callable)
    }
}
