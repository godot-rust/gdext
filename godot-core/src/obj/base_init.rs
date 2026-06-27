/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! Before Godot 4.7 initialization layers – be it an engine module or gdextension – was working with not yet fully initialized
//! object (i.e. RefCounted with reference count `0`). Increasing and decreasing RefCount anyhow (by, for example,
//! passing it to one of the Godot APIs) was resulting in freeing the Object.
//! Additionally, initializing freshly constructed instance was caller responsibility.
//!
//! Since Godot 4.7 initialization layer receives (and yields) fully initialized object.
//!
//! `base_weak_initialization` contains implementation for safe workaround around this issue (deffered init) for Godot < 4.7.
//! `base_strong_initialization` contains placeholders for Godot >= 4.7.
//!
//! For more information see also [`Base::to_init_gd`].

#[cfg(since_api = "4.7")]
pub(super) use super::base_strong_initialization::InitTracker;
#[cfg(before_api = "4.7")]
pub(super) use super::base_weak_initialization::InitTracker;

/// Represents the initialization state of a `Base<T>` object.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum InitState {
    /// Object is being constructed (inside `I*::init()` or `Gd::from_init_fn()`).
    ObjectConstructing,

    /// Object construction is complete.
    #[cfg(before_api = "4.7")]
    ObjectInitialized,

    /// `ScriptInstance` context - always considered initialized (bypasses lifecycle checks).
    Script,
}
