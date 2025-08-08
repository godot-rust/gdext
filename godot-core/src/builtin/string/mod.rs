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

use std::ops;

pub use gstring::*;
pub use node_path::NodePath;
pub use string_name::*;

use crate::meta::error::ConvertError;
use crate::meta::{FromGodot, GodotConvert, ToGodot};

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
// Encoding

/// Specifies string encoding.
///
/// Used in functions such as [`GString::try_from_bytes()`][GString::try_from_bytes] to handle multiple input string encodings.
#[non_exhaustive]
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum Encoding {
    Ascii,
    Latin1,
    Utf8,
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

/// Returns a tuple of `(from, len)` from a Rust range.
///
/// Unbounded upper bounds are represented by `len = -1`.
fn to_godot_fromlen_neg1<R>(range: R) -> (i64, i64)
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
            debug_assert!(
                from <= to,
                "range: start ({from}) > inclusive end ({n}) + 1"
            );
            to - from
        }
        ops::Bound::Excluded(&n) => {
            let to = n as i64;
            debug_assert!(from <= to, "range: start ({from}) > exclusive end ({to})");
            to - from
        }
        ops::Bound::Unbounded => -1,
    };

    (from, len)
}

/// Returns a tuple of `(from, len)` from a Rust range.
///
/// Unbounded upper bounds are represented by `i32::MAX` (yes, not `i64::MAX` -- since Godot treats some indexes as 32-bit despite being
/// declared `i64` in GDExtension API).
fn to_godot_fromlen_i32max<R>(range: R) -> (i64, i64)
where
    R: ops::RangeBounds<usize>,
{
    let (from, len) = to_godot_fromlen_neg1(range);
    if len == -1 {
        // Use i32 here because Godot may wrap around larger values (see Rustdoc).
        (from, i32::MAX as i64)
    } else {
        (from, len)
    }
}

/// Returns a tuple of `(from, to)` from a Rust range.
///
/// Unbounded upper bounds are represented by `to = 0`.
fn to_godot_fromto<R>(range: R) -> (i64, i64)
where
    R: ops::RangeBounds<usize>,
{
    let (from, len) = to_godot_fromlen_neg1(range);
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

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Padding, alignment and precision support

// Used by sub-modules of this module.
use standard_fmt::pad_if_needed;

mod standard_fmt {
    use std::fmt;
    use std::fmt::Write;

    pub fn pad_if_needed<F>(f: &mut fmt::Formatter<'_>, display_impl: F) -> fmt::Result
    where
        F: Fn(&mut fmt::Formatter<'_>) -> fmt::Result,
    {
        let needs_format = f.width().is_some() || f.precision().is_some() || f.align().is_some();

        // Early exit if no custom formatting is needed.
        if !needs_format {
            return display_impl(f);
        }

        let ic = FmtInterceptor { display_impl };

        let mut local_str = String::new();
        write!(&mut local_str, "{ic}")?;
        f.pad(&local_str)
    }

    struct FmtInterceptor<F>
    where
        F: Fn(&mut fmt::Formatter<'_>) -> fmt::Result,
    {
        display_impl: F,
    }

    impl<F> fmt::Display for FmtInterceptor<F>
    where
        F: Fn(&mut fmt::Formatter<'_>) -> fmt::Result,
    {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            (self.display_impl)(f)
        }
    }
}
