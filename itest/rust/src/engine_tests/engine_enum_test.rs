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
fn enum_with_masked_bitfield() {
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
