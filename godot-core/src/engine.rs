/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! Godot engine classes and methods.

// Re-exports of generated symbols
pub use gen::central_core::global;
pub use gen::classes::*;
pub use gen::utilities;

/// Output of generated code.
pub(super) mod gen {
    #[allow(unused_imports, dead_code, non_upper_case_globals, non_snake_case)]
    pub(crate) mod classes {
        // Path to core/classes/obj
        // Do not write macro for this, as it confuses IDEs -- just search&replace
        include!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../target/godot-gen/core/classes/mod.rs"
        ));
    }

    pub mod utilities {
        // Path to core/utilities.rs
        // Do not write macro for this, as it confuses IDEs -- just search&replace
        include!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../target/godot-gen/core/utilities.rs"
        ));
    }

    #[allow(non_upper_case_globals, non_snake_case)]
    pub mod central_core {
        // Path to core/utilities.rs
        // Do not write macro for this, as it confuses IDEs -- just search&replace
        include!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../target/godot-gen/core/central.rs"
        ));
    }
}
