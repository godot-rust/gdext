/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::builtin::GString;
use crate::classes::{Resource, ResourceLoader, ResourceSaver};
use crate::global::{Error as GodotError, suppress_godot_errors};
use crate::meta::error::IoError;
use crate::meta::{AsArg, arg_into_ref};
use crate::obj::{Gd, Inherits, Singleton};

/// ⚠️ Loads a resource from the filesystem located at `path`, panicking on error.
///
/// See [`try_load()`] for more information.
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
/// On failure, Godot error printing is temporarily suppressed, so no console error appears.
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

/// ⚠️ Loads a resource from the filesystem located at `path` on a worker thread, panicking on error.
///
/// See [`try_load()`] and [`try_load_threaded()`] for more information.
///
/// # Example
///
/// ```no_run
/// use godot::prelude::*;
///
/// godot::task::spawn(async {
///     let scene = load_threaded::<PackedScene>("res://path/to/Main.tscn").await;
/// });
/// ```
///
/// # Panics
/// If the resource cannot be loaded, is not of type `T` or inherited or the global `MainLoop` is not a `SceneTree`.
#[inline]
#[cfg(feature = "experimental-threads")]
pub async fn load_threaded<T>(path: impl AsArg<GString>) -> Gd<T>
where
    T: Inherits<Resource>,
{
    arg_into_ref!(path);
    load_threaded_impl(path)
        .await
        .unwrap_or_else(|err| panic!("failed to load resource at '{path}': {err}"))
}

/// Loads a resource from the filesystem located at `path` on a worker thread.
///
/// This function can fail if resource can't be loaded by [`ResourceLoader`] or if the subsequent cast into `T` fails.
///
/// This method is a simplified version of [`ResourceLoader::load_threaded_request`][crate::classes::ResourceLoader::load_threaded_request],
/// which can be used for more advanced scenarios.
///
/// See synchronous version [`try_load()`] for more details.
///
/// # Example
/// Loads a scene called `Main` located in the `path/to` subdirectory of the Godot project and caches it in a variable.
/// The resource is directly stored with type `PackedScene`.
///
/// ```no_run
/// use godot::prelude::*;
///
/// godot::task::spawn(async {
///     if let Ok(scene) = try_load_threaded::<PackedScene>("res://path/to/Main.tscn").await {
///         // all good
///     } else {
///         // handle error
///     }
/// });
/// ```
#[inline]
#[cfg(feature = "experimental-threads")]
pub async fn try_load_threaded<T>(path: impl AsArg<GString>) -> Result<Gd<T>, IoError>
where
    T: Inherits<Resource>,
{
    arg_into_ref!(path);
    load_threaded_impl(path).await
}

/// ⚠️ Saves a [`Resource`]-inheriting object into the file located at `path`.
///
/// See [`try_save()`] for more information.
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
#[inline]
pub fn save<T>(obj: &Gd<T>, path: impl AsArg<GString>)
where
    T: Inherits<Resource>,
{
    arg_into_ref!(path);

    save_impl(obj, path)
        .unwrap_or_else(|err| panic!("failed to save resource at path '{path}': {err}"));
}

/// Saves a [`Resource`]-inheriting object into the file located at `path`.
///
/// This function can fail if [`ResourceSaver`] can't save the resource to file, as it is a simplified version of
/// [`ResourceSaver::save()`][crate::classes::ResourceSaver::save]. The underlying method can be used for more advances scenarios.
///
/// On failure, Godot error printing is temporarily suppressed, so no console error appears.
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

// Separate function, to avoid constructing string twice.
// Note that more optimizations than that likely make no sense, as loading is quite expensive.
fn load_impl<T>(path: &GString) -> Result<Gd<T>, IoError>
where
    T: Inherits<Resource>,
{
    let _guard = suppress_godot_errors();

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

// Separate function, to avoid constructing string twice
// Note that more optimizations than that likely make no sense, as loading is quite expensive
#[cfg(feature = "experimental-threads")]
async fn load_threaded_impl<T>(path: &GString) -> Result<Gd<T>, IoError>
where
    T: Inherits<Resource>,
{
    use crate::classes::resource_loader::ThreadLoadStatus;
    use crate::classes::{Engine, SceneTree};

    let mut resource_loader = ResourceLoader::singleton();
    let tree = Engine::singleton()
        .get_main_loop()
        .ok_or_else(|| {
            IoError::loading_precondition(
                T::class_id().to_string(),
                path.to_string(),
                "SceneTree is ready",
            )
        })?
        .try_cast::<SceneTree>()
        .map_err(|_| {
            IoError::loading_precondition(
                T::class_id().to_string(),
                path.to_string(),
                "MainLoop is SceneTree",
            )
        })?;

    resource_loader
        .load_threaded_request_ex(path)
        .type_hint(&T::class_id().to_gstring())
        .done();

    loop {
        match resource_loader.load_threaded_get_status(path) {
            ThreadLoadStatus::IN_PROGRESS => {
                tree.signals().process_frame().to_future().await;
            }

            ThreadLoadStatus::LOADED => {
                break resource_loader
                    .load_threaded_get(path)
                    .unwrap()
                    .try_cast::<T>()
                    .map_err(|_| {
                        IoError::loading_cast(T::class_id().to_string(), path.to_string())
                    });
            }

            ThreadLoadStatus::INVALID_RESOURCE | ThreadLoadStatus::FAILED => {
                // Prevents a memory leak. The engine creates a resource with ref-count 0 and it has to be collected from the loader.
                resource_loader.load_threaded_get(path);

                break Err(IoError::loading(
                    T::class_id().to_string(),
                    path.to_string(),
                ));
            }

            // All load status variants have been covered.
            _ => unreachable!(),
        }
    }
}

fn save_impl<T>(obj: &Gd<T>, path: &GString) -> Result<(), IoError>
where
    T: Inherits<Resource>,
{
    let _guard = suppress_godot_errors();

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
