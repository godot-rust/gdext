/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! Allows dependent crates to statically discover classes registered by this extension.
//!
//! # Rationale
//!
//! The idea is to provide a mechanism that allows tools and integrations to query information about an extension _at build time_.
//! For example, this allows validations, data extraction/generation, etc. It is limited to downstream crates depending on the crate declaring
//! the extension, in particular their `build.rs` file. If a crate is used for discovery, then it must declare `crate-type = ["cdylib", "rlib"]`,
//! i.e. both a C dynamic library for GDExtension and a Rust library.
//!
//! The API is kept deliberately minimal -- it does not strive to cover the entire reflection API that Godot provides. Many tools may be better
//! fitted as a direct integration into the Godot editor, or as runtime code querying Godot's `ClassDB` API. If you believe this API is lacking,
//! please provide a detailed use case.
//!
//! # Usage
//!
//! To make use of discovery, your entry point trait needs to have the `discover` attribute:
//! ```no_run
//! # use godot::prelude::*;
//! /// Your tag must be public.
//! pub struct MyCoolGame;
//!
//! /// The `discovery` attributes adds an associated `MyExtension::discover()` function to the `MyExtension` tag.
//! /// It also declares a module `godot_discovery` in the current scope, which re-exports symbols from `godot::init::discovery`.
//! /// This allows your dependent crate to not depend on `godot` directly.
//! #[gdextension(discovery)]
//! unsafe impl ExtensionLibrary for MyCoolGame {}
//! ```
//!
//! To access the exposed API in a dependent crate in `build.rs`, you can call `MyExtension::discover()`:
//! ```no_run
//! # mod my_crate {
//! # use godot::init::gdextension;
//! # pub struct MyCoolGame;
//! # #[gdextension(discovery)]
//! # unsafe impl godot::init::ExtensionLibrary for MyCoolGame {}
//! # }
//! use my_crate::MyCoolGame;
//! use my_crate::godot_discovery::DiscoveredExtension;
//!
//! fn main() {
//!    let api: DiscoveredExtension = MyCoolGame::discover();
//!    for c in api.classes() {
//!        println!("Discovered class {}.", c.name());
//!    }
//! }
//! ```

use crate::private::{ClassPlugin, PluginItem};

pub struct DiscoveredExtension {
    classes: Vec<DiscoveredClass>,
}

impl DiscoveredExtension {
    pub fn classes(&self) -> &[DiscoveredClass] {
        &self.classes
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

pub struct DiscoveredClass {
    name: String,
    base_class: String,
}

impl DiscoveredClass {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn base_class(&self) -> &str {
        &self.base_class
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

#[doc(hidden)]
pub fn __discover() -> DiscoveredExtension {
    let mut classes = vec![];

    crate::private::iterate_plugins(|elem: &ClassPlugin| {
        let PluginItem::Struct {
            base_class_name, ..
        } = elem.item
        else {
            return;
        };

        let class = DiscoveredClass {
            name: elem.class_name.to_string(),
            base_class: base_class_name.to_string(),
        };

        classes.push(class);
    });

    DiscoveredExtension { classes }
}
