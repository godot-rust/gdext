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
use crate::meta::{ArgPassing, GodotType, ToArg};

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
#[diagnostic::on_unimplemented(
    message = "passing type `{Self}` to Godot requires `ToGodot` trait, which is usually provided by the library",
    note = "ToGodot is implemented for built-in types (i32, Vector2, GString, …). For objects, use Gd<T> instead of T.",
    note = "if you really need a custom representation (for non-class types), implement ToGodot manually or use #[derive(GodotConvert)].",
    note = "see also: https://godot-rust.github.io/docs/gdext/master/godot/meta"
)]
pub trait ToGodot: Sized + GodotConvert {
    /// Whether arguments of this type are passed by value or by reference.
    ///
    /// Can be either [`ByValue`][crate::meta::ByValue] or [`ByRef`][crate::meta::ByRef]. In most cases, you need `ByValue`.
    ///
    /// Select `ByValue` if:
    /// - `Self` is `Copy` (e.g. `i32`, `f64`, `Vector2`, `Color`, etc).
    /// - You need a conversion (e.g. `Self = MyString`, `Via = GString`).
    /// - You like the simple life and can't be bothered with lifetimes.
    ///
    /// Select `ByRef` if:
    /// - Performance of argument passing is very important and you have measured it.
    /// - You store a cached value which can be borrowed (e.g. `&GString`).
    ///
    /// Will auto-implement [`AsArg<T>`][crate::meta::AsArg] for either `T` (by-value) or for `&T` (by-reference).
    /// This has an influence on contexts such as [`Array::push()`][crate::builtin::Array::push], the [`array![...]`][crate::builtin::array]
    /// macro or generated signal `emit()` signatures.
    type Pass: ArgPassing;

    /// Converts this type to Godot representation, optimizing for zero-copy when possible.
    ///
    /// # Return type
    /// - For `Pass = ByValue`, returns owned `Self::Via`.
    /// - For `Pass = ByRef`, returns borrowed `&Self::Via`.
    fn to_godot(&self) -> ToArg<'_, Self::Via, Self::Pass>;

    /// Converts this type to owned Godot representation.
    ///
    /// Always returns `Self::Via`, cloning if necessary for ByRef types.
    // Future: could potentially split into separate ToGodotOwned trait, which has a blanket impl for T: Clone, while requiring
    // manual implementation for non-Clone types. This would remove the Via: Clone bound, which can be restrictive.
    fn to_godot_owned(&self) -> Self::Via
    where
        Self::Via: Clone,
    {
        Self::Pass::ref_to_owned_via(self)
    }

    /// Converts this type to a [Variant].
    // Exception safety: must not panic apart from exceptional circumstances (Nov 2024: only u64).
    // This has invariant implications, e.g. in Array::resize().
    fn to_variant(&self) -> Variant {
        Self::Pass::ref_to_variant(self)
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
#[diagnostic::on_unimplemented(
    message = "receiving type `{Self}` from Godot requires `FromGodot` trait, which is usually provided by the library",
    note = "FromGodot is implemented for built-in types (i32, Vector2, GString, …). For objects, use Gd<T> instead of T.",
    note = "if you really need a custom representation (for non-class types), implement FromGodot manually or use #[derive(GodotConvert)]",
    note = "see also: https://godot-rust.github.io/docs/gdext/master/godot/meta"
)]
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

#[macro_export]
macro_rules! impl_godot_as_self {
    ($T:ty: $Passing:ident) => {
        impl $crate::meta::GodotConvert for $T {
            type Via = $T;
        }

        $crate::impl_godot_as_self!(@to_godot $T: $Passing);

        impl $crate::meta::FromGodot for $T {
            #[inline]
            fn try_from_godot(via: Self::Via) -> Result<Self, $crate::meta::error::ConvertError> {
                Ok(via)
            }
        }
    };

    (@to_godot $T:ty: ByValue) => {
        impl $crate::meta::ToGodot for $T {
            type Pass = $crate::meta::ByValue;

            #[inline]
            fn to_godot(&self) -> Self::Via {
                self.clone()
            }
        }
    };

    (@to_godot $T:ty: ByRef) => {
        impl $crate::meta::ToGodot for $T {
            type Pass = $crate::meta::ByRef;

            #[inline]
            fn to_godot(&self) -> &Self::Via {
                self
            }
        }
    };
}
