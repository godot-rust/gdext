/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! Godot-types that are Strings.

mod gstring;
mod macros;
mod node_path;
mod string_macros;
mod string_name;

use crate::meta::error::ConvertError;
use crate::meta::{FromGodot, GodotConvert, ToGodot};
use std::ops;

pub use gstring::*;
pub use node_path::NodePath;
pub use string_name::*;

impl GodotConvert for &str {
    type Via = GString;
}

impl ToGodot for &str {
    type ToVia<'v>
        = GString
    where
        Self: 'v;

    fn to_godot(&self) -> Self::ToVia<'_> {
        GString::from(*self)
    }
}

impl GodotConvert for String {
    type Via = GString;
}

impl ToGodot for String {
    type ToVia<'v> = Self::Via;

    fn to_godot(&self) -> Self::ToVia<'_> {
        GString::from(self)
    }
}

impl FromGodot for String {
    fn try_from_godot(via: Self::Via) -> Result<Self, ConvertError> {
        Ok(via.to_string())
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

/// Returns a tuple of `(from, len)` from a Rust range.
///
/// Unbounded upper bounds are represented by `len = -1`.
fn to_godot_fromlen<R>(range: R) -> (i64, i64)
where
    R: ops::RangeBounds<usize>,
{
    let from = match range.start_bound() {
        ops::Bound::Included(&n) => n as i64,
        ops::Bound::Excluded(&n) => (n as i64) + 1,
        ops::Bound::Unbounded => 0,
    };

    let len = match range.end_bound() {
        ops::Bound::Included(&n) => {
            let to = (n + 1) as i64;
            debug_assert!(to >= from, "range: start ({from}) > inclusive end ({to})");
            to - from
        }
        ops::Bound::Excluded(&n) => {
            let to = n as i64;
            debug_assert!(to > from, "range: start ({from}) >= exclusive end ({to})");
            to - from
        }
        ops::Bound::Unbounded => -1,
    };

    (from, len)
}

/// Returns a tuple of `(from, to)` from a Rust range.
///
/// Unbounded upper bounds are represented by `to = 0`.
fn to_godot_fromto<R>(range: R) -> (i64, i64)
where
    R: ops::RangeBounds<usize>,
{
    let (from, len) = to_godot_fromlen(range);
    if len == -1 {
        (from, 0)
    } else {
        (from, from + len)
    }
}

fn populated_or_none(s: GString) -> Option<GString> {
    if s.is_empty() {
        None
    } else {
        Some(s)
    }
}

fn found_to_option(index: i64) -> Option<usize> {
    if index == -1 {
        None
    } else {
        // If this fails, then likely because we overlooked a negative value.
        let index_usize = index
            .try_into()
            .unwrap_or_else(|_| panic!("unexpected index {index} returned from Godot function"));

        Some(index_usize)
    }
}
