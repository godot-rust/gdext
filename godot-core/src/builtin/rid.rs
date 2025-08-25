/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::num::NonZeroU64;

use godot_ffi as sys;
use sys::{ffi_methods, static_assert, static_assert_eq_size_align, ExtVariantType, GodotFfi};

/// A RID ("resource ID") is an opaque handle that refers to a Godot `Resource`.
///
/// RIDs do not grant access to the resource itself. Instead, they can be used in lower-level resource APIs
/// such as the [servers]. See also [Godot API docs for `RID`][docs].
///
/// RIDs should be largely safe to work with. Certain calls to servers may fail, however doing so will
/// trigger an error from Godot, and will not cause any UB.
///
/// # Safety caveat
///
/// In Godot 3, `RID` was not as safe as described here. We believe that this is fixed in Godot 4, but this has
/// not been extensively tested as of yet. Some confirmed UB from Godot 3 does not occur anymore, but if you
/// find anything suspicious or outright UB please open an issue.
///
/// [servers]: https://docs.godotengine.org/en/stable/tutorials/optimization/using_servers.html
/// [docs]: https://docs.godotengine.org/en/stable/classes/class_rid.html
///
/// # Godot docs
///
/// [`RID` (stable)](https://docs.godotengine.org/en/stable/classes/class_rid.html)

// Using normal Rust repr to take advantage of the nullable pointer optimization. As this enum is
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

// Ensure that `Rid`s actually have the layout we expect. Since `Rid` has the same size as `u64`, it cannot
// have any padding. As the `Valid` variant must take up all but one of the niches (since it contains a
// `NonZerou64`), and the `Invalid` variant must take up the final niche.
static_assert_eq_size_align!(Rid, u64);

// SAFETY:
// As Rid and u64 have the same size, and `Rid::Invalid` is initialized, it must be represented by some `u64`
// Therefore we can safely transmute it to u64.
//
// Ensure that `Rid::Invalid` actually is represented by 0, as it should be.
static_assert!(unsafe { std::mem::transmute::<Rid, u64>(Rid::Invalid) } == 0u64);

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
    #[doc(alias = "get_id")]
    #[inline]
    pub const fn to_u64(self) -> u64 {
        match self {
            Rid::Valid(id) => id.get(),
            Rid::Invalid => 0,
        }
    }

    /// Convert this RID into a [`u64`] if it is valid. Otherwise, return None.
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
    /// # Example
    /// ```
    /// use godot::prelude::*;
    /// let id = Rid::new(1);
    /// assert_eq!(format!("{}", id), "RID(1)");
    /// ```
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // godot output: `RID(0)`
        match self {
            Rid::Valid(x) => write!(f, "RID({x})"),
            Rid::Invalid => write!(f, "RID(0)"),
        }
    }
}

// SAFETY:
// This type is represented as `Self` in Godot, so `*mut Self` is sound.
unsafe impl GodotFfi for Rid {
    const VARIANT_TYPE: ExtVariantType = ExtVariantType::Concrete(sys::VariantType::RID);

    ffi_methods! { type sys::GDExtensionTypePtr = *mut Self;
        fn new_from_sys;
        fn new_with_uninit;
        fn from_arg_ptr;
        fn sys;
        fn sys_mut;
        fn move_return_ptr;
    }

    unsafe fn new_with_init(init: impl FnOnce(sys::GDExtensionTypePtr)) -> Self {
        let mut rid = Self::Invalid;
        init(rid.sys_mut());
        rid
    }
}

crate::meta::impl_godot_as_self!(Rid: ByValue);
