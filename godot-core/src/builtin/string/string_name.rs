/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use godot_ffi as sys;
use sys::{ffi_methods, GodotFfi};

use std::fmt;

use crate::builtin::inner;

use super::{GodotString, NodePath};

/// A string optimized for unique names.
///
/// StringNames are immutable strings designed for representing unique names. StringName ensures that only
/// one instance of a given name exists.
#[repr(C)]
pub struct StringName {
    opaque: sys::types::OpaqueStringName,
}

impl StringName {
    fn from_opaque(opaque: sys::types::OpaqueStringName) -> Self {
        Self { opaque }
    }

    /// Returns the number of characters in the string.
    ///
    /// _Godot equivalent: `length`_
    #[doc(alias = "length")]
    pub fn len(&self) -> usize {
        self.as_inner().length() as usize
    }

    /// Returns `true` if this is the empty string.
    ///
    /// _Godot equivalent: `is_empty`_
    pub fn is_empty(&self) -> bool {
        self.as_inner().is_empty()
    }

    /// Returns a 32-bit integer hash value representing the string.
    pub fn hash(&self) -> u32 {
        self.as_inner()
            .hash()
            .try_into()
            .expect("Godot hashes are uint32_t")
    }

    ffi_methods! {
        type sys::GDExtensionStringNamePtr = *mut Opaque;

        // Note: unlike from_sys, from_string_sys does not default-construct instance first. Typical usage in C++ is placement new.
        fn from_string_sys = from_sys;
        fn from_string_sys_init = from_sys_init;
        fn string_sys = sys;
    }

    #[doc(hidden)]
    pub fn as_inner(&self) -> inner::InnerStringName {
        inner::InnerStringName::from_outer(self)
    }
}

// SAFETY:
// - `move_return_ptr`
//   Nothing special needs to be done beyond a `std::mem::swap` when returning a StringName.
//   So we can just use `ffi_methods`.
//
// - `from_arg_ptr`
//   StringNames are properly initialized through a `from_sys` call, but the ref-count should be
//   incremented as that is the callee's responsibility. Which we do by calling
//   `std::mem::forget(string_name.share())`.
unsafe impl GodotFfi for StringName {
    ffi_methods! { type sys::GDExtensionTypePtr = *mut Opaque;
        fn from_sys;
        fn sys;
        fn from_sys_init;
        fn move_return_ptr;
    }

    unsafe fn from_arg_ptr(ptr: sys::GDExtensionTypePtr, _call_type: sys::PtrcallType) -> Self {
        let string_name = Self::from_sys(ptr);
        std::mem::forget(string_name.clone());
        string_name
    }

    unsafe fn from_sys_init_default(init_fn: impl FnOnce(sys::GDExtensionTypePtr)) -> Self {
        let mut result = Self::default();
        init_fn(result.sys_mut());
        result
    }
}

impl_builtin_traits! {
    for StringName {
        Default => string_name_construct_default;
        Clone => string_name_construct_copy;
        Drop => string_name_destroy;
        Eq => string_name_operator_equal;
        // currently broken: https://github.com/godotengine/godot/issues/76218
        // Ord => string_name_operator_less;
        Hash;
    }
}

impl fmt::Display for StringName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = GodotString::from(self);
        <GodotString as fmt::Display>::fmt(&s, f)
    }
}

/// Uses literal syntax from GDScript: `&"string_name"`
impl fmt::Debug for StringName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let string = GodotString::from(self);
        write!(f, "&\"{string}\"")
    }
}
// ----------------------------------------------------------------------------------------------------------------------------------------------
// Conversion from/into other string-types

impl_rust_string_conv!(StringName);

impl From<&GodotString> for StringName {
    fn from(string: &GodotString) -> Self {
        unsafe {
            sys::from_sys_init_or_init_default::<Self>(|self_ptr| {
                let ctor = sys::builtin_fn!(string_name_from_string);
                let args = [string.sys_const()];
                ctor(self_ptr, args.as_ptr());
            })
        }
    }
}

impl From<GodotString> for StringName {
    /// Converts this `GodotString` to a `StringName`.
    ///
    /// This is identical to `StringName::from(&string)`, and as such there is no performance benefit.
    fn from(string: GodotString) -> Self {
        Self::from(&string)
    }
}

impl From<&NodePath> for StringName {
    fn from(path: &NodePath) -> Self {
        Self::from(GodotString::from(path))
    }
}

impl From<NodePath> for StringName {
    /// Converts this `NodePath` to a `StringName`.
    ///
    /// This is identical to `StringName::from(&path)`, and as such there is no performance benefit.
    fn from(path: NodePath) -> Self {
        Self::from(GodotString::from(path))
    }
}
