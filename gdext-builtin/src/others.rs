// Stub for various other built-in classes, which are currently incomplete, but whose types
// are required for codegen
use crate::GodotString;
use gdext_sys as sys;
use sys::{impl_ffi_as_opaque_pointer, GodotFfi};

macro_rules! impl_builtin_stub {
    ($Class:ident, $OpaqueTy:ident) => {
        #[repr(C)]
        pub struct $Class {
            opaque: sys::types::$OpaqueTy,
        }

        impl $Class {
            fn from_opaque(opaque: sys::types::$OpaqueTy) -> Self {
                Self { opaque }
            }
        }

        impl GodotFfi for $Class {
            impl_ffi_as_opaque_pointer!();
        }
    };
}

impl_builtin_stub!(Array, OpaqueArray);
impl_builtin_stub!(Dictionary, OpaqueDictionary);
impl_builtin_stub!(Transform2D, OpaqueTransform2D);
impl_builtin_stub!(Transform3D, OpaqueTransform3D);
impl_builtin_stub!(NodePath, OpaqueNodePath);

#[repr(C)]
pub struct StringName {
    opaque: sys::types::OpaqueStringName,
}
impl StringName {
    fn from_opaque(opaque: sys::types::OpaqueStringName) -> Self {
        Self { opaque }
    }

    impl_ffi_as_opaque_pointer!(sys::GDNativeStringNamePtr; from_string_sys, from_string_sys_init, string_sys, write_string_sys);

    #[doc(hidden)]
    pub fn leak_string_sys(self) -> sys::GDNativeStringNamePtr {
        let ptr = self.string_sys();
        std::mem::forget(self);
        ptr
    }
}
impl GodotFfi for StringName {
    impl_ffi_as_opaque_pointer!();
}
impl Default for StringName {
    fn default() -> Self {
        unsafe {
            Self::from_sys_init(|self_ptr| {
                let ctor = sys::get_cache().string_name_construct_default;
                ctor(self_ptr, std::ptr::null_mut());
            })
        }
    }
}
impl From<&GodotString> for StringName {
    fn from(s: &GodotString) -> Self {
        unsafe {
            Self::from_sys_init(|self_ptr| {
                let ctor = sys::get_cache().string_name_from_string;
                let args = [s.sys()];
                ctor(self_ptr, args.as_ptr());
            })
        }
    }
}
impl From<&StringName> for GodotString {
    fn from(s: &StringName) -> Self {
        unsafe {
            Self::from_sys_init(|self_ptr| {
                let ctor = sys::get_cache().string_from_string_name;
                let args = [s.sys()];
                ctor(self_ptr, args.as_ptr());
            })
        }
    }
}
