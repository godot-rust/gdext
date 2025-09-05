/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::fmt::{Debug, Display};
use std::ops;

/// Trait easing converting Rust ranges to values expected by Godot.
///
/// Note: Unbounded upper bounds must be represented by `i32::MAX` instead of `i64::MAX`,
/// since Godot treats some indexes as 32-bit despite being declared `i64` in GDExtension API.
pub(crate) trait GodotRange<T> {
    /// Returns a tuple of `(from, to)` from a Rust range.
    fn to_godot_range_fromto(&self) -> (i64, Option<i64>);

    /// Returns a tuple of `(from, to)` from a Rust range.
    ///
    /// Unbounded upper bound will be represented by `from = default_unbounded_upper`.
    ///
    /// # Panics
    /// In debug mode, when `from` > `to`.
    fn to_godot_range_fromto_checked(&self, default_unbounded_upper: i64) -> (i64, i64) {
        match self.to_godot_range_fromto() {
            (from, Some(to)) => {
                debug_assert!(from <= to, "range: start ({from}) > end ({to})");
                (from, to)
            }
            (from, None) => (from, default_unbounded_upper),
        }
    }

    /// Returns a tuple of `(from, len)` from a Rust range.
    ///
    /// Unbounded upper bounds are represented by `len = default_unbounded_upper`.
    ///
    /// # Panics
    /// In debug mode, when `from > to` (i.e. `len < 0`).
    fn to_godot_range_fromlen(&self, default_unbounded_upper: i64) -> (i64, i64) {
        match self.to_godot_range_fromto() {
            (from, Some(to)) => {
                debug_assert!(from <= to, "range: start ({from}) > end ({to})");
                (from, to - from)
            }
            (from, None) => (from, default_unbounded_upper),
        }
    }
}

// Blanket implementation for any range which can be converted to (i64, i64).
// Supports both `RangeBounds<usize>` (e.g. `GString::count(..)`) and `RangeBounds<i32>` (e.g. `array.subarray_deep(-1..-5..)`) .
// `RangeBounds<usize>` should be used for ranges which can't/shouldn't be negative.
// `RangeBounds<i32>` for any range which supports negative indices (mostly methods related to Array).
impl<T, R> GodotRange<T> for R
where
    R: ops::RangeBounds<T>,
    i64: TryFrom<T>,
    T: Copy + Display,
    <T as TryInto<i64>>::Error: Debug,
{
    fn to_godot_range_fromto(&self) -> (i64, Option<i64>) {
        let from = match self.start_bound() {
            ops::Bound::Included(&n) => i64::try_from(n).unwrap(),
            ops::Bound::Excluded(&n) => i64::try_from(n).unwrap() + 1,
            ops::Bound::Unbounded => 0,
        };

        let to = match self.end_bound() {
            ops::Bound::Included(&n) => {
                let to = i64::try_from(n).unwrap() + 1;
                Some(to)
            }
            ops::Bound::Excluded(&n) => {
                let to = i64::try_from(n).unwrap();
                Some(to)
            }
            ops::Bound::Unbounded => None,
        };

        (from, to)
    }
}
