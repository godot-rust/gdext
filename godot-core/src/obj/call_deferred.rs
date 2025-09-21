/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::obj::{Gd, GodotClass, WithBaseField};

// TODO(v0.4): seal this and similar traits.

/// Enables deferred execution for user classes containing a `Base<T>` field.
///
/// This trait provides `run_deferred()` and `run_deferred_gd()` methods for user class instances.
/// For `Gd<T>` instances, use the inherent methods [`Gd::run_deferred()`] and [`Gd::run_deferred_gd()`] instead.
///
/// The trait is automatically available for user classes containing a `Base<T>` field.
///
/// # Usage
///
/// ```no_run
/// # use godot::prelude::*;
/// #
/// #[derive(GodotClass)]
/// #[class(init, base=Node2D)]
/// struct MyNode {
///     base: Base<Node2D>,
/// }
///
/// #[godot_api]
/// impl MyNode {
///     fn some_method(&mut self) {
///         self.run_deferred(|this: &mut MyNode| {
///             // Direct access to Rust struct.
///         });
///
///         self.run_deferred_gd(|gd: Gd<MyNode>| {
///             // Access to Gd. Needs bind/bind_mut for struct access.
///         });
///     }
/// }
/// ```
pub trait WithDeferredCall: GodotClass + WithBaseField {
    /// Defers the given closure to run during [idle time](https://docs.godotengine.org/en/stable/classes/class_object.html#class-object-method-call-deferred).
    ///
    /// This is a type-safe alternative to [`Object::call_deferred()`][crate::classes::Object::call_deferred].
    ///
    /// The closure receives `&mut Self` allowing direct access to Rust fields and methods.
    ///
    /// # Panics
    /// If called outside the main thread.
    fn run_deferred<F>(&mut self, mut_self_method: F)
    where
        F: FnOnce(&mut Self) + 'static;

    /// Defers the given closure to run during [idle time](https://docs.godotengine.org/en/stable/classes/class_object.html#class-object-method-call-deferred).
    ///
    /// This is a type-safe alternative to [`Object::call_deferred()`][crate::classes::Object::call_deferred].
    ///
    /// The closure receives `Gd<Self>` which can be used to call engine methods.
    ///
    /// # Panics
    /// If called outside the main thread.
    fn run_deferred_gd<F>(&mut self, gd_function: F)
    where
        F: FnOnce(Gd<Self>) + 'static;

    #[deprecated(
        since = "0.4.0",
        note = "Split into `run_deferred()` + `run_deferred_gd`"
    )]
    fn apply_deferred<F>(&mut self, rust_function: F)
    where
        F: FnOnce(&mut Self) + 'static,
    {
        self.run_deferred(rust_function)
    }
}

impl<T> WithDeferredCall for T
where
    T: WithBaseField,
{
    fn run_deferred<F>(&mut self, mut_self_method: F)
    where
        F: FnOnce(&mut Self) + 'static,
    {
        // We need to copy the Gd, because the lifetime of `&mut self` does not extend throughout the closure, which will only be called
        // deferred. It might even be freed in-between, causing panic on bind_mut().
        self.to_gd().run_deferred(mut_self_method)
    }

    fn run_deferred_gd<F>(&mut self, gd_function: F)
    where
        F: FnOnce(Gd<Self>) + 'static,
    {
        self.to_gd().run_deferred_gd(gd_function)
    }
}
