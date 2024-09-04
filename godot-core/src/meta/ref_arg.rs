/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
use crate::meta::error::ConvertError;
use crate::meta::{FromGodot, GodotConvert, ToGodot};
use std::fmt;

pub struct RefArg<'a, T> {
    pub(crate) shared_ref: &'a T,
}

impl<'a, T> RefArg<'a, T> {
    pub fn new(shared_ref: &'a T) -> Self {
        RefArg { shared_ref }
    }
}

impl<'a, T> GodotConvert for RefArg<'a, T>
where
    T: GodotConvert,
{
    type Via = T::Via;
}

impl<'a, T> ToGodot for RefArg<'a, T>
where
    T: ToGodot,
{
    fn to_godot(&self) -> T::Via {
        self.shared_ref.to_godot()
    }
}

// TODO refactor signature tuples into separate in+out traits, so FromGodot is no longer needed.
impl<'a, T> FromGodot for RefArg<'a, T>
where
    T: FromGodot,
{
    fn try_from_godot(_via: Self::Via) -> Result<Self, ConvertError> {
        unreachable!("RefArg should only be passed *to* Godot, not *from*.")
    }
}

impl<'a, T> fmt::Debug for RefArg<'a, T>
where
    T: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "&{:?}", self.shared_ref)
    }
}
