/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::builtin::GString;
use crate::classes::{Resource, ResourceLoader, ResourceSaver};
use crate::global::Error as GodotError;
use crate::meta::error::IoError;
use crate::meta::{arg_into_ref, AsArg};
use crate::obj::{Gd, Inherits, Singleton};

/// ⚠️ Loads a resource from the filesystem located at `path`, panicking on error.
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
pub fn load<T>(path: impl AsArg<GString>) -> Gd<T>
where
    T: Inherits<Resource>,
{
    arg_into_ref!(path);
    load_impl(path).unwrap_or_else(|err| panic!("failed to load resource at '{path}': {err}"))
}

/// Loads a resource from the filesystem located at `path`.
///
/// The resource is loaded during the method call, unless it is already referenced elsewhere, e.g. in another script or in the scene.
/// This might cause slight delay, especially when loading scenes.
///
/// This function can fail if resource can't be loaded by [`ResourceLoader`] or if the subsequent cast into `T` fails.
///
/// This method is a simplified version of [`ResourceLoader::load()`][crate::classes::ResourceLoader::load],
/// which can be used for more advanced scenarios.
///
/// # Note
/// Resource paths can be obtained by right-clicking on a resource in the Godot editor (_FileSystem_ dock) and choosing _Copy Path_,
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
/// if let Ok(scene) = try_load::<PackedScene>("res://path/to/Main.tscn") {
///     // all good
/// } else {
///     // handle error
/// }
/// ```
#[inline]
pub fn try_load<T>(path: impl AsArg<GString>) -> Result<Gd<T>, IoError>
where
    T: Inherits<Resource>,
{
    arg_into_ref!(path);
    load_impl(path)
}

/// ⚠️ Saves a [`Resource`]-inheriting object into the file located at `path`.
///
/// See [`try_save`] for more information.
///
/// # Panics
/// If the resource cannot be saved.
///
/// # Example
/// ```no_run
/// use godot::prelude::*;
///
/// let obj = Resource::new_gd();
/// save(&obj, "res://BaseResource.tres")
/// ```
/// use godot::
#[inline]
pub fn save<T>(obj: &Gd<T>, path: impl AsArg<GString>)
where
    T: Inherits<Resource>,
{
    arg_into_ref!(path);

    save_impl(obj, path)
        .unwrap_or_else(|err| panic!("failed to save resource at path '{}': {}", &path, err));
}

/// Saves a [`Resource`]-inheriting object into the file located at `path`.
///
/// This function can fail if [`ResourceSaver`] can't save the resource to file, as it is a simplified version of
/// [`ResourceSaver::save()`][crate::classes::ResourceSaver::save]. The underlying method can be used for more advances scenarios.
///
/// # Note
/// Target path must be presented in Godot-recognized format, mainly the ones beginning with `res://` and `user://`. Saving
/// to `res://` is possible only when working with unexported project - after its export only `user://` is viable.
///
/// # Example
/// ```no_run
/// use godot::prelude::*;
///
/// #[derive(GodotClass)]
/// #[class(base=Resource, init)]
/// struct SavedGame {
///   // Exported properties are saved in `.tres` files.
///   #[export]
///   level: u32
/// };
///
/// let save_state = SavedGame::new_gd();
/// let res = try_save(&save_state, "user://save.tres");
///
/// assert!(res.is_ok());
/// ```
#[inline]
pub fn try_save<T>(obj: &Gd<T>, path: impl AsArg<GString>) -> Result<(), IoError>
where
    T: Inherits<Resource>,
{
    arg_into_ref!(path);

    save_impl(obj, path)
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Implementation of this file

// Separate function, to avoid constructing string twice
// Note that more optimizations than that likely make no sense, as loading is quite expensive
fn load_impl<T>(path: &GString) -> Result<Gd<T>, IoError>
where
    T: Inherits<Resource>,
{
    let loaded = ResourceLoader::singleton()
        .load_ex(path)
        .type_hint(&T::class_id().to_gstring())
        .done();

    match loaded {
        Some(res) => match res.try_cast::<T>() {
            Ok(obj) => Ok(obj),
            Err(_) => Err(IoError::loading_cast(
                T::class_id().to_string(),
                path.to_string(),
            )),
        },
        None => Err(IoError::loading(
            T::class_id().to_string(),
            path.to_string(),
        )),
    }
}

fn save_impl<T>(obj: &Gd<T>, path: &GString) -> Result<(), IoError>
where
    T: Inherits<Resource>,
{
    let res = ResourceSaver::singleton().save_ex(obj).path(path).done();

    if res == GodotError::OK {
        Ok(())
    } else {
        Err(IoError::saving(
            res,
            T::class_id().to_string(),
            path.to_string(),
        ))
    }
}
