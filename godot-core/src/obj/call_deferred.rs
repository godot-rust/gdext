/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
use crate::builtin::{Callable, Variant};
use crate::meta::UniformObjectDeref;
use crate::obj::bounds::Declarer;
use crate::obj::GodotClass;
#[cfg(since_api = "4.2")]
use crate::registry::signal::ToSignalObj;
use godot_ffi::is_main_thread;
use std::ops::DerefMut;

// Dummy traits to still allow bounds and imports.
#[cfg(before_api = "4.2")]
pub trait WithDeferredCall<T: GodotClass> {}

/// Trait that is automatically implemented for engine classes and  user classes containing a `Base<T>` field.
///
/// This trait enables type safe deferred method calls.
///
/// # Usage
///
/// ```no_compile
/// # use godot::prelude::*;
/// # use godot::classes::CollisionShape2D;
/// # use std::f32::consts::PI;
/// fn some_fn(mut node: Gd<Node2D>)
/// {
///     node.apply_deferred(|shape_mut| shape_mut.rotate(PI))
/// }
/// ```
#[cfg(since_api = "4.2")]
pub trait WithDeferredCall<T: GodotClass> {
    /// Runs the given Closure deferred.
    ///
    /// This can be a type-safe alternative to [`Object::call_deferred`][crate::classes::Object::call_deferred]. This method must be used on the main thread.
    fn apply_deferred<F>(&mut self, rust_function: F)
    where
        F: FnMut(&mut T) + 'static;
}

#[cfg(since_api = "4.2")]
impl<T, S, D> WithDeferredCall<T> for S
where
    T: UniformObjectDeref<D, Declarer = D>,
    S: ToSignalObj<T>,
    D: Declarer,
{
    fn apply_deferred<'a, F>(&mut self, mut rust_function: F)
    where
        F: FnMut(&mut T) + 'static,
    {
        assert!(
            is_main_thread(),
            "`apply_deferred` must be called on the main thread"
        );
        let mut this = self.to_signal_obj().clone();
        let callable = Callable::from_local_fn("apply_deferred", move |_| {
            rust_function(T::object_as_mut(&mut this).deref_mut());
            Ok(Variant::nil())
        });
        callable.call_deferred(&[]);
    }
}
