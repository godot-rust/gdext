/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::fmt::Debug;

use godot::meta::{FromGodot, ToGodot};

pub fn roundtrip<T>(value: T)
where
    T: FromGodot + ToGodot + PartialEq + Debug,
{
    // TODO test other roundtrip (first FromGodot, then ToGodot)
    // Some values can be represented in Variant, but not in T (e.g. Variant(0i64) -> Option<InstanceId> -> Variant is lossy)

    let variant = value.to_variant();
    let back = T::try_from_variant(&variant).unwrap();

    assert_eq!(value, back);
}
