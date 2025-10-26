/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::fmt;

use godot_ffi as sys;
use sys::{ffi_methods, ExtVariantType, GodotFfi};

use crate::builtin::{inner, Encoding, GString, NodePath, Variant};
use crate::meta::error::StringError;
use crate::meta::AsArg;
use crate::{impl_shared_string_api, meta};

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
///
/// # Null bytes
///
/// Note that Godot ignores any bytes after a null-byte. This means that for instance `"hello, world!"` and  \
/// `"hello, world!\0 ignored by Godot"` will be treated as the same string if converted to a `StringName`.
///
/// # All string types
///
/// | Intended use case | String type                                |
/// |-------------------|--------------------------------------------|
/// | General purpose   | [`GString`][crate::builtin::GString]       |
/// | Interned names    | **`StringName`**                           |
/// | Scene-node paths  | [`NodePath`][crate::builtin::NodePath]     |
///
/// # Godot docs
///
/// [`StringName` (stable)](https://docs.godotengine.org/en/stable/classes/class_stringname.html)
// Currently we rely on `transparent` for `borrow_string_sys`.
#[repr(transparent)]
pub struct StringName {
    opaque: sys::types::OpaqueStringName,
}

impl StringName {
    fn from_opaque(opaque: sys::types::OpaqueStringName) -> Self {
        Self { opaque }
    }

    /// Convert string from bytes with given encoding, returning `Err` on validation errors.
    ///
    /// Intermediate `NUL` characters are not accepted in Godot and always return `Err`.
    ///
    /// Some notes on the encodings:
    /// - **Latin-1:** Since every byte is a valid Latin-1 character, no validation besides the `NUL` byte is performed.
    ///   It is your responsibility to ensure that the input is meaningful under Latin-1.
    /// - **ASCII**: Subset of Latin-1, which is additionally validated to be valid, non-`NUL` ASCII characters.
    /// - **UTF-8**: The input is validated to be UTF-8.
    ///
    /// Specifying incorrect encoding is safe, but may result in unintended string values.
    pub fn try_from_bytes(bytes: &[u8], encoding: Encoding) -> Result<Self, StringError> {
        Self::try_from_bytes_with_nul_check(bytes, encoding, true)
    }

    /// Convert string from bytes with given encoding, returning `Err` on validation errors.
    ///
    /// Convenience function for [`try_from_bytes()`](Self::try_from_bytes); see its docs for more information.
    ///
    /// When called with `Encoding::Latin1`, this can be slightly more efficient than `try_from_bytes()`.
    pub fn try_from_cstr(cstr: &std::ffi::CStr, encoding: Encoding) -> Result<Self, StringError> {
        // Since Godot 4.2, we can directly short-circuit for Latin-1, which takes a null-terminated C string.
        if encoding == Encoding::Latin1 {
            // Note: CStr guarantees no intermediate NUL bytes, so we don't need to check for them.

            let is_static = sys::conv::SYS_FALSE;
            let s = unsafe {
                Self::new_with_string_uninit(|string_ptr| {
                    let ctor = sys::interface_fn!(string_name_new_with_latin1_chars);
                    ctor(
                        string_ptr,
                        cstr.as_ptr() as *const std::ffi::c_char,
                        is_static,
                    );
                })
            };
            return Ok(s);
        }

        Self::try_from_bytes_with_nul_check(cstr.to_bytes(), encoding, false)
    }

    fn try_from_bytes_with_nul_check(
        bytes: &[u8],
        encoding: Encoding,
        check_nul: bool,
    ) -> Result<Self, StringError> {
        match encoding {
            Encoding::Ascii => {
                // ASCII is a subset of UTF-8, and UTF-8 has a more direct implementation than Latin-1; thus use UTF-8 via `From<&str>`.
                if !bytes.is_ascii() {
                    Err(StringError::new("invalid ASCII"))
                } else if check_nul && bytes.contains(&0) {
                    Err(StringError::new("intermediate NUL byte in ASCII string"))
                } else {
                    // SAFETY: ASCII is a subset of UTF-8 and was verified above.
                    let ascii = unsafe { std::str::from_utf8_unchecked(bytes) };
                    Ok(Self::from(ascii))
                }
            }
            Encoding::Latin1 => {
                // This branch is short-circuited if invoked for CStr, which uses `string_name_new_with_latin1_chars`
                // (requires nul-termination). In general, fall back to GString conversion.
                GString::try_from_bytes_with_nul_check(bytes, Encoding::Latin1, check_nul)
                    .map(|s| Self::from(&s))
            }
            Encoding::Utf8 => {
                // from_utf8() also checks for intermediate NUL bytes.
                let utf8 = std::str::from_utf8(bytes);

                utf8.map(StringName::from)
                    .map_err(|e| StringError::with_source("invalid UTF-8", e))
            }
        }
    }

