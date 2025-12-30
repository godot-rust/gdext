/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::convert::Infallible;
use std::fmt;
use std::fmt::Write;

use godot_ffi as sys;
use sys::types::OpaqueString;
use sys::{ffi_methods, interface_fn, ExtVariantType, GodotFfi};

use crate::builtin::strings::{pad_if_needed, Encoding};
use crate::builtin::{inner, NodePath, StringName, Variant};
use crate::meta::error::StringError;
use crate::meta::AsArg;
use crate::{impl_shared_string_api, meta};

/// Godot's reference counted string type.
///
/// This is the Rust binding of GDScript's `String` type. It represents the native string class used within the Godot engine,
/// and as such has different memory layout and characteristics than `std::string::String`.
///
/// `GString` uses copy-on-write semantics and is cheap to clone. Modifying a string may trigger a copy, if that instance shares
/// its backing storage with other strings.
///
/// Note that `GString` is not immutable, but it offers a very limited set of write APIs. Most operations return new strings.
/// In order to modify Godot strings, it's often easiest to convert them to Rust strings, perform the modifications and convert back.
///
/// # `GString` vs. `String`
///
/// When interfacing with the Godot engine API, you often have the choice between `String` and `GString`. In user-declared methods
/// exposed to Godot through the `#[func]` attribute, both types can be used as parameters and return types, and conversions
/// are done transparently. For auto-generated binding APIs in `godot::classes`, both parameters and return types are `GString`.
/// Parameters are declared as `impl AsArg<GString>`, allowing you to be more flexible with arguments such as `"some_string"`.
///
/// As a general guideline, use `GString` if:
/// * your strings are very large, so you can avoid copying them
/// * you need specific operations only available in Godot (e.g. `sha256_text()`, `c_escape()`, ...)
/// * you primarily pass them between different Godot APIs, without string processing in user code
///
/// Use Rust's `String` if:
/// * you need to modify the string
/// * you would like to decouple part of your code from Godot (e.g. independent game logic, standalone tests)
/// * you want a standard type for interoperability with third-party code (e.g. `regex` crate)
/// * you have a large number of method calls per string instance (which are more expensive due to indirectly calling into Godot)
/// * you need UTF-8 encoding (`GString` uses UTF-32)
///
/// # Null bytes
///
/// Note that Godot ignores any bytes after a null-byte. This means that for instance `"hello, world!"` and `"hello, world!\0 ignored by Godot"`
/// will be treated as the same string if converted to a `GString`.
///
/// # All string types
///
/// | Intended use case | String type                                |
/// |-------------------|--------------------------------------------|
/// | General purpose   | **`GString`**                              |
/// | Interned names    | [`StringName`][crate::builtin::StringName] |
/// | Scene-node paths  | [`NodePath`][crate::builtin::NodePath]     |
///
/// # Godot docs
///
/// [`String` (stable)](https://docs.godotengine.org/en/stable/classes/class_string.html)
#[doc(alias = "String")]
// #[repr] is needed on GString itself rather than the opaque field, because PackedStringArray::as_slice() relies on a packed representation.
#[repr(transparent)]
pub struct GString {
    _opaque: OpaqueString,
}

// SAFETY: The Godot implementation of String uses an atomic copy on write pointer, making this thread-safe as we never write to it unless we own it.
unsafe impl Send for GString {}

