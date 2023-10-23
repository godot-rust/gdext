/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! Testing that GDScript and rust produces the same property info for properties exported to Godot.

// We're using some weird formatting just for simplicity's sake.
#![allow(non_snake_case)]

use std::collections::HashMap;

use crate::framework::itest;
use godot::{engine::global::PropertyUsageFlags, prelude::*};

use crate::framework::TestContext;

use crate::register_tests::gen_ffi::PropertyTemplateRust;

#[itest]
fn property_template_test(ctx: &TestContext) {
    let rust_properties = Gd::<PropertyTemplateRust>::new_default();
    let gdscript_properties = ctx.property_template.clone();

    // Accumulate errors so we can catch all of them in one go.
    let mut errors: Vec<String> = Vec::new();
    let mut properties: HashMap<String, Dictionary> = HashMap::new();

    for property in rust_properties.get_property_list().iter_shared() {
        let name = property
            .get("name")
            .unwrap()
            .to::<GodotString>()
            .to_string();
        if name.starts_with("property_") || name.starts_with("export_") {
            properties.insert(name, property);
        }
    }

    assert!(!properties.is_empty());

    for property in gdscript_properties.get_property_list().iter_shared() {
        let name = property
            .get("name")
            .unwrap()
            .to::<GodotString>()
            .to_string();

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
                "mismatch in property {name}, gdscript: {property:?}, rust: {rust_prop:?}"
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
