/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! Low level bindings to the provided C core API

#![cfg_attr(test, allow(unused))]

// Output of generated code. Mimics the file structure, symbols are re-exported.
#[rustfmt::skip]
#[allow(
    non_camel_case_types,
    non_upper_case_globals,
    non_snake_case,
    deref_nullptr,
    clippy::redundant_static_lifetimes
)]
pub(crate) mod gen;

mod compat;
mod gdextension_plus;
mod godot_ffi;
mod opaque;
mod plugins;
mod toolbox;

use compat::BindingCompat;
use std::cell;
use std::ffi::CStr;

// See https://github.com/dtolnay/paste/issues/69#issuecomment-962418430
// and https://users.rust-lang.org/t/proc-macros-using-third-party-crate/42465/4
#[doc(hidden)]
pub use paste;

pub use crate::godot_ffi::{
    from_sys_init_or_init_default, GodotFfi, GodotFuncMarshal, GodotNullablePtr,
    PrimitiveConversionError, PtrcallType,
};
pub use gdextension_plus::*;
pub use gen::central::*;
pub use gen::gdextension_interface::*;
pub use gen::interface::*;
pub use toolbox::*;

// ----------------------------------------------------------------------------------------------------------------------------------------------
// API to access Godot via FFI

struct GodotBinding {
    interface: GDExtensionInterface,
    library: GDExtensionClassLibraryPtr,
    method_table: GlobalMethodTable,
    runtime_metadata: GdextRuntimeMetadata,
    config: GdextConfig,
}

struct GdextRuntimeMetadata {
    godot_version: GDExtensionGodotVersion,
}

pub struct GdextConfig {
    pub tool_only_in_editor: bool,
    pub is_editor: cell::OnceCell<bool>,
}

/// Late-init globals
// Note: static mut is _very_ dangerous. Here a bit less so, since modification happens only once (during init) and no
// &mut references are handed out (except for registry, see below). Overall, UnsafeCell/RefCell + Sync might be a safer abstraction.
static mut BINDING: Option<GodotBinding> = None;

/// # Safety
///
/// - The `interface` pointer must be either:
///   - a data pointer to a [`GDExtensionInterface`] object (for Godot 4.0.x)
///   - a function pointer of type [`GDExtensionInterfaceGetProcAddress`] (for Godot 4.1+)
/// - The `library` pointer must be the pointer given by Godot at initialisation.
/// - This function must not be called from multiple threads.
/// - This function must be called before any use of [`get_library`].
pub unsafe fn initialize(
    compat: InitCompat,
    library: GDExtensionClassLibraryPtr,
    config: GdextConfig,
) {
    out!("Initialize gdext...");

    out!(
        "Godot version against which gdext was compiled: {}",
        GdextBuild::godot_static_version_string()
    );

    // Before anything else: if we run into a Godot binary that's compiled differently from gdext, proceeding would be UB -> panic.
    compat.ensure_static_runtime_compatibility();

    let version = compat.runtime_version();
    out!("Godot version of GDExtension API at runtime: {version:?}");

    let interface = compat.load_interface();
    out!("Loaded interface.");

    let method_table = GlobalMethodTable::load(&interface);
    out!("Loaded builtin table.");

    let runtime_metadata = GdextRuntimeMetadata {
        godot_version: version,
    };

    BINDING = Some(GodotBinding {
        interface,
        method_table,
        library,
        runtime_metadata,
        config,
    });
    out!("Assigned binding.");

    println!(
        "Initialize GDExtension API for Rust: {}",
        CStr::from_ptr(version.string)
            .to_str()
            .expect("unknown Godot version")
    );
}

/// # Safety
///
/// Must be called from the same thread as `initialize()` previously.
pub unsafe fn is_initialized() -> bool {
    BINDING.is_some()
}

/// # Safety
///
/// The interface must have been initialised with [`initialize`] before calling this function.
#[inline(always)]
pub unsafe fn get_interface() -> &'static GDExtensionInterface {
    &unwrap_ref_unchecked(&BINDING).interface
}

/// # Safety
///
/// The library must have been initialised with [`initialize`] before calling this function.
#[inline(always)]
pub unsafe fn get_library() -> GDExtensionClassLibraryPtr {
    unwrap_ref_unchecked(&BINDING).library
}

/// # Safety
///
/// The interface must have been initialised with [`initialize`] before calling this function.
#[inline(always)]
pub unsafe fn method_table() -> &'static GlobalMethodTable {
    &unwrap_ref_unchecked(&BINDING).method_table
}

/// # Safety
///
/// Must be accessed from the main thread, and the interface must have been initialized.
#[inline(always)]
pub(crate) unsafe fn runtime_metadata() -> &'static GdextRuntimeMetadata {
    &BINDING.as_ref().unwrap().runtime_metadata
}

/// # Safety
///
/// Must be accessed from the main thread, and the interface must have been initialized.
#[inline]
pub unsafe fn config() -> &'static GdextConfig {
    &BINDING.as_ref().unwrap().config
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Macros to access low-level function bindings

#[macro_export]
#[doc(hidden)]
macro_rules! builtin_fn {
    ($name:ident $(@1)?) => {
        $crate::method_table().$name
    };
}

#[macro_export]
#[doc(hidden)]
macro_rules! builtin_call {
        ($name:ident ( $($args:expr),* $(,)? )) => {
            ($crate::method_table().$name)( $($args),* )
        };
    }

#[macro_export]
#[doc(hidden)]
macro_rules! interface_fn {
    ($name:ident) => {{
        unsafe { $crate::get_interface().$name.unwrap_unchecked() }
    }};
}
