/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! Testing that GDScript and Rust produces the same property info for properties exported to Godot.

// We're using some weird formatting just for simplicity's sake.
#![allow(non_snake_case)]

use std::collections::HashMap;

use crate::framework::itest;
use godot::engine::global::PropertyUsageFlags;
use godot::prelude::*;
use godot::sys::GdextBuild;

use crate::framework::TestContext;

use crate::register_tests::gen_ffi::PropertyTestsRust;

#[itest]
fn property_template_test(ctx: &TestContext) {
    let rust_properties = PropertyTestsRust::alloc_gd();
    let gdscript_properties = ctx.property_tests.clone();

    // Accumulate errors so we can catch all of them in one go.
    let mut errors: Vec<String> = Vec::new();
    let mut properties: HashMap<String, Dictionary> = HashMap::new();

    for property in rust_properties.get_property_list().iter_shared() {
        let name = property.get("name").unwrap().to::<String>();

        // The format of array-properties in Godot 4.2 changed. This doesn't seem to cause issues if we
        // compile against 4.1 and provide the property in the format 4.1 expects but run it with Godot 4.2.
        // However this test checks that our output matches that of Godot, and so would fail in this
        // circumstance. So here for now, just ignore array properties when we compile for 4.1 but run in 4.2.
        if GdextBuild::since_api("4.2")
            && cfg!(before_api = "4.2")
            && name.starts_with("property_array_")
        {
            continue;
        }

        if name.starts_with("property_") || name.starts_with("export_") {
            properties.insert(name, property);
        }
    }

    assert!(!properties.is_empty());

    for property in gdscript_properties.get_property_list().iter_shared() {
        let name = property.get("name").unwrap().to::<String>();

        let Some(mut rust_prop) = properties.remove(&name) else {
            continue;
        };

        let mut rust_usage = rust_prop.get("usage").unwrap().to::<i64>();

        // the GDSscript variables are script variables, and so have `PROPERTY_USAGE_SCRIPT_VARIABLE` set.
        if rust_usage == PropertyUsageFlags::PROPERTY_USAGE_STORAGE.ord() as i64 {
            // `PROPERTY_USAGE_SCRIPT_VARIABLE` does the same thing as `PROPERTY_USAGE_STORAGE` and so
            // GDScript doesn't set both if it doesn't need to.
            rust_usage = PropertyUsageFlags::PROPERTY_USAGE_SCRIPT_VARIABLE.ord() as i64
        } else {
            rust_usage |= PropertyUsageFlags::PROPERTY_USAGE_SCRIPT_VARIABLE.ord() as i64;
        }

        rust_prop.set("usage", rust_usage);

        if rust_prop != property {
            errors.push(format!(
                "mismatch in property {name}, GDScript: {property:?}, Rust: {rust_prop:?}"
            ));
        }
    }

    assert!(
        properties.is_empty(),
        "not all properties were matched, missing: {properties:?}"
    );

    assert!(errors.is_empty(), "{}", errors.join("\n"));

    rust_properties.free();
}
