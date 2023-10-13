/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

#[derive(Eq, PartialEq, Debug)]
//pub struct VariantConversionError;
pub enum VariantConversionError {
    /// Variant type does not match expected type
    BadType,

    /// Variant value cannot be represented in target type
    BadValue,

    /// Variant value is missing a value for the target type
    MissingValue,

    /// Variant value is null but expected to be non-null
    VariantIsNil,
}

impl std::fmt::Display for VariantConversionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VariantConversionError::BadType => {
                f.write_str("Variant type does not match expected type")
            }
            VariantConversionError::BadValue => {
                f.write_str("Variant value cannot be represented in target type")
            }
            VariantConversionError::MissingValue => {
                f.write_str("Variant value is missing a value for the target type")
            }
            VariantConversionError::VariantIsNil => {
                f.write_str("Variant value is null but expected to be non-null")
            }
        }
    }
}

impl std::error::Error for VariantConversionError {}
