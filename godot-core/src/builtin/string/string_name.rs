/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::fmt;

use godot_ffi as sys;
use sys::{ffi_methods, GodotFfi};

use crate::builtin::inner;
use crate::builtin::meta::impl_godot_as_self;
use crate::builtin::{GString, NodePath};

/// A string optimized for unique names.
///
/// StringNames are immutable strings designed for representing unique names. StringName ensures that only
/// one instance of a given name exists.
///
/// # Ordering
///
/// In Godot, `StringName`s are **not** ordered lexicographically, and the ordering relation is **not** stable across multiple runs of your
/// application. Therefore, this type does not implement `PartialOrd` and `Ord`, as it would be very easy to introduce bugs by accidentally
/// relying on lexicographical ordering.
///
/// Instead, we provide [`transient_ord()`][Self::transient_ord] for ordering relations.
#[repr(C)]
pub struct StringName {
    opaque: sys::types::OpaqueStringName,
}

impl StringName {
    fn from_opaque(opaque: sys::types::OpaqueStringName) -> Self {
        Self { opaque }
    }

    /// Creates a `StringName` from a static, nul-terminated ASCII/Latin-1 `b"string"` literal.
    ///
    /// Avoids unnecessary copies and allocations and directly uses the backing buffer. Useful for literals.
    ///
    /// # Example
    /// ```no_run
    /// use godot::builtin::StringName;
    ///
    /// // '±' is a Latin-1 character with codepoint 0xB1. Note that this is not UTF-8, where it would need two bytes.
    /// let sname = StringName::from_latin1_with_nul(b"\xb1 Latin-1 string\0");
    /// ```
    ///
    /// # Panics
    /// When the string is not nul-terminated or contains interior nul bytes.
    ///
    /// Note that every byte is valid in Latin-1, so there is no encoding validation being performed.
    #[cfg(since_api = "4.2")]
    pub fn from_latin1_with_nul(latin1_c_str: &'static [u8]) -> Self {
        let c_str = std::ffi::CStr::from_bytes_with_nul(latin1_c_str)
            .unwrap_or_else(|_| panic!("invalid or not nul-terminated CStr: '{latin1_c_str:?}'"));

        // SAFETY: latin1_c_str is nul-terminated and remains valid for entire program duration.
        let result = unsafe {
            Self::from_string_sys_init(|ptr| {
                sys::interface_fn!(string_name_new_with_latin1_chars)(
                    ptr,
                    c_str.as_ptr(),
                    true as sys::GDExtensionBool, // p_is_static
                )
            })
        };

        // StringName expects that the destructor is not invoked on static instances (or only at global exit; see SNAME(..) macro in Godot).
        // According to testing with godot4 --verbose, there is no mention of "Orphan StringName" at shutdown when incrementing the ref-count,
        // so this should not leak memory.
        result.inc_ref();
        result
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

    /// O(1), non-lexicographic, non-stable ordering relation.
    ///
    /// The result of the comparison is **not** lexicographic and **not** stable across multiple runs of your application.
    ///
    /// However, it is very fast. It doesn't depend on the length of the strings, but on the memory location of string names.
    /// This can still be useful if you need to establish an ordering relation, but are not interested in the actual order of the strings
    /// (example: binary search).
    ///
    /// For lexicographical ordering, convert to `GString` (significantly slower).
    pub fn transient_ord(&self) -> TransientStringNameOrd<'_> {
        TransientStringNameOrd(self)
    }

    ffi_methods! {
        type sys::GDExtensionStringNamePtr = *mut Opaque;

        // Note: unlike from_sys, from_string_sys does not default-construct instance first. Typical usage in C++ is placement new.
        fn from_string_sys = from_sys;
        fn from_string_sys_init = from_sys_init;
        fn string_sys = sys;
    }

    #[doc(hidden)]
    pub fn string_sys_const(&self) -> sys::GDExtensionConstStringNamePtr {
        sys::to_const_ptr(self.string_sys())
    }

    #[doc(hidden)]
    pub fn as_inner(&self) -> inner::InnerStringName {
        inner::InnerStringName::from_outer(self)
    }

    /// Increment ref-count. This may leak memory if used wrongly.
    fn inc_ref(&self) {
        std::mem::forget(self.clone());
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
//   `std::mem::forget(string_name.clone())`.
unsafe impl GodotFfi for StringName {
    fn variant_type() -> sys::VariantType {
        sys::VariantType::StringName
    }

    ffi_methods! { type sys::GDExtensionTypePtr = *mut Opaque;
        fn from_sys;
        fn sys;
        fn from_sys_init;
        fn move_return_ptr;
    }

    unsafe fn from_arg_ptr(ptr: sys::GDExtensionTypePtr, _call_type: sys::PtrcallType) -> Self {
        let string_name = Self::from_sys(ptr);
        string_name.inc_ref();
        string_name
    }

    unsafe fn from_sys_init_default(init_fn: impl FnOnce(sys::GDExtensionTypePtr)) -> Self {
        let mut result = Self::default();
        init_fn(result.sys_mut());
        result
    }
}

impl_godot_as_self!(StringName);

impl_builtin_traits! {
    for StringName {
        Default => string_name_construct_default;
        Clone => string_name_construct_copy;
        Drop => string_name_destroy;
        Eq => string_name_operator_equal;
        // Do not provide PartialOrd or Ord. Even though Godot provides a `operator <`, it is non-lexicographic and non-deterministic
        // (based on pointers). See transient_ord() method.
        Hash;
    }
}

impl fmt::Display for StringName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = GString::from(self);
        <GString as fmt::Display>::fmt(&s, f)
    }
}

