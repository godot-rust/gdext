// Stub for various other built-in classes, which are currently incomplete, but whose types
// are required for codegen
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
            impl_ffi_as_opaque_pointer!(sys::GDNativeTypePtr);
        }
    };
}

impl_builtin_stub!(Array, OpaqueArray);
impl_builtin_stub!(Dictionary, OpaqueDictionary);
impl_builtin_stub!(StringName, OpaqueStringName);
impl_builtin_stub!(Transform2D, OpaqueTransform2D);
impl_builtin_stub!(Transform3D, OpaqueTransform3D);
impl_builtin_stub!(NodePath, OpaqueNodePath);
