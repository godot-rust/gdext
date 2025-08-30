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
impl Sealed for Vector2Axis {}
impl Sealed for Vector3Axis {}
impl Sealed for Vector4Axis {}
impl Sealed for Quaternion {}
impl Sealed for Color {}
impl Sealed for GString {}
impl Sealed for StringName {}
impl Sealed for NodePath {}
// Generic implementation for all PackedArray<T> types.
use crate::builtin::PackedArray;

// Implement Sealed for the generic PackedArray<T> type.
impl<T: meta::PackedArrayElement> Sealed for PackedArray<T> {}
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
impl<T1> Sealed for (T1,) {}
impl<T1, T2> Sealed for (T1, T2) {}
impl<T1, T2, T3> Sealed for (T1, T2, T3) {}
impl<T1, T2, T3, T4> Sealed for (T1, T2, T3, T4) {}
impl<T1, T2, T3, T4, T5> Sealed for (T1, T2, T3, T4, T5) {}
impl<T1, T2, T3, T4, T5, T6> Sealed for (T1, T2, T3, T4, T5, T6) {}
impl<T1, T2, T3, T4, T5, T6, T7> Sealed for (T1, T2, T3, T4, T5, T6, T7) {}
impl<T1, T2, T3, T4, T5, T6, T7, T8> Sealed for (T1, T2, T3, T4, T5, T6, T7, T8) {}
impl<T1, T2, T3, T4, T5, T6, T7, T8, T9> Sealed for (T1, T2, T3, T4, T5, T6, T7, T8, T9) {}
