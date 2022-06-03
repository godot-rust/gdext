use std::{convert::Infallible, fmt, str::FromStr};

use gdext_sys as sys;
use sys::types::OpaqueString;
use sys::{impl_ffi_as_opaque_pointer, interface_fn, GodotFfi};

#[repr(C, align(8))]
pub struct GodotString {
    opaque: OpaqueString,
}

impl GodotString {
    pub fn new() -> Self {
        unsafe {
            Self::from_sys_init(|opaque_ptr| {
                let ctor = sys::get_cache().string_construct_default;
                ctor(opaque_ptr, std::ptr::null_mut());
            })
        }
    }

    fn from_opaque(opaque: OpaqueString) -> Self {
        Self { opaque }
    }

    impl_ffi_as_opaque_pointer!(sys::GDNativeStringPtr; from_string_sys, from_string_sys_init, string_sys, write_string_sys);
}

impl Default for GodotString {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for GodotString {
    fn clone(&self) -> Self {
        unsafe {
            Self::from_sys_init(|opaque_ptr| {
                let ctor = sys::get_cache().string_construct_copy;
                let sys = self.sys();
                ctor(opaque_ptr, std::ptr::addr_of!(sys));
            })
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
        GodotString::from_str(val).unwrap()
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
            String::from_utf8(buf).unwrap()
        }
    }
}

impl FromStr for GodotString {
    type Err = Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let b = s.as_bytes();

        let result = unsafe {
            Self::from_string_sys_init(|ptr| {
                let ctor = interface_fn!(string_new_with_utf8_chars_and_len);
                ctor(ptr, b.as_ptr() as *const i8, b.len() as i64);
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

impl PartialEq for GodotString {
    fn eq(&self, other: &Self) -> bool {
        unsafe {
            let operator = sys::get_cache().string_operator_equal;

            let mut result: bool = false;
            operator(self.sys(), other.sys(), result.sys_mut());
            result
        }
    }
}

impl Drop for GodotString {
    fn drop(&mut self) {
        unsafe {
            let destructor = sys::get_cache().string_destroy;
            destructor(self.sys_mut());
        }
    }
}

impl GodotFfi for GodotString {
    impl_ffi_as_opaque_pointer!();
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
