/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::obj::{Gd, GodotClass, WithBaseField};

/// Obtain owned `Gd` from either `&self` or `&Gd`.
///
/// This trait allows passing either `Gd<T>` or `C` (where `C: WithBaseField`) to functions that need an owned `Gd<T>`.
///
/// This is primarily used for signal connection methods in [`TypedSignal`][crate::registry::signal::TypedSignal] and
/// [`ConnectBuilder`][crate::registry::signal::ConnectBuilder], where you can pass either a `&Gd` (outside) or `&SomeClass`
/// (from within `impl` block) as the receiver object.
///
/// # Similar traits
/// - [`UniformObjectDeref`][crate::meta::UniformObjectDeref] provides unified dereferencing of user and engine classes.
/// - [`AsArg`][crate::meta::AsArg] enables general argument conversions for Godot APIs.
pub trait ObjectToOwned<T: GodotClass> {
    /// Converts the object reference to an owned `Gd<T>`.
    fn object_to_owned(&self) -> Gd<T>;
}

impl<T: GodotClass> ObjectToOwned<T> for Gd<T> {
    fn object_to_owned(&self) -> Gd<T> {
        self.clone()
    }
}

impl<C: WithBaseField> ObjectToOwned<C> for C {
    fn object_to_owned(&self) -> Gd<C> {
        WithBaseField::to_gd(self)
    }
}
