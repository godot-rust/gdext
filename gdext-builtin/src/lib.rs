pub mod string;
pub mod variant;
pub mod vector2;
pub mod vector3;

pub use glam;

pub trait PtrCallArg {
    /// Read an argument value from a ptrcall argument.
    ///
    /// # Safety
    ///
    /// Implementations of this function will use pointer casting and must make
    /// sure that the proper types are expected as they are provided by Godot.
    unsafe fn from_ptr_call_arg(arg: *const gdext_sys::GDNativeTypePtr) -> Self;

    /// Write a value to a ptrcall argument or return value.
    ///
    /// # Safety
    ///
    /// Implementations of this function will use pointer casting and must make
    /// sure that the proper types are provided as they are expected by Godot.
    unsafe fn to_ptr_call_arg(self, arg: gdext_sys::GDNativeTypePtr);
}

#[macro_export]
macro_rules! entry {
    ($entry_point_name:ident) => {
        #[no_mangle]
        unsafe extern "C" fn $entry_point_name(
            interface: *const gdext_sys::GDNativeInterface,
            library: gdext_sys::GDNativeExtensionClassLibraryPtr,
            init: *mut gdext_sys::GDnativeInitialization,
        ) {
            gdext_sys::set_interface(interface);
        }
    };
}

macro_rules! impl_ptr_call_arg_num {
    ($t:ty) => {
        impl PtrCallArg for $t {
            unsafe fn from_ptr_call_arg(arg: *const gdext_sys::GDNativeTypePtr) -> Self {
                *(*arg as *mut $t)
            }

            unsafe fn to_ptr_call_arg(self, arg: gdext_sys::GDNativeTypePtr) {
                *(arg as *mut $t) = self;
            }
        }
    };
}

impl_ptr_call_arg_num!(u8);
impl_ptr_call_arg_num!(u16);
impl_ptr_call_arg_num!(u32);
impl_ptr_call_arg_num!(u64);

impl_ptr_call_arg_num!(i8);
impl_ptr_call_arg_num!(i16);
impl_ptr_call_arg_num!(i32);
impl_ptr_call_arg_num!(i64);

impl_ptr_call_arg_num!(f32);
impl_ptr_call_arg_num!(f64);

impl PtrCallArg for () {
    unsafe fn from_ptr_call_arg(_arg: *const gdext_sys::GDNativeTypePtr) -> Self {}

    unsafe fn to_ptr_call_arg(self, _arg: gdext_sys::GDNativeTypePtr) {
        // do nothing
    }
}
