/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::builtin::{FromVariant, ToVariant, Variant, VariantConversionError};
use godot_ffi as sys;
use godot_ffi::{ffi_methods, GodotFfi};
use std::fmt::{Display, Formatter, Result as FmtResult};

/// Represents a non-zero instance ID.
///
/// This is its own type for type safety and to deal with the inconsistent representation in Godot as both `u64` (C++) and `i64` (GDScript).
/// You can usually treat this as an opaque value and pass it to and from GDScript; there are conversion methods however.
#[derive(Copy, Clone, Eq, PartialEq, Hash, Ord, PartialOrd, Debug)]
pub struct InstanceId {
    // Note: in the public API, signed i64 is the canonical representation.
    //
    // Methods converting to/from u64 exist only because GDExtension tends to work with u64. However, user-facing APIs
    // interact with GDScript, which uses i64. Not having two representations avoids confusion about negative values.
    value: u64,
}

impl InstanceId {
    /// Constructs an instance ID from an integer, or `None` if the integer is zero.
    pub fn try_from_i64(id: i64) -> Option<Self> {
        Self::try_from_u64(id as u64)
    }

    // Private: see rationale above
    pub(crate) fn try_from_u64(id: u64) -> Option<Self> {
        if id == 0 {
            None
        } else {
            Some(InstanceId { value: id })
        }
    }

    pub fn to_i64(self) -> i64 {
        self.value as i64
    }

    // Private: see rationale above
    pub(crate) fn to_u64(self) -> u64 {
        self.value
    }

    /// Returns if the object being referred-to is inheriting `RefCounted`
    #[allow(dead_code)]
    pub(crate) fn is_ref_counted(self) -> bool {
        self.value & (1u64 << 63) != 0
    }
}

impl Display for InstanceId {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{:x}", self.value)
    }
}

impl GodotFfi for InstanceId {
    ffi_methods! { type sys::GDNativeTypePtr = *mut Self; .. }
}

impl FromVariant for InstanceId {
    fn try_from_variant(variant: &Variant) -> Result<Self, VariantConversionError> {
        i64::try_from_variant(variant)
            .and_then(|i| InstanceId::try_from_i64(i).ok_or(VariantConversionError))
    }
}

impl ToVariant for InstanceId {
    fn to_variant(&self) -> Variant {
        let int = self.to_i64();
        int.to_variant()
    }
}

/*
TODO, this is only possible if gdext-builtin and godot-core crates are merged, due to orphan rule
Rust rationale: if upstream crate later adds blanket `impl FromVariant for Option<T>`, this would collide

impl FromVariant for Option<InstanceId> {
    fn try_from_variant(variant: &Variant) -> Result<Self, VariantConversionError> {
        i64::try_from_variant(variant).and_then(|i| InstanceId::try_from_i64(i))
    }
}

impl ToVariant for Option<InstanceId> {
    fn to_variant(&self) -> Variant {
        let int = self.to_i64();
        int.to_variant()
    }
}
*/
