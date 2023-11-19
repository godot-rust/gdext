/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::{convert::Infallible, ffi::c_char, fmt, str::FromStr};

use godot_ffi as sys;
use sys::types::OpaqueString;
use sys::{ffi_methods, interface_fn, GodotFfi};

use crate::builtin::inner;
use crate::builtin::meta::impl_godot_as_self;

use super::string_chars::validate_unicode_scalar_sequence;
use super::{NodePath, StringName};

#[deprecated = "Renamed to `GString`, will soon be removed."]
pub type GodotString = GString;

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
/// are done transparently. For auto-generated binding APIs in `godot::engine`, both parameters and return types are `GString`.
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
/// # Other string types
///
/// Godot also provides two separate string classes with slightly different semantics: [`StringName`] and [`NodePath`].
#[doc(alias = "String")]
// #[repr] is needed on GString itself rather than the opaque field, because PackedStringArray::as_slice() relies on a packed representation.
#[repr(transparent)]
pub struct GString {
    opaque: OpaqueString,
}

impl GString {
    /// Construct a new empty GString.
    pub fn new() -> Self {
        Self::default()
    }

    fn from_opaque(opaque: OpaqueString) -> Self {
        Self { opaque }
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
    ///
    /// Note: This operation is *O*(*n*). Consider using [`chars_unchecked`][Self::chars_unchecked]
    /// if you can make sure the string is a valid UTF-32.
    pub fn chars_checked(&self) -> &[char] {
        unsafe {
            let s = self.string_sys();
            let len = interface_fn!(string_to_utf32_chars)(s, std::ptr::null_mut(), 0);
            let ptr = interface_fn!(string_operator_index_const)(s, 0);

            // Even when len == 0, from_raw_parts requires ptr != 0
            if ptr.is_null() {
                return &[];
            }

            validate_unicode_scalar_sequence(std::slice::from_raw_parts(ptr, len as usize))
                .expect("GString::chars_checked: string contains invalid unicode scalar values")
        }
    }

    /// Gets the internal chars slice from a [`GString`].
    ///
    /// # Safety
    ///
    /// Make sure the string only contains valid unicode scalar values, currently
    /// Godot allows for unpaired surrogates and out of range code points to be appended
    /// into the string.
    pub unsafe fn chars_unchecked(&self) -> &[char] {
        let s = self.string_sys();
        let len = interface_fn!(string_to_utf32_chars)(s, std::ptr::null_mut(), 0);
        let ptr = interface_fn!(string_operator_index_const)(s, 0);

        // Even when len == 0, from_raw_parts requires ptr != 0
        if ptr.is_null() {
            return &[];
        }
        std::slice::from_raw_parts(ptr as *const char, len as usize)
    }

    ffi_methods! {
        type sys::GDExtensionStringPtr = *mut Opaque;

        fn from_string_sys = from_sys;
        fn from_string_sys_init = from_sys_init;
        fn string_sys = sys;
    }

    /// Move `self` into a system pointer. This transfers ownership and thus does not call the destructor.
    ///
    /// # Safety
    /// `dst` must be a pointer to a `GString` which is suitable for ffi with Godot.
    pub(crate) unsafe fn move_string_ptr(self, dst: sys::GDExtensionStringPtr) {
        self.move_return_ptr(dst as *mut _, sys::PtrcallType::Standard);
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
        sys::VariantType::String
    }

    ffi_methods! { type sys::GDExtensionTypePtr = *mut Opaque;
        fn from_sys;
        fn sys;
        fn from_sys_init;
        fn move_return_ptr;
    }

    unsafe fn from_arg_ptr(ptr: sys::GDExtensionTypePtr, _call_type: sys::PtrcallType) -> Self {
        let string = Self::from_sys(ptr);
        std::mem::forget(string.clone());
        string
    }

    unsafe fn from_sys_init_default(init_fn: impl FnOnce(sys::GDExtensionTypePtr)) -> Self {
        let mut result = Self::default();
        init_fn(result.sys_mut());
        result
    }
}

impl_godot_as_self!(GString);

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
        let s: String = self.chars_checked().iter().collect();
        f.write_str(s.as_str())
    }
}

/// Uses literal syntax from GDScript: `"string"`
impl fmt::Debug for GString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = String::from(self);
        write!(f, "\"{s}\"")
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Conversion from/into Rust string-types

impl<S> From<S> for GString
where
    S: AsRef<str>,
{
    fn from(s: S) -> Self {
        let bytes = s.as_ref().as_bytes();

        unsafe {
            Self::from_string_sys_init(|string_ptr| {
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
            sys::from_sys_init_or_init_default::<Self>(|self_ptr| {
                let ctor = sys::builtin_fn!(string_from_string_name);
                let args = [string.sys_const()];
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
            sys::from_sys_init_or_init_default::<Self>(|self_ptr| {
                let ctor = sys::builtin_fn!(string_from_node_path);
                let args = [path.sys_const()];
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
