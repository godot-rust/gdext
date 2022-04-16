use crate as sys;
use sys::GodotFfi;

pub trait PtrCallArg {
    /// Read an argument value from a ptrcall argument.
    ///
    /// # Safety
    ///
    /// Implementations of this function will use pointer casting and must make
    /// sure that the proper types are expected as they are provided by Godot.
    unsafe fn ptrcall_read(arg: sys::GDNativeTypePtr) -> Self;

    /// Write a value to a ptrcall argument or return value.
    ///
    /// # Safety
    ///
    /// Implementations of this function will use pointer casting and must make
    /// sure that the proper types are provided as they are expected by Godot.
    unsafe fn ptrcall_write(self, ret: sys::GDNativeTypePtr);
}

// Blanket implementation for all `GodotFfi` classes
impl<T: GodotFfi> PtrCallArg for T {
    unsafe fn ptrcall_read(arg: sys::GDNativeTypePtr) -> Self {
        Self::from_sys(arg)
    }

    unsafe fn ptrcall_write(self, ret: sys::GDNativeTypePtr) {
        self.write_sys(ret);
        std::mem::forget(self); // TODO double-check
    }
}

macro_rules! impl_ptr_call_arg_num {
    ($t:ty) => {
        impl PtrCallArg for $t {
            unsafe fn ptrcall_read(arg: sys::GDNativeTypePtr) -> Self {
                *(arg as *mut $t)
            }

            unsafe fn ptrcall_write(self, ret: sys::GDNativeTypePtr) {
                *(ret as *mut $t) = self;
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
    unsafe fn ptrcall_read(_arg: sys::GDNativeTypePtr) -> Self {}

    unsafe fn ptrcall_write(self, _arg: sys::GDNativeTypePtr) {
        // do nothing
    }
}
