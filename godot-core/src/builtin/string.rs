/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::{convert::Infallible, fmt, str::FromStr};

use godot_ffi as sys;
use sys::types::OpaqueString;
use sys::{ffi_methods, interface_fn, GodotFfi};

use super::{
    string_chars::validate_unicode_scalar_sequence, FromVariant, ToVariant, Variant,
    VariantConversionError,
};

#[repr(C, align(8))]
pub struct GodotString {
    opaque: OpaqueString,
}

impl GodotString {
    pub fn new() -> Self {
        Self::default()
    }

    fn from_opaque(opaque: OpaqueString) -> Self {
        Self { opaque }
    }

    ffi_methods! {
        type sys::GDExtensionStringPtr = *mut Opaque;

        fn from_string_sys = from_sys;
        fn from_string_sys_init = from_sys_init;
        fn string_sys = sys;
    }

    /// Move `self` into a system pointer.
    ///
    /// # Safety
    /// `dst` must be a pointer to a `GodotString` which is suitable for ffi with Godot.
    pub unsafe fn move_string_ptr(self, dst: sys::GDExtensionStringPtr) {
        self.move_return_ptr(dst as *mut _, sys::CallType::Standard);
    }

    /// Gets the internal chars slice from a [`GodotString`].
    ///
    /// Note: This operation is *O*(*n*). Consider using [`chars_unchecked`]
    /// if you can make sure the string is a valid UTF-32.
    pub fn chars_checked(&self) -> &[char] {
        unsafe {
            let s = self.string_sys();
            let len = interface_fn!(string_to_utf32_chars)(s, std::ptr::null_mut(), 0);
            let ptr = interface_fn!(string_operator_index_const)(s, 0);

            validate_unicode_scalar_sequence(std::slice::from_raw_parts(ptr, len as usize))
                .expect("GodotString::chars_checked: string contains invalid unicode scalar values")
        }
    }

    /// Gets the internal chars slice from a [`GodotString`].
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
        std::slice::from_raw_parts(ptr as *const char, len as usize)
    }
}

unsafe impl GodotFfi for GodotString {
    ffi_methods! { type sys::GDExtensionTypePtr = *mut Opaque;
        fn from_sys;
        fn sys;
        fn from_sys_init;
        // SAFETY:
        // Nothing special needs to be done beyond a `std::mem::swap` when returning a GodotString.
        fn move_return_ptr;
    }

    // SAFETY:
    // GodotStrings are properly initialized through a `from_sys` call, but the ref-count should be
    // incremented as that is the callee's responsibility.
    //
    // Using `std::mem::forget(string.share())` increments the ref count.
    unsafe fn from_arg_ptr(ptr: sys::GDExtensionTypePtr, _call_type: sys::CallType) -> Self {
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

impl_builtin_traits! {
    for GodotString {
        Default => string_construct_default;
        Clone => string_construct_copy;
        Drop => string_destroy;
        Eq => string_operator_equal;
        Ord => string_operator_less;
    }
}

impl<S> From<S> for GodotString
where
    S: AsRef<str>,
{
    fn from(s: S) -> Self {
        let bytes = s.as_ref().as_bytes();

        unsafe {
            Self::from_string_sys_init(|string_ptr| {
                let ctor = interface_fn!(string_new_with_utf8_chars_and_len);
                ctor(string_ptr, bytes.as_ptr() as *const i8, bytes.len() as i64);
            })
        }
    }
}

impl From<&GodotString> for String {
    fn from(string: &GodotString) -> Self {
        unsafe {
            let len =
                interface_fn!(string_to_utf8_chars)(string.string_sys(), std::ptr::null_mut(), 0);

            assert!(len >= 0);
            let mut buf = vec![0u8; len as usize];

            interface_fn!(string_to_utf8_chars)(
                string.string_sys(),
                buf.as_mut_ptr() as *mut i8,
                len,
            );

            // Note: could use from_utf8_unchecked() but for now prefer safety
            String::from_utf8(buf).expect("String::from_utf8")
        }
    }
}

// TODO From<&NodePath> + test

impl FromStr for GodotString {
    type Err = Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self::from(s))
    }
}

impl fmt::Display for GodotString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = String::from(self);
        f.write_str(s.as_str())
    }
}

/// Uses literal syntax from GDScript: `"string"`
impl fmt::Debug for GodotString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = String::from(self);
        write!(f, "\"{s}\"")
    }
}

impl ToVariant for &str {
    fn to_variant(&self) -> Variant {
        GodotString::from(*self).to_variant()
    }
}

impl ToVariant for String {
    fn to_variant(&self) -> Variant {
        GodotString::from(self).to_variant()
    }
}

impl FromVariant for String {
    fn try_from_variant(variant: &Variant) -> Result<Self, VariantConversionError> {
        Ok(GodotString::try_from_variant(variant)?.to_string())
    }
}

// While this is a nice optimisation for ptrcalls, it's not easily possible
// to pass in &GodotString when doing varcalls.
/*
impl PtrCall for &GodotString {
    unsafe fn from_ptr_call_arg(arg: *const godot_ffi::GDExtensionTypePtr) -> Self {
        &*(*arg as *const GodotString)
    }

    unsafe fn to_ptr_call_arg(self, arg: godot_ffi::GDExtensionTypePtr) {
        std::ptr::write(arg as *mut GodotString, self.clone());
    }
}
*/
