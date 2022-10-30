/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

#[doc(inline)]
pub use godot_core::{builtin, engine, log, obj, sys};

/// Facilities for initializing and terminating the GDExtension library.
pub mod init {
    pub use godot_core::init::*;

    // Re-exports
    pub use godot_macros::gdextension;
}

/// Export user-defined classes and methods to be called by the engine.
pub mod bind {
    pub use godot_core::bind::*;

    // Re-exports
    pub use godot_macros::{godot_api, GodotClass};
}

/// Testing facilities (unstable).
#[doc(hidden)]
pub mod test {
    pub use godot_macros::itest;
}

#[doc(hidden)]
pub use godot_core::private;
