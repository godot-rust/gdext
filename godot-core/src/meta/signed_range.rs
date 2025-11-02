/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::collections::Bound;
use std::ops::RangeBounds;

/// Range which can be wrapped.
///
/// Constructed with [`wrapped()`] utility function.
struct WrappedRange {
    lower_bound: i64,
    upper_bound: Option<i64>,
}

/// Accepts negative bounds, interpreted relative to the end of the collection.
///
/// ## Examples
/// ```no_run
/// # use godot::meta::wrapped;
/// wrapped(1..-2); // from 1 to len-2
/// wrapped(..-2);  // from 0 to len-2
/// wrapped(-3..);  // from len-3 to end
/// wrapped(-4..3); // from len-4 to 3
/// ```
pub fn wrapped<T>(signed_range: impl RangeBounds<T>) -> impl SignedRange
where
    T: Copy + Into<i64>,
{
    let lower_bound = lower_bound(signed_range.start_bound().map(|v| (*v).into())).unwrap_or(0);
    let upper_bound = upper_bound(signed_range.end_bound().map(|v| (*v).into()));

    WrappedRange {
        lower_bound,
        upper_bound,
    }
}

fn lower_bound(bound: Bound<i64>) -> Option<i64> {
    match bound {
        Bound::Included(n) => Some(n),
        Bound::Excluded(n) => Some(n + 1),
        Bound::Unbounded => None,
    }
}

fn upper_bound(bound: Bound<i64>) -> Option<i64> {
    match bound {
        Bound::Included(n) => Some(n + 1),
        Bound::Excluded(n) => Some(n),
        Bound::Unbounded => None,
    }
}

mod sealed {
    pub trait SealedRange {}
}

/// Trait supporting regular `usize` ranges, as well as negative indices.
///
/// If a lower or upper bound is negative, then its value is relative to the end of the given collection.  \
/// Use the [`wrapped()`] utility function to construct such ranges.
pub trait SignedRange: sealed::SealedRange {
    /// Returns a tuple of `(from, to)` from a Rust range.
    /// Unbounded upper range is represented by `None`.
    // Note: in some cases unbounded upper bounds should be represented by `i32::MAX` instead of `i64::MAX`,
    // since Godot treats some indexes as 32-bit despite being declared as `i64` in GDExtension API.
    #[doc(hidden)]
    fn signed(&self) -> (i64, Option<i64>);
}

impl sealed::SealedRange for WrappedRange {}
impl SignedRange for WrappedRange {
    fn signed(&self) -> (i64, Option<i64>) {
        (self.lower_bound, self.upper_bound)
    }
}

impl<R> sealed::SealedRange for R where R: RangeBounds<usize> {}
impl<R> SignedRange for R
where
    R: RangeBounds<usize>,
{
    fn signed(&self) -> (i64, Option<i64>) {
        let lower_bound = lower_bound(self.start_bound().map(|v| *v as i64)).unwrap_or(0);
        let upper_bound = upper_bound(self.end_bound().map(|v| *v as i64));

        (lower_bound, upper_bound)
    }
}

/// Returns a tuple of `(from, to)` from a Rust range.
///
/// # Panics (safeguards-strict)
/// When `from` > `to`.
pub(crate) fn to_godot_range_fromto(range: impl SignedRange) -> (i64, i64) {
    match range.signed() {
        (from, Some(to)) => {
            crate::sys::strict_assert!(from <= to, "range: start ({from}) > end ({to})");
            (from, to)
        }
        (from, None) => (from, 0),
    }
}

/// Returns a tuple of `(from, len)` from a Rust range.
///
/// # Panics
/// In debug mode, when from > to.
pub(crate) fn to_godot_range_fromlen(range: impl SignedRange, unbounded: i64) -> (i64, i64) {
    match range.signed() {
        (from, Some(to)) => {
            crate::sys::strict_assert!(from <= to, "range: start ({from}) > end ({to})");
            (from, to - from)
        }
        (from, None) => (from, unbounded),
    }
}
