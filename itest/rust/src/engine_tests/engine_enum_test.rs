/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use godot::classes::{mesh, window};
use godot::global::{InlineAlignment, Key, KeyModifierMask, Orientation};
use godot::obj::{EngineBitfield, EngineEnum};

use crate::framework::itest;

#[itest]
fn enum_with_masked_bitfield_ord() {
    let key = Key::A;
    let mask = KeyModifierMask::SHIFT;

    assert_eq!(key.ord(), 65);
    assert_eq!(mask.ord(), 1 << 25);

    let shifted_key = key | mask;
    assert_eq!(shifted_key.ord(), 65 | (1 << 25));

    let shifted_key = mask | key;
    assert_eq!(shifted_key.ord(), 65 | (1 << 25));

    let mut key = Key::A;
    key |= KeyModifierMask::SHIFT;
    assert_eq!(key.ord(), 65 | (1 << 25));
}

#[itest]
fn enum_with_masked_bitfield_from_ord() {
    let shifted_key = Key::A | KeyModifierMask::SHIFT;
    let ord = shifted_key.ord();
    assert_eq!(ord, 65 | (1 << 25));

    let back = Key::try_from_ord(ord);
    assert_eq!(back, Some(shifted_key), "deserialize bitmasked enum");

    let back = Key::from_ord(ord);
    assert_eq!(back, shifted_key, "deserialize bitmasked enum");

    // For random values that are *not* a valid enum|mask combination, try_from_ord() should fail.
    // Not implemented, as it's hard to achieve this without breaking forward compatibility; see make_enum_engine_trait_impl().
    // let back = Key::try_from_ord(31);
    // assert_eq!(back, None, "don't deserialize invalid bitmasked enum");
}

#[itest]
fn enum_values_class() {
    let expected_modes = [
        window::Mode::WINDOWED,
        window::Mode::MINIMIZED,
        window::Mode::MAXIMIZED,
        window::Mode::FULLSCREEN,
        window::Mode::EXCLUSIVE_FULLSCREEN,
    ];

    assert_eq!(window::Mode::values(), &expected_modes);
}

#[itest]
fn enum_values_global() {
    let expected_orientations = [Orientation::VERTICAL, Orientation::HORIZONTAL];

    assert_eq!(Orientation::values(), &expected_orientations);
}

#[itest]
fn enum_values_duplicates() {
    // InlineAlignment has many duplicate ordinals, but values() should return only distinct ones
    // The order matches the declaration order in the JSON API, not ordinal order
    let expected = [
        (InlineAlignment::TOP_TO, 0, true),
        (InlineAlignment::CENTER_TO, 1, true),
        (InlineAlignment::BASELINE_TO, 3, true),
        (InlineAlignment::BOTTOM_TO, 2, true),
        (InlineAlignment::TO_TOP, 0, false), // duplicate of TOP_TO
        (InlineAlignment::TO_CENTER, 4, true),
        (InlineAlignment::TO_BASELINE, 8, true),
        (InlineAlignment::TO_BOTTOM, 12, true),
        (InlineAlignment::TOP, 0, false), // duplicate of TOP_TO
        (InlineAlignment::CENTER, 5, true),
        (InlineAlignment::BOTTOM, 14, true),
        (InlineAlignment::IMAGE_MASK, 3, false), // duplicate of BASELINE_TO
        (InlineAlignment::TEXT_MASK, 12, false), // duplicate of TO_BOTTOM
    ];

    let all_constants = InlineAlignment::all_constants();
    let mut expected_distinct_values = vec![];
    for ((value, ord, is_distinct), c) in expected.into_iter().zip(all_constants) {
        if is_distinct {
            assert_eq!(c.rust_name(), value.as_str()); // First distinct.
            expected_distinct_values.push(value);
        }

        assert_eq!(c.value(), value);
        assert_eq!(c.value().ord(), ord);
    }

    assert_eq!(InlineAlignment::values(), &expected_distinct_values);

    // Some known duplicates.
    assert_eq!(InlineAlignment::TOP_TO, InlineAlignment::TO_TOP); // ord 0
    assert_eq!(InlineAlignment::TOP_TO, InlineAlignment::TOP); // ord 0
    assert_eq!(InlineAlignment::BASELINE_TO, InlineAlignment::IMAGE_MASK); // ord 3
    assert_eq!(InlineAlignment::TO_BOTTOM, InlineAlignment::TEXT_MASK); // ord 12
}

#[itest]
fn enum_values_max_excluded() {
    let expected_array_types = [
        mesh::ArrayType::VERTEX,
        mesh::ArrayType::NORMAL,
        mesh::ArrayType::TANGENT,
        mesh::ArrayType::COLOR,
        mesh::ArrayType::TEX_UV,
        mesh::ArrayType::TEX_UV2,
        mesh::ArrayType::CUSTOM0,
        mesh::ArrayType::CUSTOM1,
        mesh::ArrayType::CUSTOM2,
        mesh::ArrayType::CUSTOM3,
        mesh::ArrayType::BONES,
        mesh::ArrayType::WEIGHTS,
        mesh::ArrayType::INDEX,
    ];

    let array_types = mesh::ArrayType::values();
    assert_eq!(array_types, &expected_array_types);
    assert!(
        !array_types.contains(&mesh::ArrayType::MAX),
        "ArrayType::MAX should be excluded from values()"
    );

    // However, it should still be present in all_constants().
    let all_constants = mesh::ArrayType::all_constants();
    assert!(
        all_constants
            .iter()
            .any(|c| c.value() == mesh::ArrayType::MAX),
        "ArrayType::MAX should be present in all_constants()"
    );
}

#[itest]
fn enum_all_constants() {
    let constants = InlineAlignment::all_constants();
    assert!(
        constants.len() > InlineAlignment::values().len(),
        "all_constants() should include duplicates"
    );

    // Check one known constant.
    let first = constants[0];
    assert_eq!(first.rust_name(), "TOP_TO");
    assert_eq!(first.godot_name(), "INLINE_ALIGNMENT_TOP_TO");
    assert_eq!(first.value(), InlineAlignment::TOP_TO);
    assert_eq!(first.value().ord(), 0);

    // Check specific constants at known indices, with equal ordinals.
    let known_a = constants[2];
    let known_b = constants[11];

    assert_eq!(known_a.rust_name(), "BASELINE_TO");
    assert_eq!(known_a.godot_name(), "INLINE_ALIGNMENT_BASELINE_TO");
    assert_eq!(known_a.value(), InlineAlignment::BASELINE_TO);
    assert_eq!(known_a.value().ord(), 3);

    assert_eq!(known_b.rust_name(), "IMAGE_MASK");
    assert_eq!(known_b.godot_name(), "INLINE_ALIGNMENT_IMAGE_MASK");
    assert_eq!(known_b.value(), InlineAlignment::IMAGE_MASK);
    assert_eq!(known_b.value().ord(), 3);

    // "Front-end" values are equal, too.
    assert_eq!(
        InlineAlignment::IMAGE_MASK.ord(),
        InlineAlignment::BASELINE_TO.ord()
    );
}

#[itest]
fn bitfield_all_constants() {
    let shift_constant = KeyModifierMask::all_constants()
        .iter()
        .find(|c| c.rust_name() == "SHIFT")
        .expect("SHIFT constant should exist");

    assert_eq!(shift_constant.godot_name(), "KEY_MASK_SHIFT");
    assert_eq!(shift_constant.value(), KeyModifierMask::SHIFT);
    assert_eq!(shift_constant.value().ord(), 1 << 25);
}
