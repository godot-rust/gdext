/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::builtin::meta::VariantMetadata;
use crate::builtin::{FromVariant, ToVariant, Variant, VariantConversionError};
use godot_ffi as sys;
use godot_ffi::{ffi_methods, GodotFfi, VariantType};
use std::fmt::{Debug, Display, Formatter, Result as FmtResult};
use std::num::NonZeroU64;

/// Represents a non-zero instance ID.
///
/// This is its own type for type safety and to deal with the inconsistent representation in Godot as both `u64` (C++) and `i64` (GDScript).
/// You can usually treat this as an opaque value and pass it to and from GDScript; there are conversion methods however.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[repr(transparent)]
pub struct InstanceId {
    // Note: in the public API, signed i64 is the canonical representation.
    //
    // Methods converting to/from u64 exist only because GDExtension tends to work with u64. However, user-facing APIs
    // interact with GDScript, which uses i64. Not having two representations avoids confusion about negative values.
    value: NonZeroU64,
}

impl InstanceId {
    /// Constructs an instance ID from an integer, or `None` if the integer is zero.
    ///
    /// This does *not* check if the instance is valid.
    pub fn try_from_i64(id: i64) -> Option<Self> {
        Self::try_from_u64(id as u64)
    }

    /// ⚠️ Constructs an instance ID from a non-zero integer, or panics.
    ///
    /// This does *not* check if the instance is valid.
    ///
    /// # Panics
    /// If `id` is zero.
    pub fn from_nonzero(id: i64) -> Self {
        Self::try_from_i64(id).expect("expected non-zero instance ID")
    }

    // Private: see rationale above
    pub(crate) fn try_from_u64(id: u64) -> Option<Self> {
        NonZeroU64::new(id).map(|value| Self { value })
    }

    pub fn to_i64(self) -> i64 {
        self.to_u64() as i64
    }

    /// Returns if the obj being referred-to is inheriting `RefCounted`.
    ///
    /// This is a very fast operation and involves no engine round-trip, as the information is encoded in the ID itself.
    pub fn is_ref_counted(self) -> bool {
        self.to_u64() & (1u64 << 63) != 0
    }

    // Private: see rationale above
    pub(crate) fn to_u64(self) -> u64 {
        self.value.get()
    }
}

impl Display for InstanceId {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{}", self.to_i64())
    }
}

impl Debug for InstanceId {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "InstanceId({})", self.to_i64())
    }
}

// SAFETY:
// This type is represented as `Self` in Godot, so `*mut Self` is sound.
unsafe impl GodotFfi for InstanceId {
    ffi_methods! { type sys::GDExtensionTypePtr = *mut Self; .. }
}

impl FromVariant for InstanceId {
    fn try_from_variant(variant: &Variant) -> Result<Self, VariantConversionError> {
        i64::try_from_variant(variant)
            .and_then(|i| InstanceId::try_from_i64(i).ok_or(VariantConversionError::BadValue))
    }
}

impl ToVariant for InstanceId {
    fn to_variant(&self) -> Variant {
        let int = self.to_i64();
        int.to_variant()
    }
}

/*
// Note: Option impl is only possible as long as `FromVariant` and `InstanceId` are in same crate.
// (Rust rationale: if upstream crate later adds blanket `impl FromVariant for Option<T>`, this would collide)
impl FromVariant for Option<InstanceId> {
    fn try_from_variant(variant: &Variant) -> Result<Self, VariantConversionError> {
        if variant.is_nil() {
            Ok(None)
        } else {
            // Should 0 in variant be mapped to None, or cause an error like now?
            i64::try_from_variant(variant).map(|i| InstanceId::try_from_i64(i))
        }
    }
}

impl ToVariant for Option<InstanceId> {
    fn to_variant(&self) -> Variant {
        if let Some(id) = self {
            id.to_variant()
        } else {
            0i64.to_variant()
        }
    }
}
*/

impl VariantMetadata for InstanceId {
    fn variant_type() -> VariantType {
        VariantType::Int
    }

    fn param_metadata() -> sys::GDExtensionClassMethodArgumentMetadata {
        sys::GDEXTENSION_METHOD_ARGUMENT_METADATA_INT_IS_INT64
    }
}
