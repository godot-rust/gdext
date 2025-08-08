/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::builtin::{real, Vector2, Vector2i, Vector3, Vector3i, Vector4, Vector4i};
use crate::meta::error::{ConvertError, FromGodotError};
use crate::meta::{FromGodot, GodotConvert, ToGodot};
use crate::obj::EngineEnum;

macro_rules! impl_vector_axis_enum {
    ($Vector:ident, $AxisEnum:ident, ($($axis:ident),+)) => {
        #[doc = concat!("Enumerates the axes in a [`", stringify!($Vector), "`].")]
        ///
        #[doc = concat!("`", stringify!($Vector), "` implements `Index<", stringify!($AxisEnum), ">` and `IndexMut<", stringify!($AxisEnum), ">`")]
        #[doc = ", so you can use this type to access a vector component as `vec[axis]`."]
        #[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
        #[repr(i32)]
        pub enum $AxisEnum {
            $(
                #[doc = concat!("The ", stringify!($axis), " axis.")]
                $axis,
            )+
        }

        impl EngineEnum for $AxisEnum {
            fn try_from_ord(ord: i32) -> Option<Self> {
                match ord {
                    $(
                        x if x == Self::$axis as i32 => Some(Self::$axis),
                    )+
                    _ => None,
                }
            }

            fn ord(self) -> i32 {
                self as i32
            }

            fn as_str(&self) -> &'static str {
                match *self {
                    $(
                        Self::$axis => stringify!($axis),
                    )+
                }
            }

            fn godot_name(&self) -> &'static str {
                match *self {
                    $(
                        Self::$axis => concat!("AXIS_", stringify!($axis)),
                    )+
                }
            }

            fn values() -> &'static [Self] {
                // For vector axis enums, all values are distinct, so both are the same
                &[
                    $( $AxisEnum::$axis, )+
                ]
            }

            fn all_constants() -> &'static [crate::meta::inspect::EnumConstant<$AxisEnum>] {
                use crate::meta::inspect::EnumConstant;
                const { &[
                    $(
                        EnumConstant::new(
                            stringify!($axis),
                            concat!("AXIS_", stringify!($axis)),
                            $AxisEnum::$axis
                        ),
                    )+
                ] }
            }
        }

        impl GodotConvert for $AxisEnum {
            type Via = i32;
        }

        impl ToGodot for $AxisEnum {
            type ToVia<'v> = i32;

            fn to_godot(&self) -> Self::ToVia<'_> {
                self.ord()
            }
        }

        impl FromGodot for $AxisEnum {
            fn try_from_godot(via: Self::Via) -> Result<Self, ConvertError> {
                Self::try_from_ord(via).ok_or_else(|| FromGodotError::InvalidEnum.into_error(via))
            }
        }
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

impl_vector_axis_enum!(Vector2, Vector2Axis, (X, Y));
impl_vector_axis_enum!(Vector3, Vector3Axis, (X, Y, Z));
impl_vector_axis_enum!(Vector4, Vector4Axis, (X, Y, Z, W));

impl_vector_index!(Vector2, real, (x, y), Vector2Axis, (X, Y));
impl_vector_index!(Vector2i, i32, (x, y), Vector2Axis, (X, Y));

impl_vector_index!(Vector3, real, (x, y, z), Vector3Axis, (X, Y, Z));
impl_vector_index!(Vector3i, i32, (x, y, z), Vector3Axis, (X, Y, Z));

impl_vector_index!(Vector4, real, (x, y, z, w), Vector4Axis, (X, Y, Z, W));
impl_vector_index!(Vector4i, i32, (x, y, z, w), Vector4Axis, (X, Y, Z, W));
