/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! Functions shared between various built-in tests.

use godot::builtin::VariantOperator;
use godot::meta::{FromGodot, ToGodot};
use godot::private::class_macros::assert_eq_approx;

/// Asserts that result of evaluated operation via variants and expected one are approximately equal.
///
/// Used to check if operations performed in Godot Rust yield the same result as ones performed via Godot runtime.
pub(crate) fn assert_evaluate_approx_eq<T, U, E>(
    left: T,
    right: U,
    op: VariantOperator,
    expected: E,
) where
    T: ToGodot,
    U: ToGodot,
    E: FromGodot + std::fmt::Debug + godot::builtin::math::ApproxEq + Copy,
{
    let lhs = left
        .to_variant()
        .evaluate(&right.to_variant(), op)
        .expect("Result of evaluation can't be null!")
        .to::<E>();

    assert_eq_approx!(lhs, expected);
}
