/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! Testing that GDScript and Rust produces the same property info for properties exported to Godot.

// We're using some weird formatting just for simplicity's sake.
#![allow(non_snake_case)]

use std::collections::HashMap;

use godot::global::PropertyUsageFlags;
use godot::prelude::*;
use godot::sys::GdextBuild;

use crate::framework::{itest, TestContext};
use crate::register_tests::gen_ffi::PropertyTestsRust;

#[itest]
fn property_template_test(ctx: &TestContext) {
    let rust_properties = PropertyTestsRust::new_alloc();
    let gdscript_properties = ctx.property_tests.clone();

    // Accumulate errors so we can catch all of them in one go.
    let mut errors: Vec<String> = Vec::new();
    let mut properties: HashMap<String, VarDictionary> = HashMap::new();

    for property in rust_properties.get_property_list().iter_shared() {
        let name = property.get("name").unwrap().to::<String>();

        // Skip @export_file and similar properties for Array<GString> and PackedStringArray (only supported in Godot 4.3+).
        // Here, we use API and not runtime level, because inclusion/exclusion of GDScript code is determined at build time in godot-bindings.
        // Anecdote: the format of array properties changed in Godot 4.2.
        //
        // Name can start in `export_file`, `export_global_file`, `export_dir`, `export_global_dir`.
        // Can end in either `_array` or `_parray`.
        #[cfg(before_api = "4.3")]
        if (name.contains("_file_") || name.contains("_dir_")) && name.ends_with("array") {
            continue;
        }

        if name.starts_with("var_") || name.starts_with("export_") {
            properties.insert(name, property);
        }
    }

    assert!(!properties.is_empty());

    for mut gdscript_prop in gdscript_properties.get_property_list().iter_shared() {
        let name = gdscript_prop.at("name").to::<String>();

        let Some(mut rust_prop) = properties.remove(&name) else {
            continue;
        };

        let mut rust_usage = rust_prop.at("usage").to::<PropertyUsageFlags>();

        // The GDSscript variables are script variables, and so have `PROPERTY_USAGE_SCRIPT_VARIABLE` set.
        // Before 4.3, `PROPERTY_USAGE_SCRIPT_VARIABLE` did the same thing as `PROPERTY_USAGE_STORAGE` and
        // so GDScript didn't set both if it didn't need to.
        if GdextBuild::before_api("4.3") {
            if rust_usage == PropertyUsageFlags::STORAGE {
                rust_usage = PropertyUsageFlags::SCRIPT_VARIABLE
            } else {
                rust_usage |= PropertyUsageFlags::SCRIPT_VARIABLE;
            }
        } else {
            rust_usage |= PropertyUsageFlags::SCRIPT_VARIABLE;
        }

        rust_prop.set("usage", rust_usage);

        // From Godot 4.4, GDScript uses `.0` for integral floats, see https://github.com/godotengine/godot/pull/47502.
        // We still register them the old way, to test compatibility. See also godot-core/src/registry/property.rs.
        // Since GDScript now registers them with `.0`, we need to account for that.
        if GdextBuild::since_api("4.4") {
            let mut hint_string = gdscript_prop.at("hint_string").to::<String>();

            // Don't check against `.0` to not accidentally catch `.02`. We don't have regex available here.
            if hint_string.contains(".0,") {
                hint_string = hint_string.replace(".0,", ",");
                gdscript_prop.set("hint_string", hint_string.clone());
            }

            if hint_string.ends_with(".0") {
                gdscript_prop.set("hint_string", hint_string.trim_end_matches(".0"));
            }
        }

        if rust_prop != gdscript_prop {
            errors.push(format!(
                "mismatch in property {name}:\n  GDScript: {gdscript_prop:?}\n  Rust:     {rust_prop:?}"
            ));
        }
        /*else { // Keep around for debugging.
            println!(
                "good property {name}:\n  GDScript: {gdscript_prop:?}\n  Rust:     {rust_prop:?}"
            );
        }*/
    }

    rust_properties.free();

    assert!(
        properties.is_empty(),
        "not all properties were matched, missing: {properties:?}"
    );

    assert!(
        errors.is_empty(),
        "Encountered {} mismatches between GDScript and Rust:\n{}",
        errors.len(),
        errors.join("\n")
    );
}
