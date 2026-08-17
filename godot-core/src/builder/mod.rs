/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::marker::PhantomData;

use crate::obj::GodotClass;

/// Class builder to store state for registering a class with Godot.
///
/// In the future this will be used, but for now it's a dummy struct.
pub struct ClassBuilder<C> {
    _c: PhantomData<C>,
}

impl<C> ClassBuilder<C>
where
    C: GodotClass,
{
    pub(crate) fn new() -> Self {
        Self { _c: PhantomData }
    }
}
