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
use crate::meta::{arg_into_ref, AsArg};
use crate::obj::{Gd, Inherits};

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
