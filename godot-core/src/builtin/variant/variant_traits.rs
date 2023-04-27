/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::builtin::Variant;

/// Trait to enable conversions of types _from_ the [`Variant`] type.
pub trait FromVariant: Sized {
    /// Tries to convert a `Variant` to `Self`, allowing to check the success or failure.
    fn try_from_variant(variant: &Variant) -> Result<Self, VariantConversionError>;

    /// ⚠️ Converts from `Variant` to `Self`, panicking on error.
    ///
    /// This method should generally not be overridden by trait impls, even if conversions are infallible.
    /// Implementing [`Self::try_from_variant`] suffices.
    fn from_variant(variant: &Variant) -> Self {
        Self::try_from_variant(variant).unwrap_or_else(|e| {
            panic!(
                "failed to convert from variant {:?} to {}; {:?}",
                variant,
                std::any::type_name::<Self>(),
                e
            )
        })
    }
}

/// Trait to enable conversions of types _to_ the [`Variant`] type.
pub trait ToVariant {
    /*fn try_to_variant(&self) -> Result<Variant, VariantConversionError>;

    fn to_variant(&self) -> Variant {
        Self::try_to_variant(self).unwrap_or_else(|e| {
            panic!(
                "failed to convert from {} to variant; {:?}",
                std::any::type_name::<Self>(),
                e
            )
        })
    }*/

    /// Infallible conversion from `Self` type to `Variant`.
    ///
    /// This method must not panic. If your conversion is fallible, this trait should not be used.
    fn to_variant(&self) -> Variant;
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

#[derive(Eq, PartialEq, Debug)]
//pub struct VariantConversionError;
pub enum VariantConversionError {
    /// Variant type does not match expected type
    BadType,

    /// Variant value cannot be represented in target type
    BadValue,

    /// Variant value is missing a value for the target type
    MissingValue,
}
