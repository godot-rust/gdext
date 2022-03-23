use std::ffi::CString;
use std::{convert::Infallible, mem::MaybeUninit, str::FromStr};

use gdext_sys::types::OpaqueString;
use gdext_sys::{self as sys, interface_fn};
use once_cell::sync::Lazy;

use crate::PtrCallArg;

#[repr(C, align(8))]
pub struct GodotString {
    opaque: OpaqueString,
}

impl GodotString {
    #[doc(hidden)]
    pub fn as_mut_ptr(&mut self) -> sys::GDNativeStringPtr {
        &mut self.opaque as *mut _ as sys::GDNativeStringPtr
    }

    #[doc(hidden)]
    pub fn as_ptr(&self) -> sys::GDNativeStringPtr {
        &self.opaque as *const _ as sys::GDNativeStringPtr
    }

    #[doc(hidden)]
    pub fn as_c_string(&self) -> *const std::os::raw::c_char {
        // TODO make this less hacky and leaky
        let s: String = self.into();

        let c = CString::new(s).unwrap();
        let ptr = c.as_ptr();
        std::mem::forget(c);
        ptr
    }

    pub fn new() -> Self {
        unsafe {
            static CONSTR: Lazy<
                unsafe extern "C" fn(sys::GDNativeTypePtr, *const sys::GDNativeTypePtr),
            > = Lazy::new(|| unsafe {
                interface_fn!(variant_get_ptr_constructor)(
                    sys::GDNativeVariantType_GDNATIVE_VARIANT_TYPE_STRING,
                    0,
                )
                .unwrap()
            });

            let mut opaque = MaybeUninit::<OpaqueString>::uninit();
            CONSTR(
                opaque.as_mut_ptr() as sys::GDNativeTypePtr,
                std::ptr::null(),
            );

            GodotString {
                opaque: opaque.assume_init(),
            }
        }
    }

    pub fn from(s: &str) -> Self {
        Self::from_str(s).unwrap()
    }
}

impl Default for GodotString {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for GodotString {
    fn clone(&self) -> Self {
        unsafe {
            let mut s = Self::new();

            static CONSTR: Lazy<
                unsafe extern "C" fn(sys::GDNativeTypePtr, *const sys::GDNativeTypePtr),
            > = Lazy::new(|| unsafe {
                interface_fn!(variant_get_ptr_constructor)(
                    sys::GDNativeVariantType_GDNATIVE_VARIANT_TYPE_STRING,
                    1,
                )
                .unwrap()
            });
            CONSTR(s.as_mut_ptr(), self.as_ptr() as *const _);
            s
        }
    }
}

impl From<String> for GodotString {
    fn from(s: String) -> GodotString {
        GodotString::from(s.as_str())
    }
}

impl From<&str> for GodotString {
    fn from(val: &str) -> Self {
        GodotString::from(val)
    }
}

impl std::fmt::Display for GodotString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = String::from(self);
        f.write_str(s.as_str())
    }
}

impl From<&GodotString> for String {
    fn from(string: &GodotString) -> Self {
        unsafe {
            let len = interface_fn!(string_to_utf8_chars)(string.as_ptr(), std::ptr::null_mut(), 0);

            assert!(len >= 0);
            let mut buf = vec![0u8; len as usize];

            interface_fn!(string_to_utf8_chars)(string.as_ptr(), buf.as_mut_ptr() as *mut i8, len);

            String::from_utf8_unchecked(buf)
        }
    }
}

impl FromStr for GodotString {
    type Err = Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut opaque = MaybeUninit::<OpaqueString>::uninit();

        let b = s.as_bytes();
        unsafe {
            interface_fn!(string_new_with_utf8_chars_and_len)(
                &mut opaque as *mut _ as sys::GDNativeStringPtr,
                b.as_ptr() as *mut _,
                b.len() as i64,
            );

            Ok(Self {
                opaque: opaque.assume_init(),
            })
        }
    }
}

impl Drop for GodotString {
    fn drop(&mut self) {
        unsafe {
            static DESTR: Lazy<unsafe extern "C" fn(sys::GDNativeTypePtr)> = Lazy::new(|| unsafe {
                interface_fn!(variant_get_ptr_destructor)(
                    sys::GDNativeVariantType_GDNATIVE_VARIANT_TYPE_STRING,
                )
                .unwrap()
            });
            DESTR(self.as_mut_ptr());
        }
    }
}

impl PtrCallArg for GodotString {
    unsafe fn from_ptr_call_arg(arg: *const gdext_sys::GDNativeTypePtr) -> Self {
        // C++: arg is String**
        //Clone::clone(&*(arg as *mut GodotString))
        //let arg = *arg;
        Self {
            opaque: OpaqueString::from_value_sys(*arg as *mut _),
        }
    }

    unsafe fn to_ptr_call_arg(self, arg: gdext_sys::GDNativeTypePtr) {
        std::ptr::write(arg as *mut GodotString, self);
    }
}

// While this is a nice optimisation for ptrcalls, it's not easily possible
// to pass in &GodotString when doing varcalls.
/*
impl PtrCallArg for &GodotString {
    unsafe fn from_ptr_call_arg(arg: *const gdext_sys::GDNativeTypePtr) -> Self {
        &*(*arg as *const GodotString)
    }

    unsafe fn to_ptr_call_arg(self, arg: gdext_sys::GDNativeTypePtr) {
        std::ptr::write(arg as *mut GodotString, self.clone());
    }
}
*/
