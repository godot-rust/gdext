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
pub fn to_snake_case(ty_name: &str) -> String {
    use heck::ToSnakeCase;

    assert!(
        is_valid_ident(ty_name),
        "invalid identifier for snake_case conversion: {ty_name}"
    );

    // Special cases
    if let Some(special_case) = to_snake_special_case(ty_name) {
        return special_case.to_string();
    }

    ty_name
        .replace("1D", "_1d") // e.g. animation_node_blend_space_1d
        .replace("2D", "_2d")
        .replace("3D", "_3d")
        .replace("GDNative", "Gdnative")
        .replace("GDExtension", "Gdextension")
        .replace("GDScript", "Gdscript")
        .replace("VSync", "Vsync")
        .replace("SDFGIY", "SdfgiY")
        .replace("ENet", "Enet")
        .to_snake_case()
}

/// Used for `PascalCase` identifiers: classes and enums.
pub fn to_pascal_case(ty_name: &str) -> String {
    use heck::ToPascalCase;

    assert!(
        is_valid_ident(ty_name),
        "invalid identifier for PascalCase conversion: {ty_name}"
    );

    // Special cases: reuse snake_case impl to ensure at least consistency between those 2.
    if let Some(snake_special) = to_snake_special_case(ty_name) {
        return snake_special.to_pascal_case();
    }

    ty_name
        .to_pascal_case()
        .replace("GdExtension", "GDExtension")
        .replace("GdNative", "GDNative")
        .replace("GdScript", "GDScript")
        .replace("Vsync", "VSync")
        .replace("Sdfgiy", "SdfgiY")
}

#[allow(dead_code)] // Keep around in case we need it later.
pub fn shout_to_pascal(shout_case: &str) -> String {
    // TODO use heck?

    assert!(
        is_valid_shout_ident(shout_case),
        "invalid identifier for SHOUT_CASE -> PascalCase conversion: {shout_case}"
    );

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
// Virtual functions

pub fn make_unsafe_virtual_fn_name(rust_fn_name: &str) -> String {
    format!("{rust_fn_name}_rawptr")
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Enum conversions

pub fn make_enum_name(enum_name: &str) -> Ident {
    ident(&make_enum_name_str(enum_name))
}

pub fn make_enum_name_str(enum_name: &str) -> String {
    match enum_name {
        // Special cases with '.' in name.
        "Variant.Type" => "VariantType".to_string(),
        "Variant.Operator" => "VariantOperator".to_string(),
        e => to_pascal_case(e),
    }
}

/// Maps enumerator names from Godot to Rust, applying a best-effort heuristic.
///
/// Takes into account redundancies in the enumerator, mostly if it contains parts of the enum name. This avoids
/// repetition and leads to significantly shorter names, without losing information. `#[doc(alias)]` ensures that
/// the original name can still be found in API docs.
pub fn make_enumerator_names(
    godot_class_name: Option<&str>,
    godot_enum_name: &str,
    enumerators: Vec<&str>,
) -> Vec<Ident> {
    debug_assert_eq!(
        make_enum_name(godot_enum_name),
        godot_enum_name,
        "enum name must already be mapped"
    );

    shorten_enumerator_names(godot_class_name, godot_enum_name, enumerators)
        .iter()
        .map(|e| ident(e))
        .collect()
}

/// Finds a common prefix in all enumerators, that is then stripped.
///
/// This won't work if there is only one enumerator (there are some special cases for those).
///
/// `class_name` is the containing class, or `None` if it is a global enum.
pub fn shorten_enumerator_names<'e>(
    godot_class_name: Option<&str>,
    godot_enum_name: &str,
    enumerators: Vec<&'e str>,
) -> Vec<&'e str> {
    // Hardcoded exceptions.
    if let Some(prefixes) = reduce_hardcoded_prefix(godot_class_name, godot_enum_name) {
        // Remove prefix from every enumerator.
        return enumerators
            .iter()
            .map(|e| try_strip_prefixes(e, prefixes))
            .collect::<Vec<_>>();
    }

    if enumerators.len() <= 1 {
        return enumerators;
    }

    // Look for common prefix; start at everything before last underscore.
    let original = &enumerators[0];
    let Some((mut longest_prefix, mut pos)) = enumerator_prefix(original, enumerators[0].len())
    else {
        // If there is no underscore (i.e. enumerator consists of only one part), return that immediately.
        return enumerators;
    };

    // Go through remaining enumerators, shorten prefix until it is contained in all of them.
    'outer: for e in enumerators[1..].iter() {
        // if all enums should have common prefix, change condition:   ... || starts_with_invalid_char(&e[pos..])
        while !e.starts_with(longest_prefix) {
            // Find a shorter prefix...
            if let Some((prefix, new_pos)) = enumerator_prefix(original, pos - 1) {
                // Found: shorten prefix, rewind position.
                longest_prefix = prefix;
                pos = new_pos;
            } else {
                // Not found: there is no common prefix. We keep the original enumerators, exit here.
                pos = 0;
                break 'outer;
            }
        }
    }

    let pos = pos; // immutable.
    let last_index = enumerators.len() - 1;
    enumerators
        .into_iter()
        .enumerate()
        .map(|(i, e)| {
            // Special-case MAX constants which refer to enum size and should not be prefixed.
            // Examples: FftSize.SIZE_MAX (2x), EnvironmentSdfgiRayCount.COUNT_MAX, JointType.TYPE_MAX
            if e.ends_with("_MAX") && i == last_index {
                // TODO for enums, this could be an associated constants, as it's not part of the possible values.
                // Special case that act like a max: Curve.TangentMode.MODE_COUNT
                return "MAX";
            }

            // If an enumerator begins with a digit, include one more prefix part -- but only for that enumerator, not the whole group.
            let mut local_pos = pos;
            while starts_with_invalid_char(&e[local_pos..]) {
                // Move pos to the one '_' before its current position (or beginning), in order to include one part more.
                // 'while' instead of 'if' because previous part could also start with a digit.
                debug_assert!(local_pos > 0, "enumerator {e} starts with digit");

                // Example:     Source.SOURCE_3D_NORMAL -> 3D_NORMAL after prefix removal.
                // After rewind, again SOURCE_3D_NORMAL.
                local_pos = if let Some(new_pos) = e[..local_pos - 1].rfind('_') {
                    new_pos + 1
                } else {
                    0
                };
            }

            &e[local_pos..]
        })
        .collect()
}

