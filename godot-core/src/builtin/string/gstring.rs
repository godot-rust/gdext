/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::fmt::Write;
use std::{convert::Infallible, ffi::c_char, fmt, str::FromStr};

use godot_ffi as sys;
use sys::types::OpaqueString;
use sys::{ffi_methods, interface_fn, GodotFfi};

use crate::builtin::{inner, NodePath, StringName};

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
/// In the future, we will likely declare parameters as `impl Into<GString>`, allowing `String` or `&str` to be passed.
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
/// * you need UTF-8 encoding (`GString`'s encoding is platform-dependent and unspecified)
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
#[doc(alias = "String")]
// #[repr] is needed on GString itself rather than the opaque field, because PackedStringArray::as_slice() relies on a packed representation.
#[repr(transparent)]
pub struct GString {
    _opaque: OpaqueString,
}

impl GString {
    /// Construct a new empty GString.
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.as_inner().length().try_into().unwrap()
    }

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

    /// Gets the internal chars slice from a [`GString`].
    pub fn chars(&self) -> &[char] {
        // SAFETY: Godot 4.1 ensures valid UTF-32, making interpreting as char slice safe.
        // See https://github.com/godotengine/godot/pull/74760.
        unsafe {
            let s = self.string_sys();
            let len = interface_fn!(string_to_utf32_chars)(s, std::ptr::null_mut(), 0);
            let ptr = interface_fn!(string_operator_index_const)(s, 0);

            // Even when len == 0, from_raw_parts requires ptr != 0
            if ptr.is_null() {
                return &[];
            }

            std::slice::from_raw_parts(ptr as *const char, len as usize)
        }
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

    /// Moves this string into a string sys pointer. This is the same as using [`GodotFfi::move_return_ptr`].
    ///
    /// # Safety
    ///
    /// `dst` must be a valid string pointer.
    pub(crate) unsafe fn move_into_string_ptr(self, dst: sys::GDExtensionStringPtr) {
        let dst: sys::GDExtensionTypePtr = dst.cast();

        self.move_return_ptr(dst, sys::PtrcallType::Standard);
    }

    #[doc(hidden)]
    pub fn as_inner(&self) -> inner::InnerString {
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
    fn variant_type() -> sys::VariantType {
        sys::VariantType::STRING
    }

    ffi_methods! { type sys::GDExtensionTypePtr = *mut Self; .. }
}

crate::meta::impl_godot_as_self!(GString);

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

impl fmt::Display for GString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for ch in self.chars() {
            f.write_char(*ch)?;
        }

        Ok(())
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
// Conversion from/into Rust string-types

impl From<&str> for GString {
    fn from(s: &str) -> Self {
        let bytes = s.as_bytes();

        unsafe {
            Self::new_with_string_uninit(|string_ptr| {
                let ctor = interface_fn!(string_new_with_utf8_chars_and_len);
                ctor(
                    string_ptr,
                    bytes.as_ptr() as *const c_char,
                    bytes.len() as i64,
                );
            })
        }
    }
}

impl From<String> for GString {
    fn from(value: String) -> Self {
        value.as_str().into()
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
                buf.as_mut_ptr() as *mut c_char,
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

impl FromStr for GString {
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

impl From<StringName> for GString {
    /// Converts this `StringName` to a `GString`.
    ///
    /// This is identical to `GString::from(&string_name)`, and as such there is no performance benefit.
    fn from(string_name: StringName) -> Self {
        Self::from(&string_name)
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

impl From<NodePath> for GString {
    /// Converts this `NodePath` to a `GString`.
    ///
    /// This is identical to `GString::from(&path)`, and as such there is no performance benefit.
    fn from(path: NodePath) -> Self {
        Self::from(&path)
    }
}

#[cfg(feature = "serde")]
mod serialize {
    use super::*;
    use serde::de::{Error, Visitor};
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::fmt::Formatter;

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
            impl<'de> Visitor<'de> for GStringVisitor {
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
