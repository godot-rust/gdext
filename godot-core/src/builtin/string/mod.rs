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
