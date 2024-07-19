/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate as sys;

#[cfg(since_api = "4.1")]
mod compat_4_1plus;
#[cfg(since_api = "4.1")]
pub use compat_4_1plus::*;

#[cfg(before_api = "4.1")]
mod compat_4_0;
#[cfg(before_api = "4.1")]
pub use compat_4_0::*;

/// Dispatch at runtime between Godot 4.0 legacy and 4.1+ APIs.
///
/// Provides a compatibility layer to be able to use 4.0.x extensions under Godot versions >= 4.1.
/// Also performs deterministic checks and expressive errors for cases where compatibility cannot be provided.
///
/// # Safety
///
/// [`ensure_static_runtime_compatibility`](BindingCompat::ensure_static_runtime_compatibility) succeeding should be sufficient to ensure that
/// both [`runtime_version`](BindingCompat::runtime_version) and [`load_interface`](BindingCompat::load_interface) can be called safely.
pub(crate) unsafe trait BindingCompat {
    // Implementation note: these methods could be unsafe, but that would remove any `unsafe` statements _inside_
    // the function bodies, making reasoning about them harder. Also, the call site is already an unsafe function,
    // so it would not add safety there, either.
    // Either case, given the spec of the GDExtension C API in 4.0 and 4.1, the operations should be safe.

    /// Panics on mismatch between compiled and runtime Godot version.
    ///
    /// This can happen in the following cases, with their respective subcases:
    ///
    /// 1) When a gdext version compiled against 4.1+ GDExtension API is invoked with an entry point using the legacy calling convention.
    ///    a) The .gdextension file's `[configuration]` section does not contain a `compatibility_minimum = 4.1` statement.
    ///    b) gdext was compiled against a 4.1+ Godot version, but at runtime the library is loaded from a 4.0.x version.
    ///
    /// 2) When a gdext version compiled against 4.0.x GDExtension API is invoked using the modern way.
    ///
    /// This is no guarantee, but rather a best-effort heuristic to attempt aborting rather than causing UB/crashes.
    /// Changes in the way how Godot loads GDExtension can invalidate assumptions made here.
    fn ensure_static_runtime_compatibility(&self);

    /// Return version dynamically passed via `gdextension_interface.h` file.
    ///
    /// # Safety
    ///
    /// `self` must be a valid interface or get proc address pointer.
    unsafe fn runtime_version(&self) -> sys::GDExtensionGodotVersion;

    /// Return the interface, either as-is from the header (legacy) or code-generated (modern API).
    ///
    /// # Safety
    ///
    /// `self` must be a valid interface or get proc address pointer.
    unsafe fn load_interface(&self) -> sys::GDExtensionInterface;
}
