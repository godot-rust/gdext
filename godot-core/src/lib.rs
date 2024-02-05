/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

mod registry;
mod storage;

pub mod builder;
pub mod builtin;
pub mod init;
pub mod log;
pub mod obj;
pub mod property;

#[doc(hidden)]
#[path = "deprecated.rs"]
pub mod __deprecated;
#[doc(hidden)]
pub mod private;

pub use godot_ffi as sys;
#[doc(hidden)]
pub use godot_ffi::out;
pub use registry::*;

/// Maps the Godot class API to Rust.
///
/// This module contains the following symbols:
/// * Classes: `CanvasItem`, etc.
/// * Virtual traits: `ICanvasItem`, etc.
/// * Enum/flag modules: `canvas_item`, etc.
///
/// Noteworthy sub-modules are:
/// * [`notify`][crate::engine::notify]: all notification types, used when working with the virtual callback to handle lifecycle notifications.
/// * [`global`][crate::engine::global]: global enums not belonging to a specific class.
/// * [`utilities`][crate::engine::utilities]: utility methods that are global in Godot.
/// * [`translate`][crate::engine::translate]: convenience macros for translation.
pub mod engine;

// Output of generated code. Mimics the file structure, symbols are re-exported.
#[rustfmt::skip]
#[allow(unused_imports, dead_code, non_upper_case_globals, non_snake_case)]
#[allow(clippy::too_many_arguments, clippy::let_and_return, clippy::new_ret_no_self)]
#[allow(clippy::let_unit_value)] // let args = ();
#[allow(clippy::wrong_self_convention)] // to_string() is const
#[allow(clippy::upper_case_acronyms)] // TODO remove this line once we transform names
#[allow(unreachable_code, clippy::unimplemented)] // TODO remove once #153 is implemented
mod gen;



macro_rules! generate_gdextension_api_version {
    (
        $(
            ($name:ident, $gdextension_api:ident) => {
                $($version:literal, )*
            }
        ),* $(,)?
    ) => {
        $(
            $(
                #[cfg($gdextension_api = $version)]
                #[allow(dead_code)]
                const $name: &str = $version;
            )*
        )*
    };
}

// If multiple gdextension_api_version's are found then this will generate several structs with the same
// name, causing a compile error.
//
// This includes all versions we're developing for, including unreleased future versions.
generate_gdextension_api_version!(
    (GDEXTENSION_EXACT_API, gdextension_exact_api) => {
        "4.0",
        "4.0.1",
        "4.0.2",
        "4.0.3",
        "4.0.4",
        "4.1",
        "4.1.1",
    },
    (GDEXTENSION_API, gdextension_minor_api) => {
        "4.0",
        "4.1",
    },
);
