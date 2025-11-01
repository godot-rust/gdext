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

pub use gstring::*;
pub use node_path::NodePath;
pub use string_name::*;

use crate::meta;
use crate::meta::error::ConvertError;
use crate::meta::{FromGodot, GodotConvert, ToGodot};

impl GodotConvert for &str {
    type Via = GString;
}

impl ToGodot for &str {
    type Pass = meta::ByValue;

    fn to_godot(&self) -> Self::Via {
        GString::from(*self)
    }
}

impl GodotConvert for String {
    type Via = GString;
}

impl ToGodot for String {
    type Pass = meta::ByValue;

    fn to_godot(&self) -> Self::Via {
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
// Utilities

fn populated_or_none(s: GString) -> Option<GString> {
    if s.is_empty() {
        None
    } else {
        Some(s)
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
