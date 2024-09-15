/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
use crate::meta::error::ConvertError;
use crate::meta::{FromGodot, GodotConvert, ToGodot};
use std::fmt;

pub struct RefArg<'r, T> {
    pub(crate) shared_ref: &'r T,
}

impl<'r, T> RefArg<'r, T> {
    pub fn new(shared_ref: &'r T) -> Self {
        RefArg { shared_ref }
    }
}

impl<'r, T> GodotConvert for RefArg<'r, T>
where
    T: GodotConvert,
{
    type Via = T::Via;
}

impl<'r, T> ToGodot for RefArg<'r, T>
where
    T: ToGodot,
{
    type ToVia<'v> = Self::Via
    where Self: 'v;

    fn to_godot(&self) -> Self::ToVia<'_> {
        self.shared_ref.to_godot()
    }
}

// TODO refactor signature tuples into separate in+out traits, so FromGodot is no longer needed.
impl<'r, T> FromGodot for RefArg<'r, T>
where
    T: FromGodot,
{
    fn try_from_godot(_via: Self::Via) -> Result<Self, ConvertError> {
        unreachable!("RefArg should only be passed *to* Godot, not *from*.")
    }
}

impl<'r, T> fmt::Debug for RefArg<'r, T>
where
    T: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "&{:?}", self.shared_ref)
    }
}
