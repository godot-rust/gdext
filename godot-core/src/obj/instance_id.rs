/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::fmt::{Debug, Display, Formatter, Result as FmtResult};
use std::num::NonZeroU64;

use crate::meta::error::{ConvertError, FromGodotError};
use crate::meta::{FromGodot, GodotConvert, ToGodot};
use crate::registry::property::SimpleVar;

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
    /// If `id` is zero. Use [`try_from_i64`][Self::try_from_i64] if you are unsure.
    pub fn from_i64(id: i64) -> Self {
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

    /// Dynamically checks if the instance behind the ID exists.
    ///
    /// Rather slow, involves engine round-trip plus object DB lookup. If you need the object, use
    /// [`Gd::from_instance_id()`][crate::obj::Gd::from_instance_id] instead.
    ///
    /// This corresponds to Godot's global function `is_instance_id_valid()`.
    #[doc(alias = "is_instance_id_valid")]
    pub fn lookup_validity(self) -> bool {
        crate::global::is_instance_id_valid(self.to_i64())
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

impl GodotConvert for InstanceId {
    // Use i64 and not u64 because the former can be represented in Variant, and is also the number format GDScript uses.
    // The engine's C++ code can still use u64.
    type Via = i64;
}

impl ToGodot for InstanceId {
    type Pass = crate::meta::ByValue;

    fn to_godot(&self) -> Self::Via {
        self.to_i64()
    }
}

impl FromGodot for InstanceId {
    fn try_from_godot(via: Self::Via) -> Result<Self, ConvertError> {
        Self::try_from_i64(via).ok_or_else(|| FromGodotError::ZeroInstanceId.into_error(via))
    }
}

impl SimpleVar for InstanceId {}
