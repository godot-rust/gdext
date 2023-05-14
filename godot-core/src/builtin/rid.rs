/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::num::NonZeroU64;

use godot_ffi as sys;
use sys::{ffi_methods, GodotFfi};

/// A RID ("resource ID") is an opaque handle that refers to a Godot `Resource`.
///
/// RIDs do not grant access to the resource itself. Instead, they can be used in lower-level resource APIs
/// such as the [servers]. See also [Godot API docs for `RID`][docs].
///
/// RIDs should be largely safe to work with. Certain calls to servers may fail, however doing so will
/// trigger an error from Godot, and will not cause any UB.
///
/// # Safety Caveat:
///
/// In Godot 3, RID was not as safe as described here. We believe that this is fixed in Godot 4, but this has
/// not been extensively tested as of yet. Some confirmed UB from Godot 3 does not occur anymore, but if you
/// find anything suspicious or outright UB please open an issue.
///
/// [servers]: https://docs.godotengine.org/en/stable/tutorials/optimization/using_servers.html
/// [docs]: https://docs.godotengine.org/en/stable/classes/class_rid.html

// Using normal rust repr to take advantage advantage of the nullable pointer optimization. As this enum is
// eligible for it, it is also guaranteed to have it. Meaning the layout of this type is identical to `u64`.
// See: https://doc.rust-lang.org/nomicon/ffi.html#the-nullable-pointer-optimization
// Cannot use `#[repr(C)]` as it does not use the nullable pointer optimization.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub enum Rid {
    /// A valid RID may refer to some resource, but is not guaranteed to do so.
    Valid(NonZeroU64),

    /// An invalid RID will never refer to a resource. Internally it is represented as a 0.
    Invalid,
}

impl Rid {
    /// Create a new RID.
    #[inline]
    pub const fn new(id: u64) -> Self {
        match NonZeroU64::new(id) {
            Some(id) => Self::Valid(id),
            None => Self::Invalid,
        }
    }

    /// Convert this RID into a [`u64`]. Returns 0 if it is invalid.
    ///
    /// _Godot equivalent: `Rid.get_id()`_
    #[inline]
    pub const fn to_u64(self) -> u64 {
        match self {
            Rid::Valid(id) => id.get(),
            Rid::Invalid => 0,
        }
    }

    /// Convert this RID into a [`u64`] if it is valid. Otherwise return None.
    #[inline]
    pub const fn to_valid_u64(self) -> Option<u64> {
        match self {
            Rid::Valid(id) => Some(id.get()),
            Rid::Invalid => None,
        }
    }

    /// Returns `true` if this is a valid RID.
    #[inline]
    pub const fn is_valid(&self) -> bool {
        matches!(self, Rid::Valid(_))
    }

    /// Returns `true` if this is an invalid RID.
    #[inline]
    pub const fn is_invalid(&self) -> bool {
        matches!(self, Rid::Invalid)
    }
}

impl std::fmt::Display for Rid {
    /// Formats `Rid` to match Godot's string representation.
    ///
    /// Example:
    /// ```
    /// use godot::prelude::*;
    /// let id = Rid::new(1);
    /// assert_eq!(format!("{}", id), "RID(1)");
    /// ```
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // godot output: `RID(0)`
        match self {
            Rid::Valid(x) => write!(f, "RID({})", x),
            Rid::Invalid => write!(f, "RID(0)"),
        }
    }
}

// SAFETY:
// This type is represented as `Self` in Godot, so `*mut Self` is sound.
unsafe impl GodotFfi for Rid {
    ffi_methods! { type sys::GDExtensionTypePtr = *mut Self; .. }
}
