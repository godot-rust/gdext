/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! Versions to be updated whenever Godot releases a new patch version we support.
//!
//! This file contains several templating comments, who are substituted by the machinery itest/repo-tweak.
//! When modifying those, make sure to rerun.

/// All stable Godot releases _and_ upcoming next minor release.
pub const ALL_VERSIONS: &[(u8, u8, u8)] = &[
    // [version-sync] [[
    //  [include] past+current+future
    //  [line] \t$triple,
    (4, 2, 0),
    (4, 2, 1),
    (4, 2, 2),
    (4, 3, 0),
    (4, 4, 0),
    (4, 5, 0),
    (4, 6, 0),
    (4, 7, 0),
    // ]]
];

// [version-sync] [[
//  [line] #[cfg(feature = "api-$kebabVersion")]\npub use gdextension_api::version_$snakeVersion as prebuilt;
#[cfg(feature = "api-4-2")]
pub use gdextension_api::version_4_2 as prebuilt;
#[cfg(feature = "api-4-2-1")]
pub use gdextension_api::version_4_2_1 as prebuilt;
#[cfg(feature = "api-4-2-2")]
pub use gdextension_api::version_4_2_2 as prebuilt;
#[cfg(feature = "api-4-3")]
pub use gdextension_api::version_4_3 as prebuilt;
#[cfg(feature = "api-4-4")]
pub use gdextension_api::version_4_4 as prebuilt;
#[cfg(feature = "api-4-5")]
pub use gdextension_api::version_4_5 as prebuilt;
#[cfg(feature = "api-4-6")]
pub use gdextension_api::version_4_6 as prebuilt;
// ]]

// If none of the api-* features are provided, use default prebuilt version (typically latest Godot stable release).

// [version-sync] [[
//  [line] \tfeature = "api-$kebabVersion",
//  [pre] #[cfg(not(any(
//  [post] \tfeature = "api-custom",\n\tfeature = "api-custom-json",\n)))]
#[cfg(not(any(
    feature = "api-4-2",
    feature = "api-4-2-1",
    feature = "api-4-2-2",
    feature = "api-4-3",
    feature = "api-4-4",
    feature = "api-4-5",
    feature = "api-4-6",
    feature = "api-custom",
    feature = "api-custom-json",
)))]
// ]]
// [version-sync] [[
//  [include] current.minor
//  [line] pub use gdextension_api::version_$snakeVersion as prebuilt;
pub use gdextension_api::version_4_6 as prebuilt;
// ]]
