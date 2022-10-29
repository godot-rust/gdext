/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::builtin::Variant;

pub trait FromVariant: Sized {
    fn try_from_variant(variant: &Variant) -> Result<Self, VariantConversionError>;

    #[cfg(feature = "convenience")]
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

    fn to_variant(&self) -> Variant;
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

#[derive(Debug)]
pub struct VariantConversionError;
/*pub enum VariantConversionError {
    /// Variant type does not match expected type
    BadType,

    /// Variant value cannot be represented in target type
    BadValue,
}*/