    /// Number of characters in the string.
    ///
    /// _Godot equivalent: `length`_
    #[doc(alias = "length")]
    pub fn len(&self) -> usize {
        self.as_inner().length() as usize
    }

    crate::declare_hash_u32_method! {
        /// Returns a 32-bit integer hash value representing the string.
    }

    #[deprecated = "renamed to `hash_u32`"]
    pub fn hash(&self) -> u32 {
        self.as_inner()
            .hash()
            .try_into()
            .expect("Godot hashes are uint32_t")
    }

    meta::declare_arg_method! {
        /// Use as argument for an [`impl AsArg<GString|NodePath>`][crate::meta::AsArg] parameter.
        ///
        /// This is a convenient way to convert arguments of similar string types.
        ///
        /// # Example
        /// [`Node::set_name()`][crate::classes::Node::set_name] takes `GString`, let's pass a `StringName`:
        /// ```no_run
        /// # use godot::prelude::*;
        /// let needle = StringName::from("str");
        /// let haystack = GString::from("a long string");
        /// let found = haystack.find(needle.arg());
        /// ```
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
        fn new_from_string_sys = new_from_sys;
        fn new_with_string_uninit = new_with_uninit;
        fn string_sys = sys;
        fn string_sys_mut = sys_mut;
    }

    /// Consumes self and turns it into a sys-ptr, should be used together with [`from_owned_string_sys`](Self::from_owned_string_sys).
    ///
    /// This will leak memory unless `from_owned_string_sys` is called on the returned pointer.
    pub(crate) fn into_owned_string_sys(self) -> sys::GDExtensionStringNamePtr {
        sys::static_assert_eq_size_align!(StringName, sys::types::OpaqueStringName);

        let leaked = Box::into_raw(Box::new(self));
        leaked.cast()
    }

    /// Creates a `StringName` from a sys-ptr without incrementing the refcount.
    ///
    /// # Safety
    ///
    /// * Must only be used on a pointer returned from a call to [`into_owned_string_sys`](Self::into_owned_string_sys).
    /// * Must not be called more than once on the same pointer.
    #[deny(unsafe_op_in_unsafe_fn)]
    pub(crate) unsafe fn from_owned_string_sys(ptr: sys::GDExtensionStringNamePtr) -> Self {
        sys::static_assert_eq_size_align!(StringName, sys::types::OpaqueStringName);

        let ptr = ptr.cast::<Self>();

        // SAFETY: `ptr` was returned from a call to `into_owned_string_sys`, which means it was created by a call to
        // `Box::into_raw`, thus we can use `Box::from_raw` here. Additionally, this is only called once on this pointer.
        let boxed = unsafe { Box::from_raw(ptr) };
        *boxed
    }

    /// Convert a `StringName` sys pointer to a reference with unbounded lifetime.
    ///
    /// # Safety
    ///
    /// `ptr` must point to a live `StringName` for the duration of `'a`.
    pub(crate) unsafe fn borrow_string_sys<'a>(
        ptr: sys::GDExtensionConstStringNamePtr,
    ) -> &'a StringName {
        sys::static_assert_eq_size_align!(StringName, sys::types::OpaqueStringName);
        &*(ptr.cast::<StringName>())
    }

    /// Convert a `StringName` sys pointer to a mutable reference with unbounded lifetime.
    ///
    /// # Safety
    ///
    /// - `ptr` must point to a live `StringName` for the duration of `'a`.
    /// - Must be exclusive - no other reference to given `StringName` instance can exist for the duration of `'a`.
    pub(crate) unsafe fn borrow_string_sys_mut<'a>(
        ptr: sys::GDExtensionStringNamePtr,
    ) -> &'a mut StringName {
        sys::static_assert_eq_size_align!(StringName, sys::types::OpaqueStringName);
        &mut *(ptr.cast::<StringName>())
    }

    #[doc(hidden)]
    pub fn as_inner(&self) -> inner::InnerStringName<'_> {
        inner::InnerStringName::from_outer(self)
    }

    #[doc(hidden)] // Private for now. Needs API discussion, also regarding overlap with try_from_cstr().
    pub fn __cstr(c_str: &'static std::ffi::CStr) -> Self {
        // This used to be set to true, but `p_is_static` parameter in Godot should only be enabled if the result is indeed stored
        // in a static. See discussion in https://github.com/godot-rust/gdext/pull/1316. We may unify this into a regular constructor,
        // or provide a dedicated StringName cache (similar to ClassId cache) in the future, which would be freed on shutdown.
        let is_static = false;

        Self::__cstr_with_static(c_str, is_static)
    }

    /// Creates a `StringName` from a static ASCII/Latin-1 `c"string"`.
    ///
    /// If `is_static` is true, avoids unnecessary copies and allocations and directly uses the backing buffer. However, this must
    /// be stored in an actual `static` to not cause leaks/error messages with Godot. For literals, use `is_static=false`.
    ///
    /// Note that while Latin-1 encoding is the most common encoding for c-strings, it isn't a requirement. So if your c-string
    /// uses a different encoding (e.g. UTF-8), it is possible that some characters will not show up as expected.
    ///
    /// # Safety
    /// `c_str` must be a static c-string that remains valid for the entire program duration.
    ///
    /// # Example
    /// ```no_run
    /// use godot::builtin::StringName;
    ///
    /// // '±' is a Latin-1 character with codepoint 0xB1. Note that this is not UTF-8, where it would need two bytes.
    /// let sname = StringName::__cstr(c"\xb1 Latin-1 string");
    /// ```
    #[doc(hidden)] // Private for now. Needs API discussion, also regarding overlap with try_from_cstr().
    pub fn __cstr_with_static(c_str: &'static std::ffi::CStr, is_static: bool) -> Self {
        // SAFETY: c_str is nul-terminated and remains valid for entire program duration.
        unsafe {
            Self::new_with_string_uninit(|ptr| {
                sys::interface_fn!(string_name_new_with_latin1_chars)(
                    ptr,
                    c_str.as_ptr(),
                    sys::conv::bool_to_sys(is_static),
                )
            })
        }
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
    const VARIANT_TYPE: ExtVariantType = ExtVariantType::Concrete(sys::VariantType::STRING_NAME);

    ffi_methods! { type sys::GDExtensionTypePtr = *mut Opaque; .. }
}

