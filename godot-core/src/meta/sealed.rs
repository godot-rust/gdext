/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! Sealed trait that can be used to restrict trait impls.

// To ensure the user does not implement `GodotType` for their own types.
use crate::builtin::*;
use crate::meta;
use crate::meta::traits::{ArrayElement, GodotNullableFfi, GodotType};
use crate::obj::{DynGd, Gd, GodotClass, RawGd};

pub trait Sealed {}
impl Sealed for Aabb {}
impl Sealed for Basis {}
impl Sealed for Callable {}
impl Sealed for Vector2 {}
impl Sealed for Vector3 {}
impl Sealed for Vector4 {}
impl Sealed for Vector2i {}
impl Sealed for Vector3i {}
impl Sealed for Vector4i {}
impl Sealed for Quaternion {}
impl Sealed for Color {}
impl Sealed for GString {}
impl Sealed for StringName {}
impl Sealed for NodePath {}
impl Sealed for PackedByteArray {}
impl Sealed for PackedInt32Array {}
impl Sealed for PackedInt64Array {}
impl Sealed for PackedFloat32Array {}
impl Sealed for PackedFloat64Array {}
impl Sealed for PackedStringArray {}
impl Sealed for PackedVector2Array {}
impl Sealed for PackedVector3Array {}
#[cfg(since_api = "4.3")] #[cfg_attr(published_docs, doc(cfg(since_api = "4.3")))]
impl Sealed for PackedVector4Array {}
impl Sealed for PackedColorArray {}
impl Sealed for Plane {}
impl Sealed for Projection {}
impl Sealed for Rid {}
impl Sealed for Rect2 {}
impl Sealed for Rect2i {}
impl Sealed for Signal {}
impl Sealed for Transform2D {}
impl Sealed for Transform3D {}
impl Sealed for Dictionary {}
impl Sealed for bool {}
impl Sealed for i64 {}
impl Sealed for i32 {}
impl Sealed for i16 {}
impl Sealed for i8 {}
impl Sealed for u64 {}
impl Sealed for u32 {}
impl Sealed for u16 {}
impl Sealed for u8 {}
impl Sealed for f64 {}
impl Sealed for f32 {}
impl Sealed for () {}
impl Sealed for Variant {}
impl<T: ArrayElement> Sealed for Array<T> {}
impl<T: GodotClass> Sealed for Gd<T> {}
impl<T: GodotClass> Sealed for RawGd<T> {}
impl<T: GodotClass, D: ?Sized> Sealed for DynGd<T, D> {}
impl<T: GodotClass> Sealed for meta::ObjectArg<T> {}
impl<T> Sealed for Option<T>
where
    T: GodotType,
    T::Ffi: GodotNullableFfi,
{
}
