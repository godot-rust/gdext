/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

mod impls;

use crate::builtin::{Variant, VariantConversionError};

use super::{GodotFfiVariant, GodotType};

/// Indicates that a type has some canonical Godot type that can represent it.
///
/// The type specified here is what will be used to pass this type across to ffi-boundary to/from Godot.
/// Generally [`ToGodot`] needs to be implemented to pass a type to Godot, and [`FromGodot`] to receive this
/// type from Godot.
pub trait GodotConvert {
    /// The type used for ffi-passing.
    type Via: GodotType;
}

/// Defines the canonical conversion to Godot for a type.
///
/// It is assumed that all the methods return equal values given equal inputs. Additionally it is assumed
/// that if [`FromGodot`] is implemented, converting to Godot and back again will return a value equal to the
/// starting value.
///
/// Violating these assumptions is safe but will give unexpected results.
pub trait ToGodot: Sized + GodotConvert {
    /// Converts this type to the Godot type by reference, usually by cloning.
    fn to_godot(&self) -> Self::Via;

    /// Converts this type to the Godot type.
    ///
    /// This can in some cases enable some optimizations, such as avoiding reference counting for
    /// reference-counted values.
    fn into_godot(self) -> Self::Via {
        self.to_godot()
    }

    /// Converts this type to a [Variant].
    fn to_variant(&self) -> Variant {
        self.to_godot().to_ffi().ffi_to_variant()
    }
}

/// Defines the canonical conversion from Godot for a type.
///
/// It is assumed that all the methods return equal values given equal inputs. Additionally it is assumed
/// that if [`ToGodot`] is implemented, converting to Godot and back again will return a value equal to the
/// starting value.
///
/// Violating these assumptions is safe but will give unexpected results.
pub trait FromGodot: Sized + GodotConvert {
    // TODO: better error
    /// Performs the conversion.
    #[must_use]
    fn try_from_godot(via: Self::Via) -> Option<Self>;

    /// ⚠️ Performs the conversion.
    ///
    /// # Panics
    /// If the conversion fails.
    fn from_godot(via: Self::Via) -> Self {
        Self::try_from_godot(via).unwrap()
    }

    /// Performs the conversion from a [`Variant`].
    fn try_from_variant(variant: &Variant) -> Result<Self, VariantConversionError> {
        let ffi = <Self::Via as GodotType>::Ffi::ffi_from_variant(variant)?;
        let via = Self::Via::try_from_ffi(ffi).ok_or(VariantConversionError::BadValue)?;
        Self::try_from_godot(via).ok_or(VariantConversionError::BadValue)
    }

    /// ⚠️ Performs the conversion from a [`Variant`].
    ///
    /// # Panics
    /// If the conversion fails.
    fn from_variant(variant: &Variant) -> Self {
        Self::try_from_variant(variant).unwrap()
    }
}

pub(crate) fn into_ffi<T: ToGodot>(t: T) -> <T::Via as GodotType>::Ffi {
    let via = t.into_godot();
    via.into_ffi()
}

pub(crate) fn try_from_ffi<T: FromGodot>(ffi: <T::Via as GodotType>::Ffi) -> Option<T> {
    let via = <T::Via as GodotType>::try_from_ffi(ffi)?;
    T::try_from_godot(via)
}

macro_rules! impl_godot_as_self {
    ($T:ty) => {
        impl $crate::builtin::meta::GodotConvert for $T {
            type Via = $T;
        }

        impl $crate::builtin::meta::ToGodot for $T {
            #[inline]
            fn to_godot(&self) -> Self::Via {
                self.clone()
            }

            #[inline]
            fn into_godot(self) -> Self::Via {
                self
            }
        }

        impl $crate::builtin::meta::FromGodot for $T {
            #[inline]
            fn try_from_godot(via: Self::Via) -> Option<Self> {
                Some(via)
            }
        }
    };
}

pub(crate) use impl_godot_as_self;