/// Uses literal syntax from GDScript: `&"string_name"`
impl fmt::Debug for StringName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let string = GString::from(self);
        write!(f, "&\"{string}\"")
    }
}

// SAFETY: StringName is immutable once constructed. Shared references can thus not undergo mutation.
unsafe impl Sync for StringName {}

// SAFETY: StringName is immutable once constructed. Also, its inc-ref/dec-ref operations are mutex-protected in Godot.
// That is, it's safe to construct a StringName on thread A and destroy it on thread B.
unsafe impl Send for StringName {}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Conversion from/into other string-types

impl_rust_string_conv!(StringName);

impl<S> From<S> for StringName
where
    S: AsRef<str>,
{
    #[cfg(before_api = "4.2")]
    fn from(string: S) -> Self {
        let intermediate = GString::from(string.as_ref());
        Self::from(&intermediate)
    }

    #[cfg(since_api = "4.2")]
    fn from(string: S) -> Self {
        let utf8 = string.as_ref().as_bytes();

        // SAFETY: Rust guarantees validity and range of string.
        unsafe {
            Self::from_string_sys_init(|ptr| {
                sys::interface_fn!(string_name_new_with_utf8_chars_and_len)(
                    ptr,
                    utf8.as_ptr() as *const std::ffi::c_char,
                    utf8.len() as i64,
                );
            })
        }
    }
}

impl From<&GString> for StringName {
    fn from(string: &GString) -> Self {
        unsafe {
            sys::from_sys_init_or_init_default::<Self>(|self_ptr| {
                let ctor = sys::builtin_fn!(string_name_from_string);
                let args = [string.sys_const()];
                ctor(self_ptr, args.as_ptr());
            })
        }
    }
}

impl From<GString> for StringName {
    /// Converts this `GString` to a `StringName`.
    ///
    /// This is identical to `StringName::from(&string)`, and as such there is no performance benefit.
    fn from(string: GString) -> Self {
        Self::from(&string)
    }
}

impl From<&NodePath> for StringName {
    fn from(path: &NodePath) -> Self {
        Self::from(GString::from(path))
    }
}

impl From<NodePath> for StringName {
    /// Converts this `NodePath` to a `StringName`.
    ///
    /// This is identical to `StringName::from(&path)`, and as such there is no performance benefit.
    fn from(path: NodePath) -> Self {
        Self::from(GString::from(path))
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Ordering

/// Type that implements `Ord` for `StringNames`.
///
/// See [`StringName::transient_ord()`].
pub struct TransientStringNameOrd<'a>(&'a StringName);

impl<'a> PartialEq for TransientStringNameOrd<'a> {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl<'a> Eq for TransientStringNameOrd<'a> {}

impl<'a> PartialOrd for TransientStringNameOrd<'a> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

// implement Ord like above
impl<'a> Ord for TransientStringNameOrd<'a> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // SAFETY: builtin operator provided by Godot.
        let op_less = |lhs, rhs| unsafe {
            let mut result = false;
            sys::builtin_call! {
                string_name_operator_less(lhs, rhs, result.sys_mut())
            }
            result
        };

        let self_ptr = self.0.sys();
        let other_ptr = other.0.sys();

        if op_less(self_ptr, other_ptr) {
            std::cmp::Ordering::Less
        } else if op_less(other_ptr, self_ptr) {
            std::cmp::Ordering::Greater
        } else if self.eq(other) {
            std::cmp::Ordering::Equal
        } else {
            panic!(
                "Godot provides inconsistent StringName ordering for \"{}\" and \"{}\"",
                self.0, other.0
            );
        }
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// serde support

#[cfg(feature = "serde")]
mod serialize {
    use super::*;
    use serde::de::{Error, Visitor};
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::fmt::Formatter;

    impl Serialize for StringName {
        #[inline]
        fn serialize<S>(
            &self,
            serializer: S,
        ) -> Result<<S as Serializer>::Ok, <S as Serializer>::Error>
        where
            S: Serializer,
        {
            serializer.serialize_str(&self.to_string())
        }
    }

    impl<'de> serialize::Deserialize<'de> for StringName {
        #[inline]
        fn deserialize<D>(deserializer: D) -> Result<Self, <D as Deserializer<'de>>::Error>
        where
            D: Deserializer<'de>,
        {
            struct StringNameVisitor;
            impl<'de> Visitor<'de> for StringNameVisitor {
                type Value = StringName;

                fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
                    formatter.write_str("a StringName")
                }

                fn visit_str<E>(self, s: &str) -> Result<Self::Value, E>
                where
                    E: Error,
                {
                    Ok(StringName::from(s))
                }
            }

            deserializer.deserialize_str(StringNameVisitor)
        }
    }
}
