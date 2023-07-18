/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! Godot engine classes and methods.

// Re-exports of generated symbols
use crate::builtin::{GodotString, NodePath};
use crate::obj::dom::EngineDomain;
use crate::obj::{Gd, GodotClass, Inherits};

pub use crate::gen::central::global;
pub use crate::gen::classes::*;
pub use crate::gen::utilities;

/// Support for Godot _native structures_.
///
/// Native structures are a niche API in Godot. These are low-level data types that are passed as pointers to/from the engine.
/// In Rust, they are represented as `#[repr(C)]` structs.
///
/// There is unfortunately not much official documentation available; you may need to look at Godot source code.
/// Most users will not need native structures, as they are very specialized.
pub mod native {
    pub use crate::gen::native::*;
}

/// Extension trait for convenience functions on `PackedScene`
pub trait PackedSceneExt {
    /// ⚠️ Instantiates the scene as type `T`, panicking if not found or bad type.
    ///
    /// # Panics
    /// If the scene is not type `T` or inherited.
    fn instantiate_as<T>(&self) -> Gd<T>
    where
        T: Inherits<Node>,
    {
        self.try_instantiate_as::<T>()
            .unwrap_or_else(|| panic!("Failed to instantiate {to}", to = T::CLASS_NAME))
    }

    /// Instantiates the scene as type `T` (fallible).
    ///
    /// If the scene is not type `T` or inherited.
    fn try_instantiate_as<T>(&self) -> Option<Gd<T>>
    where
        T: Inherits<Node>;
}

impl PackedSceneExt for PackedScene {
    fn try_instantiate_as<T>(&self) -> Option<Gd<T>>
    where
        T: Inherits<Node>,
    {
        self.instantiate().and_then(|gd| gd.try_cast::<T>())
    }
}

/// Extension trait with convenience functions for the node tree.
pub trait NodeExt {
    /// Retrieves the node at path `path`, panicking if not found or bad type.
    ///
    /// # Panics
    /// If the node is not found, or if it does not have type `T` or inherited.
    fn get_node_as<T>(&self, path: impl Into<NodePath>) -> Gd<T>
    where
        T: GodotClass + Inherits<Node>,
    {
        let path = path.into();
        let copy = path.clone(); // TODO avoid copy

        self.try_get_node_as(path).unwrap_or_else(|| {
            panic!(
                "There is no node of type {ty} path `{copy}`",
                ty = T::CLASS_NAME
            )
        })
    }

    /// Retrieves the node at path `path` (fallible).
    ///
    /// If the node is not found, or if it does not have type `T` or inherited,
    /// `None` will be returned.
    fn try_get_node_as<T>(&self, path: impl Into<NodePath>) -> Option<Gd<T>>
    where
        T: GodotClass + Inherits<Node>;
}

impl NodeExt for Node {
    fn try_get_node_as<T>(&self, path: impl Into<NodePath>) -> Option<Gd<T>>
    where
        T: GodotClass + Inherits<Node>,
    {
        let path = path.into();

        // TODO differentiate errors (not found, bad type) with Result
        self.get_node_or_null(path)
            .and_then(|node| node.try_cast::<T>())
    }
}

impl<U> NodeExt for Gd<U>
where
    U: GodotClass<Declarer = EngineDomain> + Inherits<Node>,
{
    fn try_get_node_as<T>(&self, path: impl Into<NodePath>) -> Option<Gd<T>>
    where
        T: GodotClass + Inherits<Node>,
    {
        // TODO this could be implemented without share(), but currently lacks the proper bounds
        // This would need more sophisticated upcast design, e.g. T::upcast_{ref|mut}::<U>() for indirect relations
        // to make the indirect Deref more explicit

        use crate::obj::Share;

        let path = path.into();
        let node = self.share().upcast::<Node>();

        <Node as NodeExt>::try_get_node_as(&*node, path)
    }
}

/// Loads a resource from the filesystem located at `path`, panicking on error.
///
/// See [`try_load`] for more information.
///
/// # Example
///
/// ```no_run
/// use godot::prelude::*;
///
/// let scene = load::<PackedScene>("res://path/to/Main.tscn");
/// ```
///
/// # Panics
/// If the resource cannot be loaded, or is not of type `T` or inherited.
#[inline]
pub fn load<T>(path: impl Into<GodotString>) -> Gd<T>
where
    T: GodotClass + Inherits<Resource>,
{
    let path = path.into();
    load_impl(&path).unwrap_or_else(|| panic!("failed to load node at path `{path}`"))
}

/// Loads a resource from the filesystem located at `path`.
///
/// The resource is loaded on the method call (unless it's referenced already elsewhere, e.g. in another script or in the scene),
/// which might cause slight delay, especially when loading scenes.
///
/// If the resource cannot be loaded, or is not of type `T` or inherited, this method returns `None`.
///
/// This method is a simplified version of [`ResourceLoader::load()`][crate::api::ResourceLoader::load],
/// which can be used for more advanced scenarios.
///
/// # Note:
/// Resource paths can be obtained by right-clicking on a resource in the Godot editor (_FileSystem_ dock) and choosing "Copy Path",
/// or by dragging the file from the _FileSystem_ dock into the script.
///
/// The path must be absolute (typically starting with `res://`), a local path will fail.
///
/// # Example:
/// Loads a scene called `Main` located in the `path/to` subdirectory of the Godot project and caches it in a variable.
/// The resource is directly stored with type `PackedScene`.
///
/// ```no_run
/// use godot::prelude::*;
///
/// if let Some(scene) = try_load::<PackedScene>("res://path/to/Main.tscn") {
///     // all good
/// } else {
///     // handle error
/// }
/// ```
// TODO Result to differentiate 2 errors
#[inline]
pub fn try_load<T>(path: impl Into<GodotString>) -> Option<Gd<T>>
where
    T: GodotClass + Inherits<Resource>,
{
    load_impl(&path.into())
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Utilities for crate

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Implementation of this file

// Separate function, to avoid constructing string twice
// Note that more optimizations than that likely make no sense, as loading is quite expensive
fn load_impl<T>(path: &GodotString) -> Option<Gd<T>>
where
    T: GodotClass + Inherits<Resource>,
{
    let type_hint = T::CLASS_NAME;

    ResourceLoader::singleton()
        .load_ex(path.clone())
        .type_hint(type_hint.into())
        .done() // TODO unclone
        .and_then(|res| res.try_cast::<T>())
}
