/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::framework::itest;

use godot::global::{Key, KeyModifierMask};
use godot::obj::{EngineBitfield, EngineEnum};

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
