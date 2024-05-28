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

/// Signal to the compiler that a value is used (to avoid optimization).
pub fn bench_used<T: Sized>(value: T) {
    // The following check would be used to prevent `()` arguments, ensuring that a value from the bench is actually going into the blackbox.
    // However, we run into this issue, despite no array being used: https://github.com/rust-lang/rust/issues/43408.
    //   error[E0401]: can't use generic parameters from outer function
    // sys::static_assert!(std::mem::size_of::<T>() != 0, "returned unit value in benchmark; make sure to use a real value");

    std::hint::black_box(value);
}
