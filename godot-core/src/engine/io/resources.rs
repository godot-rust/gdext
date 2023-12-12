/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::builtin::GString;
use crate::gen::classes::{Resource, ResourceLoader};
use crate::obj::{Gd, GodotClass, Inherits};

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
pub fn load<T>(path: impl Into<GString>) -> Gd<T>
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
/// This method is a simplified version of [`ResourceLoader::load()`][crate::engine::ResourceLoader::load],
/// which can be used for more advanced scenarios.
///
/// # Note:
/// Resource paths can be obtained by right-clicking on a resource in the Godot editor (_FileSystem_ dock) and choosing "Copy Path",
/// or by dragging the file from the _FileSystem_ dock into the script.
///
/// The path must be absolute (typically starting with `res://`), a local path will fail.
///
/// # Example
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
pub fn try_load<T>(path: impl Into<GString>) -> Option<Gd<T>>
where
    T: GodotClass + Inherits<Resource>,
{
    load_impl(&path.into())
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Implementation of this file

// Separate function, to avoid constructing string twice
// Note that more optimizations than that likely make no sense, as loading is quite expensive
fn load_impl<T>(path: &GString) -> Option<Gd<T>>
where
    T: GodotClass + Inherits<Resource>,
{
    ResourceLoader::singleton()
        .load_ex(path.clone())
        .type_hint(T::class_name().to_godot_string())
        .done() // TODO unclone
        .and_then(|res| res.try_cast::<T>().ok())
}
