/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! Godot engine classes and methods.

// Re-exports of generated symbols
use crate::builtin::{GodotString, NodePath};
use crate::engine::resource_loader::CacheMode;
use crate::obj::{Gd, GodotClass, Inherits};
pub use gen::central_core::global;
pub use gen::classes::*;
pub use gen::utilities;

/// Output of generated code.
pub(super) mod gen {
    #[allow(unused_imports, dead_code, non_upper_case_globals, non_snake_case)]
    pub(crate) mod classes {
        // Path to core/classes/obj
        // Do not write macro for this, as it confuses IDEs -- just search&replace
        include!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../target/godot-gen/core/classes/mod.rs"
        ));
    }

    pub mod utilities {
        // Path to core/utilities.rs
        // Do not write macro for this, as it confuses IDEs -- just search&replace
        include!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../target/godot-gen/core/utilities.rs"
        ));
    }

    #[allow(non_upper_case_globals, non_snake_case)]
    pub mod central_core {
        // Path to core/utilities.rs
        // Do not write macro for this, as it confuses IDEs -- just search&replace
        include!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../target/godot-gen/core/central.rs"
        ));
    }
}

/// Extension trait with convenience functions for the node tree
pub trait NodeExt {
    fn get_node_as<T>(&self, path: impl Into<NodePath>) -> Gd<T>
    where
        T: GodotClass + Inherits<Node>;
}

impl NodeExt for Node {
    fn get_node_as<T>(&self, path: impl Into<NodePath>) -> Gd<T>
    where
        T: GodotClass + Inherits<Node>,
    {
        let path = path.into();

        let node = self
            .get_node(path)
            .unwrap_or_else(|| panic!("There is no node at path `{path}`"));
        node.cast::<T>()
    }
}

impl<U> NodeExt for Gd<U>
where
    U: GodotClass + Inherits<Node>,
{
    fn get_node_as<T>(&self, path: impl Into<NodePath>) -> Gd<T>
    where
        T: GodotClass + Inherits<Node>,
    {
        // TODO easier impl, no share(), but ideally also don't add too many bounds
        let path = path.into();

        use crate::obj::Share;
        let node = self
            .share()
            .upcast::<Node>()
            .get_node(path)
            .unwrap_or_else(|| panic!("There is no node at path `{path}`"));

        node.cast::<T>()
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

// Separate function, to avoid constructing string twice
// Note that more optimizations than that likely make no sense, as loading is quite expensive
fn load_impl<T>(path: &GodotString) -> Option<Gd<T>>
where
    T: GodotClass + Inherits<Resource>,
{
    let type_hint = T::CLASS_NAME;

    ResourceLoader::singleton()
        .load(
            path.clone(), /* TODO unclone */
            type_hint.into(),
            CacheMode::CACHE_MODE_REUSE,
        )
        .and_then(|res| res.try_cast::<T>())
}