impl GString {
    /// Construct a new empty `GString`.
    pub fn new() -> Self {
        Self::default()
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

    /// Convert string from C-string with given encoding, returning `Err` on validation errors.
    ///
    /// Convenience function for [`try_from_bytes()`](Self::try_from_bytes); see its docs for more information.
    pub fn try_from_cstr(cstr: &std::ffi::CStr, encoding: Encoding) -> Result<Self, StringError> {
        Self::try_from_bytes_with_nul_check(cstr.to_bytes(), encoding, false)
    }

    pub(super) fn try_from_bytes_with_nul_check(
        bytes: &[u8],
        encoding: Encoding,
        check_nul: bool,
    ) -> Result<Self, StringError> {
        match encoding {
            Encoding::Ascii => {
                // If the bytes are ASCII, we can fall back to Latin-1, which is always valid (except for NUL).
                // is_ascii() does *not* check for the NUL byte, so the check in the Latin-1 branch is still necessary.
                if bytes.is_ascii() {
                    Self::try_from_bytes_with_nul_check(bytes, Encoding::Latin1, check_nul)
                        .map_err(|_e| StringError::new("intermediate NUL byte in ASCII string"))
                } else {
                    Err(StringError::new("invalid ASCII"))
                }
            }
            Encoding::Latin1 => {
                // Intermediate NUL bytes are not accepted in Godot. Both ASCII + Latin-1 encodings need to explicitly check for this.
                if check_nul && bytes.contains(&0) {
                    // Error overwritten when called from ASCII branch.
                    return Err(StringError::new("intermediate NUL byte in Latin-1 string"));
                }

                let s = unsafe {
                    Self::new_with_string_uninit(|string_ptr| {
                        let ctor = interface_fn!(string_new_with_latin1_chars_and_len);
                        ctor(
                            string_ptr,
                            bytes.as_ptr() as *const std::ffi::c_char,
                            bytes.len() as i64,
                        );
                    })
                };
                Ok(s)
            }
            Encoding::Utf8 => {
                // from_utf8() also checks for intermediate NUL bytes.
                let utf8 = std::str::from_utf8(bytes);

                utf8.map(GString::from)
                    .map_err(|e| StringError::with_source("invalid UTF-8", e))
            }
        }
    }

    /// Number of characters in the string.
    ///
    /// _Godot equivalent: `length`_
    #[doc(alias = "length")]
    pub fn len(&self) -> usize {
        self.as_inner().length().try_into().unwrap()
    }

    crate::declare_hash_u32_method! {
        /// Returns a 32-bit integer hash value representing the string.
    }

    /// Gets the UTF-32 character slice from a `GString`.
    pub fn chars(&self) -> &[char] {
        // SAFETY: Since 4.1, Godot ensures valid UTF-32, making interpreting as char slice safe.
        // See https://github.com/godotengine/godot/pull/74760.
        let (ptr, len) = self.raw_slice();

        // Even when len == 0, from_raw_parts requires ptr != null.
        if ptr.is_null() {
            return &[];
        }

        unsafe { std::slice::from_raw_parts(ptr, len) }
    }

    /// Returns the raw pointer and length of the internal UTF-32 character array.
    ///
    /// This is used by `StringName::chars()` in Godot 4.5+ where the buffer is shared via reference counting.
    /// Since Godot 4.1, the buffer contains valid UTF-32.
    pub(crate) fn raw_slice(&self) -> (*const char, usize) {
        let s = self.string_sys();

        let len: sys::GDExtensionInt;
        let ptr: *const sys::char32_t;
        unsafe {
            len = interface_fn!(string_to_utf32_chars)(s, std::ptr::null_mut(), 0);
            ptr = interface_fn!(string_operator_index_const)(s, 0);
        }

        (ptr.cast(), len as usize)
    }

    ffi_methods! {
        type sys::GDExtensionStringPtr = *mut Self;

        fn new_from_string_sys = new_from_sys;
        fn new_with_string_uninit = new_with_uninit;
        fn string_sys = sys;
        fn string_sys_mut = sys_mut;
    }

    /// Consumes self and turns it into a sys-ptr, should be used together with [`from_owned_string_sys`](Self::from_owned_string_sys).
    ///
    /// This will leak memory unless `from_owned_string_sys` is called on the returned pointer.
    pub(crate) fn into_owned_string_sys(self) -> sys::GDExtensionStringPtr {
        sys::static_assert_eq_size_align!(StringName, sys::types::OpaqueString);

        let leaked = Box::into_raw(Box::new(self));
        leaked.cast()
    }

    /// Creates a `GString` from a sys-ptr without incrementing the refcount.
    ///
    /// # Safety
    ///
    /// * Must only be used on a pointer returned from a call to [`into_owned_string_sys`](Self::into_owned_string_sys).
    /// * Must not be called more than once on the same pointer.
    #[deny(unsafe_op_in_unsafe_fn)]
    pub(crate) unsafe fn from_owned_string_sys(ptr: sys::GDExtensionStringPtr) -> Self {
        sys::static_assert_eq_size_align!(StringName, sys::types::OpaqueString);

        let ptr = ptr.cast::<Self>();

        // SAFETY: `ptr` was returned from a call to `into_owned_string_sys`, which means it was created by a call to
        // `Box::into_raw`, thus we can use `Box::from_raw` here. Additionally, this is only called once on this pointer.
        let boxed = unsafe { Box::from_raw(ptr) };
        *boxed
    }

    /// Convert a `GString` sys pointer to a mutable reference with unbounded lifetime.
    ///
    /// # Safety
    ///
    /// - `ptr` must point to a live `GString` for the duration of `'a`.
    /// - Must be exclusive - no other reference to given `GString` instance can exist for the duration of `'a`.
    pub(crate) unsafe fn borrow_string_sys_mut<'a>(ptr: sys::GDExtensionStringPtr) -> &'a mut Self {
        sys::static_assert_eq_size_align!(StringName, sys::types::OpaqueString);
        &mut *(ptr.cast::<GString>())
    }

    /// Moves this string into a string sys pointer. This is the same as using [`GodotFfi::move_return_ptr`].
    ///
    /// # Safety
    ///
    /// `dst` must be a valid string pointer.
    pub(crate) unsafe fn move_into_string_ptr(self, dst: sys::GDExtensionStringPtr) {
        let dst: sys::GDExtensionTypePtr = dst.cast();

        self.move_return_ptr(dst, sys::PtrcallType::Standard);
    }

    meta::declare_arg_method! {
        /// Use as argument for an [`impl AsArg<StringName|NodePath>`][crate::meta::AsArg] parameter.
        ///
        /// This is a convenient way to convert arguments of similar string types.
        ///
        /// # Example
        /// [`Node::has_node()`][crate::classes::Node::has_node] takes `NodePath`, let's pass a `GString`:
        /// ```no_run
        /// # use godot::prelude::*;
        /// let name = GString::from("subnode");
        ///
        /// let node = Node::new_alloc();
        /// if node.has_node(name.arg()) {
        ///     // ...
        /// }
        /// ```
    }

    #[doc(hidden)]
    pub fn as_inner(&self) -> inner::InnerString<'_> {
        inner::InnerString::from_outer(self)
    }
}

