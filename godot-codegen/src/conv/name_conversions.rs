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
        // "SDFGIYScale" => Some("sdfgi_y_scale"),
        _ => None,
    }
}

/// Used for `snake_case` identifiers: modules.
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
        .replace("VSync", "Vsync")
        .replace("SDFGIY", "SdfgiY")
        .to_snake_case()
}

/// Used for `PascalCase` identifiers: classes and enums.
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
        .replace("Vsync", "VSync")
        .replace("Sdfgiy", "SdfgiY")
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

/// Checks an enum name against possible replacements for parts of the enumerator.
///
/// Key: part of an enum name.
/// Value: part of an enumerator name.
///
/// If the enum contains the key, then the algorithm runs *in addition* with the replaced value.
const ENUM_REPLACEMENTS: &[(&str, &str)] = &[
    // Sorted alphabetically.
    // Note: ERROR -> ERR is explicitly excluded, because without prefix, ERROR.* cannot easily be differentiated from Error.OK/FAILED.
    ("CAMERA_2D", "CAMERA2D"), // Exception; most enumerators containing "1D/2D/3D" have "_" before it.
    ("COLOR_SPACE", "GRADIENT_COLOR_SPACE"), // class Gradient
    ("COMPARISON_TYPE", "CTYPE"),
    ("COMPRESSION", "COMPRESS"),
    ("CONDITION", "COND"),
    ("CUBE_MAP", "CUBEMAP"),
    ("ENVIRONMENT", "ENV"),
    ("FUNCTION", "FUNC"),
    ("G6DOF_JOINT_AXIS_FLAG", "G6DOF_JOINT_FLAG"),
    ("INTERPOLATION", "INTERPOLATE"),
    ("INTERPOLATION_MODE", "GRADIENT_INTERPOLATE"), // class Gradient
    ("OPERATION", "OP"),                            // enum PolyBooleanOperation
    ("OPERATOR", "OP"),
    ("PARAMETER", "PARAM"),
    ("POST_PROCESSING", "POSTPROCESSING"),
    ("PROCESS_THREAD_MESSAGES", "FLAG_PROCESS_THREAD"), // class Node
    ("QUALIFIER", "QUAL"),
    ("SHADER_PARAMETER_TYPE", "VAR_TYPE"),
    ("TRANSITION", "TRANS"),
    ("VIEWPORT_OCCLUSION_CULLING", "VIEWPORT_OCCLUSION"),
    ("VISIBLE_CHARACTERS_BEHAVIOR", "VC"),
];

pub fn make_enum_name(enum_name: &str) -> Ident {
    ident(&make_enum_name_str(enum_name))
}

pub fn make_enum_name_str(enum_name: &str) -> String {
    to_pascal_case(enum_name)
}

/// Maps enumerator names from Godot to Rust, applying a best-effort heuristic.
///
/// Takes into account redundancies in the enumerator, mostly if it contains parts of the enum name. This avoids
/// repetition and leads to significantly shorter names, without losing information. `#[doc(alias)]` ensures that
/// the original name can still be found in API docs.
pub fn make_enumerator_name(enumerator: &str, enum_name: &str) -> Ident {
    debug_assert_eq!(
        make_enum_name(enum_name),
        enum_name,
        "enum name must already be mapped"
    );

    ident(&transform_enumerator_name(enumerator, enum_name))
}

fn transform_enumerator_name(enumerator: &str, enum_name: &str) -> String {
    // Go through snake case, to ensure consistent mapping to underscore-based names. Don't use heck's to_shouty_snake_case() directly.
    let enum_upper = to_snake_case(enum_name).to_ascii_uppercase();

    // Enumerator contains enum or beginning parts of it as prefix:
    // * JointType::JOINT_TYPE_PIN -> PIN.
    let search = &enum_upper[..];

    // Enumerator "XR_" prefix is always implied by the surrounding class name:
    // * Class XRInterface - enums Capabilities, TrackingStatus, PlayAreaMode, EnvironmentBlendMode
    // * Class XRPose - enum TrackingConfidence
    let enumerator = enumerator.strip_prefix("XR_").unwrap_or(enumerator);
    // * Class OpenXRAction - enum ActionType
    let enumerator = enumerator.strip_prefix("OPENXR_").unwrap_or(enumerator);

    // Check if there are abbreviations, replace on first match (unlike that there are more...).
    for (original, replaced) in ENUM_REPLACEMENTS {
        if search.contains(original) {
            let replaced_enumerator = search.replace(original, replaced);
            let replaced = strip_enumerator_prefix(enumerator, &replaced_enumerator);

            // If this is already an improvement, return here.
            if replaced != enumerator {
                return replaced;
            }
        }
    }

    // Try without the first part of the enum name:
    // * ZipAppend::APPEND_CREATE -> CREATE
    // * ProcessInfo::INFO_COLLISION_PAIRS -> COLLISION_PAIRS
    const NUM_REPETITIONS: usize = 2;
    if let Some(mut sep_index) = search.find('_') {
        for _ in 0..NUM_REPETITIONS {
            let remain = &search[sep_index + 1..];
            let replaced = strip_enumerator_prefix(enumerator, remain);

            if replaced != enumerator {
                return replaced;
            }

            if let Some(next_sep) = remain.find('_') {
                sep_index += next_sep + 1;
            } else {
                break;
            }
        }
    }

    strip_enumerator_prefix(enumerator, search)
}

fn valid_ident(ident: &str) -> Option<&str> {
    let Some(ident) = ident.strip_prefix('_') else {
        return None;
    };

    // Not empty and starts with alpha.
    let is_valid = ident
        .chars()
        .next()
        .map_or(false, |c| c.is_ascii_alphabetic());

    is_valid.then_some(ident)
}

/// Split a string `SOME_ENUMERATOR_VALUE` into its parts, return remainder of prefix-matching `search` with some variations.
fn strip_enumerator_prefix(enumerator: &str, mut search: &str) -> String {
    loop {
        if let Some(remain) = enumerator.strip_prefix(search) {
            // If already exhausted, leave the enumerator as is.
            if let Some(remain) = valid_ident(remain) {
                return remain.to_string();
            }
        }

        // Plural: Hands::HAND_LEFT -> LEFT
        if let Some(singular) = search.strip_suffix('S') {
            if let Some(remain) = enumerator.strip_prefix(singular) {
                if let Some(remain) = valid_ident(remain) {
                    return remain.to_string();
                }
            }
        }

        let Some(sep) = search.rfind('_') else { break };
        search = &search[..sep];
    }

    enumerator.to_string()
}
