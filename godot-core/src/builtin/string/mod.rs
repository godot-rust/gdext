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
mod string_name;

use crate::meta::error::ConvertError;
use crate::meta::{FromGodot, GodotConvert, ToGodot};
use std::ops;

pub use gstring::*;
pub use node_path::NodePath;
pub use string_name::{StringName, TransientStringNameOrd};

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
fn to_fromlen_pair<R>(range: R) -> (i64, i64)
where
    R: ops::RangeBounds<usize>,
{
    let from = match range.start_bound() {
        ops::Bound::Included(&n) => n as i64,
        ops::Bound::Excluded(&n) => (n as i64) + 1,
        ops::Bound::Unbounded => 0,
    };

    let len = match range.end_bound() {
        ops::Bound::Included(&n) => ((n + 1) as i64) - from,
        ops::Bound::Excluded(&n) => (n as i64) - from,
        ops::Bound::Unbounded => -1,
    };

    (from, len)
}

fn populated_or_none(s: GString) -> Option<GString> {
    if s.is_empty() {
        None
    } else {
        Some(s)
    }
}