/// Exceptions for enums, where the heuristic wrongly identifies common prefixes, or those are not intuitive.
///
/// Parameters:
/// - `class_name` is the containing class, or `None` if it is a global enum.
/// - `enum_name` is the name of the enum. All of its enums will be shortened according to the same prefix.
///
/// Returns:
/// - `None` if the heuristic should be used.
/// - `Some(prefix)` if the specified `prefix` should be removed from every enumerator. If it's missing, the enumerator is left as-is.
fn reduce_hardcoded_prefix(
    class_name: Option<&str>,
    enum_name: &str,
) -> Option<&'static [&'static str]> {
    let result: &[&str] = match (class_name, enum_name) {
        (None, "Key") => &["KEY_"],

        // Inconsistency with varying prefixes.
        (Some("RenderingServer" | "Mesh"), "ArrayFormat") => &["ARRAY_FORMAT_", "ARRAY_"],
        (Some("AudioServer"), "SpeakerMode") => &["SPEAKER_MODE_", "SPEAKER_"],
        (Some("ENetConnection"), "HostStatistic") => &["HOST_"], // do not remove TOTAL_ prefix (shared by all), e.g. TOTAL_SENT_DATA.
        (None, "MethodFlags") => &["METHOD_FLAG_", "METHOD_FLAGS_"],

        // There are two "mask" entries which span multiple bits: KEY_CODE_MASK, KEY_MODIFIER_MASK -> CODE_MASK, MODIFIER_MASK.
        // All other entries are one bit only, clarify this: KEY_MASK_CTRL -> CTRL.
        (None, "KeyModifierMask") => &["KEY_MASK_", "KEY_"],

        // Only 1 enumerator; algorithm can't detect common prefixes.
        (Some("RenderingDevice"), "StorageBufferUsage") => &["STORAGE_BUFFER_USAGE_"],
        (Some(_), "PathfindingAlgorithm") => &["PATHFINDING_ALGORITHM_"],

        // All others use heuristic.
        _ => return None,
    };

    Some(result)
}

// Could have signature:  try_strip_prefixes<'e>(enumerator: &'e str, prefixes: &[&str]) -> &'e str
// But not much point, result expects owned strings.
// fn try_strip_prefixes(enumerator: &str, prefixes: &[&str]) -> String {
fn try_strip_prefixes<'e>(enumerator: &'e str, prefixes: &[&str]) -> &'e str {
    // Try all prefixes in order, use full enumerator otherwise.
    for prefix in prefixes {
        if let Some(stripped) = enumerator.strip_prefix(prefix) {
            // If resulting enumerator is invalid, try next one
            if !starts_with_invalid_char(stripped) {
                return stripped;
            }
        }
    }

    // No prefix worked, use full enumerator.
    enumerator
}

/// Check if input is a valid identifier; i.e. no special characters except '_' and not starting with a digit.
fn is_valid_ident(s: &str) -> bool {
    !starts_with_invalid_char(s) && s.chars().all(|c| c == '_' || c.is_ascii_alphanumeric())
}

fn is_valid_shout_ident(s: &str) -> bool {
    !starts_with_invalid_char(s)
        && s.chars()
            .all(|c| c == '_' || c.is_ascii_digit() || c.is_ascii_uppercase())
}

fn starts_with_invalid_char(s: &str) -> bool {
    s.starts_with(|c: char| c.is_ascii_digit())
}

fn enumerator_prefix(original: &str, rfind_pos: usize) -> Option<(&str, usize)> {
    assert_ne!(rfind_pos, 0);

    // Find next underscore from the end, return prefix before that. pos+1 to include "_" in prefix.
    original[..rfind_pos]
        .rfind('_')
        .map(|pos| (&original[..pos + 1], pos + 1))
}
