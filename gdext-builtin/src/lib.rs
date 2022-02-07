pub mod string;
pub mod variant;
pub mod vector2;
pub mod vector3;

pub use glam;

pub trait FromPtrCallArg {
    unsafe fn from_ptr_call_arg(arg: gdext_sys::GDNativeTypePtr) -> Self;
}

#[macro_export]
macro_rules! entry {
    ($entry_point_name:ident) => {
        #[no_mangle]
        unsafe extern "C" fn $entry_point_name(
            interface: *const sys::GDNativeInterface,
            library: sys::GDNativeExtensionClassLibraryPtr,
            init: *mut sys::GDnativeInitialization,
        ) {
            $crate::sys::set_interface(interface);
        }
    };
}

impl FromPtrCallArg for f32 {
    unsafe fn from_ptr_call_arg(arg: gdext_sys::GDNativeTypePtr) -> Self {
        f64::from_ptr_call_arg(arg) as f32
    }
}

impl FromPtrCallArg for f64 {
    unsafe fn from_ptr_call_arg(arg: gdext_sys::GDNativeTypePtr) -> Self {
        *(arg as *const f64)
    }
}

impl FromPtrCallArg for i64 {
    unsafe fn from_ptr_call_arg(arg: gdext_sys::GDNativeTypePtr) -> Self {
        *(arg as *const i64)
    }
}
