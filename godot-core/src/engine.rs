/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! Godot engine classes and methods.

// Re-exports of generated symbols
use crate::builtin::NodePath;
use crate::obj::{Gd, GodotClass, Inherits};
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

/// Extension trait with convenience functions for the node tree
pub trait NodeExt {
    fn get_node_as<T>(&self, path: NodePath) -> Gd<T>
    where
        T: GodotClass + Inherits<Node>;
}

impl NodeExt for Node {
    fn get_node_as<T>(&self, path: NodePath) -> Gd<T>
    where
        T: GodotClass + Inherits<Node>,
    {
        let node = self.get_node(path);
        node.cast::<T>()
    }
}

impl<U> NodeExt for Gd<U>
where
    U: GodotClass + Inherits<Node>,
{
    fn get_node_as<T>(&self, path: NodePath) -> Gd<T>
    where
        T: GodotClass + Inherits<Node>,
    {
        // TODO easier impl, no share(), but ideally also don't add too many bounds

        use crate::obj::Share;
        let node = self.share().upcast::<Node>().get_node(path);
        node.cast::<T>()
    }
}
