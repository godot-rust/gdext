/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::{convert::Infallible, fmt, str::FromStr};

use gdext_sys::{self as sys, ffi_methods, interface_fn, GodotFfi};
use sys::types::OpaqueString;

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
        type sys::GDNativeStringPtr = *mut Opaque;

        fn from_string_sys = from_sys;
        fn from_string_sys_init = from_sys_init;
        fn string_sys = sys;
        fn write_string_sys = write_sys;
    }

    // #[doc(hidden)]
    // pub fn leak_string_sys(self) -> sys::GDNativeStringPtr {
    //     let ptr = self.string_sys();
    //     std::mem::forget(self);
    //     ptr
    // }
}

impl GodotFfi for GodotString {
    ffi_methods! {
        type sys::GDNativeTypePtr = *mut Opaque;
        fn from_sys;
        fn sys;
        fn write_sys;
    }

    unsafe fn from_sys_init(init_fn: impl FnOnce(sys::GDNativeTypePtr)) -> Self {
        // Can't use uninitialized pointer -- String CoW implementation in C++ expects that on assignment,
        // the target CoW pointer is either initialized or nullptr

        let mut result = Self::default();
        init_fn(result.sys_mut());
        result
    }
}

impl Default for GodotString {
    fn default() -> Self {
        // Note: can't use from_sys_init(), as that calls the default constructor
        // (because most assignments expect initialized target type)

        let mut uninit = std::mem::MaybeUninit::<GodotString>::uninit();

        unsafe {
            let self_ptr = (*uninit.as_mut_ptr()).sys_mut();
            let ctor = sys::method_table().string_construct_default;
            ctor(self_ptr, std::ptr::null_mut());

            uninit.assume_init()
        }
    }
}

impl Clone for GodotString {
    fn clone(&self) -> Self {
        unsafe {
            Self::from_sys_init(|self_ptr| {
                let ctor = sys::method_table().string_construct_copy;
                let args = [self.sys()];
                ctor(self_ptr, args.as_ptr());
            })
        }
    }
}

impl From<&String> for GodotString {
    fn from(s: &String) -> GodotString {
        GodotString::from(s.as_str())
    }
}

impl From<String> for GodotString {
    fn from(s: String) -> GodotString {
        GodotString::from(s.as_str())
    }
}

impl From<&str> for GodotString {
    fn from(val: &str) -> Self {
        GodotString::from_str(val).expect("From<&str>")
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

impl FromStr for GodotString {
    type Err = Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let b = s.as_bytes();

        let result = unsafe {
            Self::from_string_sys_init(|string_ptr| {
                let ctor = interface_fn!(string_new_with_utf8_chars_and_len);
                ctor(string_ptr, b.as_ptr() as *const i8, b.len() as i64);
            })
        };

        Ok(result)
    }
}

impl fmt::Display for GodotString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = String::from(self);
        f.write_str(s.as_str())
    }
}

impl fmt::Debug for GodotString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = String::from(self);
        write!(f, "GodotString(\"{s}\")")
    }
}

impl_traits_as_sys! {
    for GodotString {
        Eq => string_operator_equal;
        Ord => string_operator_less;
    }
}

impl Drop for GodotString {
    fn drop(&mut self) {
        unsafe {
            let destructor = sys::method_table().string_destroy;
            destructor(self.sys_mut());
        }
    }
}

// While this is a nice optimisation for ptrcalls, it's not easily possible
// to pass in &GodotString when doing varcalls.
/*
impl PtrCall for &GodotString {
    unsafe fn from_ptr_call_arg(arg: *const gdext_sys::GDNativeTypePtr) -> Self {
        &*(*arg as *const GodotString)
    }

    unsafe fn to_ptr_call_arg(self, arg: gdext_sys::GDNativeTypePtr) {
        std::ptr::write(arg as *mut GodotString, self.clone());
    }
}
*/