// SAFETY:
// - `move_return_ptr`
//   Nothing special needs to be done beyond a `std::mem::swap` when returning a String.
//   So we can just use `ffi_methods`.
//
// - `from_arg_ptr`
//   Strings are properly initialized through a `from_sys` call, but the ref-count should be
//   incremented as that is the callee's responsibility. Which we do by calling
//   `std::mem::forget(string.clone())`.
unsafe impl GodotFfi for GString {
    const VARIANT_TYPE: ExtVariantType = ExtVariantType::Concrete(sys::VariantType::STRING);

    ffi_methods! { type sys::GDExtensionTypePtr = *mut Self; .. }
}

meta::impl_godot_as_self!(GString: ByRef);

impl_builtin_traits! {
    for GString {
        Default => string_construct_default;
        Clone => string_construct_copy;
        Drop => string_destroy;
        Eq => string_operator_equal;
        Ord => string_operator_less;
        Hash;
    }
}

impl_shared_string_api! {
    builtin: GString,
    builtin_mod: gstring,
}

impl fmt::Display for GString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        pad_if_needed(f, |f| {
            for ch in self.chars() {
                f.write_char(*ch)?;
            }

            Ok(())
        })
    }
}

/// Uses literal syntax from GDScript: `"string"`
impl fmt::Debug for GString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Reuse Display impl.
        write!(f, "\"{self}\"")
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Comparison with Rust strings

// API design:
// * StringName and NodePath don't implement PartialEq<&str> yet, because they require allocation (convert to GString).
//   == should ideally not allocate.
// * Reverse `impl PartialEq<GString> for &str` is not implemented now. Comparisons usually take the form of variable == "literal".
//   Can be added later if there are good use-cases.

