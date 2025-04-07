/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

/// Applying inverse transforms.
///
/// See also: [`Transform2D`](crate::builtin::Transform2D), [`Transform3D`](crate::builtin::Transform3D), [`Basis`](crate::builtin::Basis).
///
/// _Godot equivalent: `rhs * mat`_
pub trait XformInv<T>: std::ops::Mul<T> {
    fn xform_inv(&self, rhs: T) -> T;
}
