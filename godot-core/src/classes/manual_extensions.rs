/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! Adds new convenience APIs to existing classes.
//!
//! This should not add new functionality, but provide existing one in a slightly nicer way to use. Generally, we should be conservative
//! about adding methods here, as it's a potentially endless quest, and many are better suited in high-level APIs or third-party crates.
//!
//! See also sister module [super::type_safe_replacements].

use crate::builtin::NodePath;
use crate::classes::{Node, PackedScene};
use crate::global::Error;
use crate::meta::{AsArg, arg_into_ref};
use crate::obj::{Gd, Inherits};
#[cfg(feature = "codegen-full")]
use crate::{builtin::GString, classes::GDExtensionManager, meta::ToGodot};

/// Manual extensions for the `Node` class.
impl Node {
    /// ⚠️ Retrieves the node at path `path`, panicking if not found or bad type.
    ///
    /// # Panics
    /// If the node is not found, or if it does not have type `T` or inherited.
    pub fn get_node_as<T>(&self, path: impl AsArg<NodePath>) -> Gd<T>
    where
        T: Inherits<Node>,
    {
        arg_into_ref!(path);

        self.try_get_node_as(path).unwrap_or_else(|| {
            panic!(
                "There is no node of type {ty} at path `{path}`",
                ty = T::class_id()
            )
        })
    }

    /// Retrieves the node at path `path` (fallible).
    ///
    /// If the node is not found, or if it does not have type `T` or inherited,
    /// `None` will be returned.
    pub fn try_get_node_as<T>(&self, path: impl AsArg<NodePath>) -> Option<Gd<T>>
    where
        T: Inherits<Node>,
    {
        arg_into_ref!(path);

        // TODO differentiate errors (not found, bad type) with Result
        self.get_node_or_null(path)
            .and_then(|node| node.try_cast::<T>().ok())
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

/// Manual extensions for the `PackedScene` class.
impl PackedScene {
    /// ⚠️ Instantiates the scene as type `T`, panicking if not found or bad type.
    ///
    /// # Panics
    /// If the scene is not type `T` or inherited.
    pub fn instantiate_as<T>(&self) -> Gd<T>
    where
        T: Inherits<Node>,
    {
        self.try_instantiate_as::<T>()
            .unwrap_or_else(|| panic!("Failed to instantiate {to}", to = T::class_id()))
    }

    /// Instantiates the scene as type `T` (fallible).
    ///
    /// If the scene is not type `T` or inherited.
    pub fn try_instantiate_as<T>(&self) -> Option<Gd<T>>
    where
        T: Inherits<Node>,
    {
        self.instantiate().and_then(|gd| gd.try_cast::<T>().ok())
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

/// Manual extensions for the `global::Error` enum.
impl Error {
    /// Returns `Ok(())` for `Error::OK`, and `Err(self)` for any `Error::ERR_*`.
    ///
    /// This is a convenience method that may be used to convert this type into one that can be used with the try operator (`?`)
    /// for easy short circuiting of Godot `Error`s.
    ///
    /// ```
    /// use godot::global::Error;
    ///
    /// assert_eq!(Error::OK.into_result(), Ok(()));
    /// assert_eq!(Error::FAILED.into_result(), Err(Error::FAILED));
    /// ```
    pub fn into_result(self) -> Result<(), Self> {
        if self == Error::OK { Ok(()) } else { Err(self) }
    }

    /// Creates an `Error` from a `Result<(), Error>`.
    ///
    /// `Ok(())` becomes `Error::OK`, and `Err(e)` becomes `e` (even in the unusual case where `e == Error::OK`).
    ///
    /// ```
    /// use godot::global::Error;
    ///
    /// assert_eq!(Error::from_result(Ok(())), Error::OK);
    /// assert_eq!(Error::from_result(Err(Error::FAILED)), Error::FAILED);
    /// ```
    pub fn from_result(result: Result<(), Self>) -> Self {
        match result {
            Ok(()) => Error::OK,
            Err(e) => e,
        }
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

/// Manual extensions for the `GDExtensionManager` class.
///
/// Godot's `load_extension`/`unload_extension`/`reload_extension` are marked `unsafe` via codegen special-case: invoking them synchronously
/// while the calling extension is on the stack rug-pulls the running library, invalidating all callables created from it. See
/// [#1221](https://github.com/godot-rust/gdext/issues/1221) and [#1249](https://github.com/godot-rust/gdext/issues/1249).
///
/// The `*_deferred` variants below run at the end of the current frame instead, which is safe even for the calling extension itself.
#[cfg(feature = "codegen-full")]
impl GDExtensionManager {
    /// Schedules a `load_extension` call for the end of the current frame.
    pub fn load_extension_deferred(&mut self, path: impl AsArg<GString>) {
        self.defer_path_call("load_extension", path);
    }

    /// Schedules an `unload_extension` call for the end of the current frame.
    pub fn unload_extension_deferred(&mut self, path: impl AsArg<GString>) {
        self.defer_path_call("unload_extension", path);
    }

    /// Schedules a `reload_extension` call for the end of the current frame.
    pub fn reload_extension_deferred(&mut self, path: impl AsArg<GString>) {
        self.defer_path_call("reload_extension", path);
    }

    // Deferred calls go through Godot's reflection, so the method name cannot be checked at compile time; a Godot-side rename would surface
    // only at runtime. The same three names are listed in `special_cases::get_class_method_unsafe_docs()`.
    fn defer_path_call(&mut self, godot_method: &str, path: impl AsArg<GString>) {
        arg_into_ref!(path);
        self.call_deferred(godot_method, &[path.to_variant()]);
    }
}
