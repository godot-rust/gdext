/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::ops::DerefMut;

use godot_ffi::is_main_thread;

use crate::builtin::{Callable, Variant};
use crate::meta::UniformObjectDeref;
use crate::obj::bounds::Declarer;
use crate::obj::GodotClass;
#[cfg(since_api = "4.2")]
use crate::registry::signal::ToSignalObj;

// Dummy traits to still allow bounds and imports.
#[cfg(before_api = "4.2")]
pub trait WithDeferredCall<T: GodotClass> {}

// TODO(v0.4): seal this and similar traits.

/// Enables `Gd::apply_deferred()` for type-safe deferred calls.
///
/// The trait is automatically available for all engine-defined Godot classes and user classes containing a `Base<T>` field.
///
/// # Usage
///
/// ```no_run
/// # use godot::prelude::*;
/// # use std::f32::consts::PI;
/// fn some_fn(mut node: Gd<Node2D>) {
///     node.apply_deferred(|n: &mut Node2D| n.rotate(PI))
/// }
/// ```
#[cfg(since_api = "4.2")]
pub trait WithDeferredCall<T: GodotClass> {
    /// Defers the given closure to run during [idle time](https://docs.godotengine.org/en/stable/classes/class_object.html#class-object-method-call-deferred).
    ///
    /// This is a type-safe alternative to [`Object::call_deferred()`][crate::classes::Object::call_deferred].
    ///
    /// # Panics
    /// If called outside the main thread.
    fn apply_deferred<F>(&mut self, rust_function: F)
    where
        F: FnOnce(&mut T) + 'static;
}

#[cfg(since_api = "4.2")]
impl<T, S, D> WithDeferredCall<T> for S
where
    T: UniformObjectDeref<D, Declarer = D>,
    S: ToSignalObj<T>,
    D: Declarer,
{
    fn apply_deferred<'a, F>(&mut self, rust_function: F)
    where
        F: FnOnce(&mut T) + 'static,
    {
        assert!(
            is_main_thread(),
            "`apply_deferred` must be called on the main thread"
        );

        let mut this = self.to_signal_obj().clone();
        let callable = Callable::from_once_fn("apply_deferred", move |_| {
            let mut this_mut = T::object_as_mut(&mut this);
            rust_function(this_mut.deref_mut());
            Ok(Variant::nil())
        });
        callable.call_deferred(&[]);
    }
}