meta::impl_godot_as_self!(StringName: ByRef);

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

impl_shared_string_api! {
    builtin: StringName,
    find_builder: ExStringNameFind,
    split_builder: ExStringNameSplit,
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

impl From<&str> for StringName {
    fn from(string: &str) -> Self {
        let utf8 = string.as_bytes();

        // SAFETY: Rust guarantees validity and range of string.
        unsafe {
            Self::new_with_string_uninit(|ptr| {
                sys::interface_fn!(string_name_new_with_utf8_chars_and_len)(
                    ptr,
                    utf8.as_ptr() as *const std::ffi::c_char,
                    utf8.len() as i64,
                );
            })
        }
    }
}

impl From<&String> for StringName {
    fn from(value: &String) -> Self {
        value.as_str().into()
    }
}

impl From<&GString> for StringName {
    /// See also [`GString::to_string_name()`].
    fn from(string: &GString) -> Self {
        unsafe {
            Self::new_with_uninit(|self_ptr| {
                let ctor = sys::builtin_fn!(string_name_from_string);
                let args = [string.sys()];
                ctor(self_ptr, args.as_ptr());
            })
        }
    }
}

impl From<&NodePath> for StringName {
    fn from(path: &NodePath) -> Self {
        Self::from(&GString::from(path))
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Ordering

/// Type that implements `Ord` for `StringNames`.
///
/// See [`StringName::transient_ord()`].
pub struct TransientStringNameOrd<'a>(&'a StringName);

impl PartialEq for TransientStringNameOrd<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl Eq for TransientStringNameOrd<'_> {}

impl PartialOrd for TransientStringNameOrd<'_> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for TransientStringNameOrd<'_> {
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
    use std::fmt::Formatter;

    use serde::de::{Error, Visitor};
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    use super::*;

    // For "Available on crate feature `serde`" in docs. Cannot be inherited from module. Also does not support #[derive] (e.g. in Vector2).
    #[cfg_attr(published_docs, doc(cfg(feature = "serde")))]
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

    #[cfg_attr(published_docs, doc(cfg(feature = "serde")))]
    impl<'de> Deserialize<'de> for StringName {
        #[inline]
        fn deserialize<D>(deserializer: D) -> Result<Self, <D as Deserializer<'de>>::Error>
        where
            D: Deserializer<'de>,
        {
            struct StringNameVisitor;
            impl Visitor<'_> for StringNameVisitor {
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

// TODO(v0.4.x): consider re-exposing in public API. Open questions: thread-safety, performance, memory leaks, global overhead.
// Possibly in a more general StringName cache, similar to ClassId. See https://github.com/godot-rust/gdext/pull/1316.
/// Creates and gets a reference to a static `StringName` from a ASCII/Latin-1 `c"string"`.
///
/// This is the fastest way to create a StringName repeatedly, with the result being cached and never released, like `SNAME` in Godot source code. Suitable for scenarios where high performance is required.
#[macro_export]
macro_rules! static_sname {
    ($str:literal) => {{
        use std::sync::OnceLock;

        let c_str: &'static std::ffi::CStr = $str;
        static SNAME: OnceLock<StringName> = OnceLock::new();
        SNAME.get_or_init(|| StringName::__cstr_with_static(c_str, true))
    }};
}
