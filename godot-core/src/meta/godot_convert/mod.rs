/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

mod impls;

use crate::builtin::Variant;
use crate::meta::error::ConvertError;
use crate::meta::traits::GodotFfiVariant;
use crate::meta::GodotType;

/// Indicates that a type can be passed to/from Godot, either directly or through an intermediate "via" type.
///
/// The associated type `Via` specifies _how_ this type is passed across the FFI boundary to/from Godot.
/// Generally [`ToGodot`] needs to be implemented to pass a type to Godot, and [`FromGodot`] to receive this type from Godot.
///
/// [`GodotType`] is a stronger bound than [`GodotConvert`], since it expresses that a type is _directly_ representable
/// in Godot (without intermediate "via"). Every `GodotType` also implements `GodotConvert` with `Via = Self`.
///
/// Please read the [`godot::meta` module docs][crate::meta] for further information about conversions.
#[doc(alias = "via", alias = "transparent")]
#[diagnostic::on_unimplemented(
    message = "`GodotConvert` is needed for `#[func]` parameters/returns, as well as `#[var]` and `#[export]` properties",
    note = "check following errors for more information"
)]
pub trait GodotConvert {
    /// The type through which `Self` is represented in Godot.
    type Via: GodotType;
}

/// Defines the canonical conversion to Godot for a type.
///
/// It is assumed that all the methods return equal values given equal inputs. Additionally, it is assumed
/// that if [`FromGodot`] is implemented, converting to Godot and back again will return a value equal to the
/// starting value.
///
/// Violating these assumptions is safe but will give unexpected results.
///
/// Please read the [`godot::meta` module docs][crate::meta] for further information about conversions.
///
/// This trait can be derived using the [`#[derive(GodotConvert)]`](../register/derive.GodotConvert.html) macro.
pub trait ToGodot: Sized + GodotConvert {
    /// Target type of [`to_godot()`](ToGodot::to_godot), which can differ from [`Via`][GodotConvert::Via] for pass-by-reference types.
    ///
    /// Note that currently, this only differs from `Via` when `Self` is [`RefArg<'r, T>`][crate::meta::RefArg], which is
    /// used inside generated code of  engine methods. Other uses of `to_godot()`, such as return types in `#[func]`, still use value types.
    /// This may change in future versions.
    ///
    /// See also [`AsArg<T>`](crate::meta::AsArg) used as the "front-end" in Godot API parameters.
    type ToVia<'v>: GodotType
    where
        Self: 'v;

    /// Converts this type to the Godot type by reference, usually by cloning.
    fn to_godot(&self) -> Self::ToVia<'_>;

    /// Converts this type to a [Variant].
    // Exception safety: must not panic apart from exceptional circumstances (Nov 2024: only u64).
    // This has invariant implications, e.g. in Array::resize().
    fn to_variant(&self) -> Variant {
        self.to_godot().to_ffi().ffi_to_variant()
    }
}

/// Defines the canonical conversion from Godot for a type.
///
/// It is assumed that all the methods return equal values given equal inputs. Additionally, it is assumed
/// that if [`ToGodot`] is implemented, converting to Godot and back again will return a value equal to the
/// starting value.
///
/// Violating these assumptions is safe but will give unexpected results.
///
/// Please read the [`godot::meta` module docs][crate::meta] for further information about conversions.
///
/// This trait can be derived using the [`#[derive(GodotConvert)]`](../register/derive.GodotConvert.html) macro.
pub trait FromGodot: Sized + GodotConvert {
    /// Converts the Godot representation to this type, returning `Err` on failure.
    fn try_from_godot(via: Self::Via) -> Result<Self, ConvertError>;

    /// ⚠️ Converts the Godot representation to this type.
    ///
    /// # Panics
    /// If the conversion fails.
    fn from_godot(via: Self::Via) -> Self {
        Self::try_from_godot(via)
            .unwrap_or_else(|err| panic!("FromGodot::from_godot() failed: {err}"))
    }

    /// Performs the conversion from a [`Variant`], returning `Err` on failure.
    fn try_from_variant(variant: &Variant) -> Result<Self, ConvertError> {
        let ffi = <Self::Via as GodotType>::Ffi::ffi_from_variant(variant)?;

        let via = Self::Via::try_from_ffi(ffi)?;
        Self::try_from_godot(via)
    }

    /// ⚠️ Performs the conversion from a [`Variant`].
    ///
    /// # Panics
    /// If the conversion fails.
    fn from_variant(variant: &Variant) -> Self {
        Self::try_from_variant(variant).unwrap_or_else(|err| {
            panic!("FromGodot::from_variant() failed -- {err}");
        })
    }
}

pub(crate) fn into_ffi_variant<T: ToGodot>(value: &T) -> Variant {
    let via = value.to_godot();
    let ffi = via.to_ffi();
    GodotFfiVariant::ffi_to_variant(&ffi)
}

pub(crate) fn try_from_ffi<T: FromGodot>(
    ffi: <T::Via as GodotType>::Ffi,
) -> Result<T, ConvertError> {
    let via = <T::Via as GodotType>::try_from_ffi(ffi)?;
    T::try_from_godot(via)
}

#[macro_export]
macro_rules! impl_godot_as_self {
    ($T:ty) => {
        impl $crate::meta::GodotConvert for $T {
            type Via = $T;
        }

        impl $crate::meta::ToGodot for $T {
            type ToVia<'v> = Self::Via;

            #[inline]
            fn to_godot(&self) -> Self::ToVia<'_> {
                self.clone()
            }
        }

        impl $crate::meta::FromGodot for $T {
            #[inline]
            fn try_from_godot(via: Self::Via) -> Result<Self, $crate::meta::error::ConvertError> {
                Ok(via)
            }
        }
    };
}
