/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! Identifier renamings (Godot -> Rust)

use proc_macro2::Ident;

use crate::util::ident;

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Case conversions

fn to_snake_special_case(class_name: &str) -> Option<&'static str> {
    match class_name {
        // Classes
        "JSONRPC" => Some("json_rpc"),
        "OpenXRAPIExtension" => Some("open_xr_api_extension"),
        "OpenXRIPBinding" => Some("open_xr_ip_binding"),

        // Enums
        "SDFGIYScale" => Some("sdfgi_y_scale"),
        "VSyncMode" => Some("vsync_mode"),
        _ => None,
    }
}

pub fn to_snake_case(class_name: &str) -> String {
    use heck::ToSnakeCase;

    // Special cases
    if let Some(special_case) = to_snake_special_case(class_name) {
        return special_case.to_string();
    }

    class_name
        .replace("1D", "_1d") // e.g. animation_node_blend_space_1d
        .replace("2D", "_2d")
        .replace("3D", "_3d")
        .replace("GDNative", "Gdnative")
        .replace("GDExtension", "Gdextension")
        .to_snake_case()
}

pub fn to_pascal_case(class_name: &str) -> String {
    use heck::ToPascalCase;

    // Special cases: reuse snake_case impl to ensure at least consistency between those 2.
    if let Some(snake_special) = to_snake_special_case(class_name) {
        return snake_special.to_pascal_case();
    }

    class_name
        .to_pascal_case()
        .replace("GdExtension", "GDExtension")
        .replace("GdNative", "GDNative")
}

pub fn shout_to_pascal(shout_case: &str) -> String {
    // TODO use heck?

    let mut result = String::with_capacity(shout_case.len());
    let mut next_upper = true;

    for ch in shout_case.chars() {
        if next_upper {
            assert_ne!(ch, '_'); // no double underscore
            result.push(ch); // unchanged
            next_upper = false;
        } else if ch == '_' {
            next_upper = true;
        } else {
            result.push(ch.to_ascii_lowercase());
        }
    }

    result
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Enum conversions

pub fn make_enum_name(enum_name: &str) -> Ident {
    ident(&to_pascal_case(enum_name))
}

pub fn make_enumerator_name(enumerator_name: &str, _enum_name: &str) -> Ident {
    // TODO strip prefixes of `enum_name` appearing in `enumerator_name`
    // tons of variantions, see test cases in lib.rs

    ident(enumerator_name)
}
