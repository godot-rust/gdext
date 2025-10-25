/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::cell::RefCell;
use std::collections::HashMap;

use sys::is_main_thread;

use crate::builtin::NodePath;
use crate::classes::{Engine, Node, SceneTree};
use crate::meta::error::ConvertError;
use crate::obj::{Gd, Inherits, Singleton};
use crate::sys;

/// Retrieves an autoload by name.
///
/// See [Godot docs] for an explanation of the autoload concept. Godot sometimes uses the term "autoload" interchangeably with "singleton";
/// we strictly refer to the former to separate from [`Singleton`][crate::obj::Singleton] objects.
///
/// If the autoload can be resolved, it will be cached and returned very quickly the second time.
///
/// [Godot docs]: https://docs.godotengine.org/en/stable/tutorials/scripting/singletons_autoload.html
///
/// # Panics
/// This is a convenience function that calls [`try_get_autoload_by_name()`]. Panics if that fails, e.g. not found or wrong type.
///
/// # Example
/// ```no_run
/// use godot::prelude::*;
/// use godot::tools::get_autoload_by_name;
///
/// #[derive(GodotClass)]
/// #[class(init, base=Node)]
/// struct GlobalStats {
///     base: Base<Node>,
/// }
///
/// // Assuming "Statistics" is registered as an autoload in `project.godot`,
/// // this returns the one instance of type Gd<GlobalStats>.
/// let stats = get_autoload_by_name::<GlobalStats>("Statistics");
/// ```
pub fn get_autoload_by_name<T>(autoload_name: &str) -> Gd<T>
where
    T: Inherits<Node>,
{
    try_get_autoload_by_name::<T>(autoload_name)
        .unwrap_or_else(|err| panic!("Failed to get autoload `{autoload_name}`: {err}"))
}

/// Retrieves an autoload by name (fallible).
///
/// Autoloads are accessed via the `/root/{name}` path in the scene tree. The name is the one you used to register the autoload in
/// `project.godot`. By convention, it often corresponds to the class name, but does not have to.
///
/// If the autoload can be resolved, it will be cached and returned very quickly the second time.
///
/// See also [`get_autoload_by_name()`] for simpler function expecting the class name and non-fallible invocation.
///
/// This function returns `Err` if:
/// - No autoload is registered under `name`.
/// - The autoload cannot be cast to type `T`.
/// - There is an error fetching the scene tree.
///
/// # Example
/// ```no_run
/// use godot::prelude::*;
/// use godot::tools::try_get_autoload_by_name;
///
/// #[derive(GodotClass)]
/// #[class(init, base=Node)]
/// struct GlobalStats {
///     base: Base<Node>,
/// }
///
/// let result = try_get_autoload_by_name::<GlobalStats>("Statistics");
/// match result {
///     Ok(autoload) => { /* Use the Gd<GlobalStats>. */ }
///     Err(err) => eprintln!("Failed to get autoload: {err}"),
/// }
/// ```
pub fn try_get_autoload_by_name<T>(autoload_name: &str) -> Result<Gd<T>, ConvertError>
where
    T: Inherits<Node>,
{
    ensure_main_thread()?;

    // Check cache first.
    let cached = AUTOLOAD_CACHE.with(|cache| cache.borrow().get(autoload_name).cloned());

    if let Some(cached_node) = cached {
        return cast_autoload(cached_node, autoload_name);
    }

    // Cache miss - fetch from scene tree.
    let main_loop = Engine::singleton()
        .get_main_loop()
        .ok_or_else(|| ConvertError::new("main loop not available"))?;

    let scene_tree = main_loop
        .try_cast::<SceneTree>()
        .map_err(|_| ConvertError::new("main loop is not a SceneTree"))?;

    let autoload_path = NodePath::from(&format!("/root/{autoload_name}"));

    let root = scene_tree
        .get_root()
        .ok_or_else(|| ConvertError::new("scene tree root not available"))?;

    let autoload_node = root
        .try_get_node_as::<Node>(&autoload_path)
        .ok_or_else(|| ConvertError::new(format!("autoload `{autoload_name}` not found")))?;

    // Store in cache as Gd<Node>.
    AUTOLOAD_CACHE.with(|cache| {
        cache
            .borrow_mut()
            .insert(autoload_name.to_string(), autoload_node.clone());
    });

    // Cast to requested type.
    cast_autoload(autoload_node, autoload_name)
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Cache implementation

thread_local! {
    /// Cache for autoloads. Maps autoload name to `Gd<Node>`.
    ///
    /// Uses `thread_local!` because `Gd<T>` is not `Send`/`Sync`. Since all Godot objects must be accessed
    /// from the main thread, this is safe. We enforce main-thread access via `ensure_main_thread()`.
    static AUTOLOAD_CACHE: RefCell<HashMap<String, Gd<Node>>> = RefCell::new(HashMap::new());
}

/// Verifies that the current thread is the main thread.
///
/// Returns an error if called from a thread other than the main thread. This is necessary because `Gd<T>` is not thread-safe.
fn ensure_main_thread() -> Result<(), ConvertError> {
    if is_main_thread() {
        Ok(())
    } else {
        Err(ConvertError::new(
            "Autoloads must be fetched from main thread, as Gd<T> is not thread-safe",
        ))
    }
}

/// Casts an autoload node to the requested type, with descriptive error message on failure.
fn cast_autoload<T>(node: Gd<Node>, autoload_name: &str) -> Result<Gd<T>, ConvertError>
where
    T: Inherits<Node>,
{
    node.try_cast::<T>().map_err(|node| {
        let expected = T::class_id();
        let actual = node.get_class();

        ConvertError::new(format!(
            "autoload `{autoload_name}` has wrong type (expected {expected}, got {actual})",
        ))
    })
}

/// Clears the autoload cache (called during shutdown).
///
/// # Panics
/// Panics if called from a thread other than the main thread.
pub(crate) fn clear_autoload_cache() {
    ensure_main_thread().expect("clear_autoload_cache() must be called from the main thread");

    AUTOLOAD_CACHE.with(|cache| {
        cache.borrow_mut().clear();
    });
}