impl PartialEq<&str> for GString {
    fn eq(&self, other: &&str) -> bool {
        self.chars().iter().copied().eq(other.chars())
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Conversion from/into Rust string-types

impl From<&str> for GString {
    fn from(s: &str) -> Self {
        let bytes = s.as_bytes();

        unsafe {
            Self::new_with_string_uninit(|string_ptr| {
                #[cfg(before_api = "4.3")]
                let ctor = interface_fn!(string_new_with_utf8_chars_and_len);
                #[cfg(since_api = "4.3")]
                let ctor = interface_fn!(string_new_with_utf8_chars_and_len2);

                ctor(
                    string_ptr,
                    bytes.as_ptr() as *const std::ffi::c_char,
                    bytes.len() as i64,
                );
            })
        }
    }
}

impl From<&[char]> for GString {
    fn from(chars: &[char]) -> Self {
        // SAFETY: A `char` value is by definition a valid Unicode code point.
        unsafe {
            Self::new_with_string_uninit(|string_ptr| {
                let ctor = interface_fn!(string_new_with_utf32_chars_and_len);
                ctor(
                    string_ptr,
                    chars.as_ptr() as *const sys::char32_t,
                    chars.len() as i64,
                );
            })
        }
    }
}

impl From<&String> for GString {
    fn from(value: &String) -> Self {
        value.as_str().into()
    }
}

impl From<&GString> for String {
    fn from(string: &GString) -> Self {
        unsafe {
            let len =
                interface_fn!(string_to_utf8_chars)(string.string_sys(), std::ptr::null_mut(), 0);

            assert!(len >= 0);
            let mut buf = vec![0u8; len as usize];

            interface_fn!(string_to_utf8_chars)(
                string.string_sys(),
                buf.as_mut_ptr() as *mut std::ffi::c_char,
                len,
            );

            // Note: could use from_utf8_unchecked() but for now prefer safety
            String::from_utf8(buf).expect("String::from_utf8")
        }
    }
}

impl From<GString> for String {
    /// Converts this `GString` to a `String`.
    ///
    /// This is identical to `String::from(&string)`, and as such there is no performance benefit.
    fn from(string: GString) -> Self {
        Self::from(&string)
    }
}

impl std::str::FromStr for GString {
    type Err = Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self::from(s))
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Conversion from other Godot string-types

impl From<&StringName> for GString {
    fn from(string: &StringName) -> Self {
        unsafe {
            Self::new_with_uninit(|self_ptr| {
                let ctor = sys::builtin_fn!(string_from_string_name);
                let args = [string.sys()];
                ctor(self_ptr, args.as_ptr());
            })
        }
    }
}

impl From<&NodePath> for GString {
    fn from(path: &NodePath) -> Self {
        unsafe {
            Self::new_with_uninit(|self_ptr| {
                let ctor = sys::builtin_fn!(string_from_node_path);
                let args = [path.sys()];
                ctor(self_ptr, args.as_ptr());
            })
        }
    }
}

#[cfg(feature = "serde")]
mod serialize {
    use std::fmt::Formatter;

    use serde::de::{Error, Visitor};
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    use super::*;

    // For "Available on crate feature `serde`" in docs. Cannot be inherited from module. Also does not support #[derive] (e.g. in Vector2).
    #[cfg_attr(published_docs, doc(cfg(feature = "serde")))]
    impl Serialize for GString {
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
    impl<'de> Deserialize<'de> for GString {
        #[inline]
        fn deserialize<D>(deserializer: D) -> Result<Self, <D as Deserializer<'de>>::Error>
        where
            D: Deserializer<'de>,
        {
            struct GStringVisitor;
            impl Visitor<'_> for GStringVisitor {
                type Value = GString;

                fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
                    formatter.write_str("a GString")
                }

                fn visit_str<E>(self, s: &str) -> Result<Self::Value, E>
                where
                    E: Error,
                {
                    Ok(GString::from(s))
                }
            }

            deserializer.deserialize_str(GStringVisitor)
        }
    }
}
