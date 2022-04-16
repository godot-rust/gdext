use std::mem::MaybeUninit;
use crate as sys;
use sys::GodotFfi;

/// Implemented for types which can be passed as arguments and return values from Godot's `ptrcall` FFI.
pub trait PtrCall where Self:Sized {
    /// Read an argument value from a ptrcall argument.
    ///
    /// # Safety
    ///
    /// Implementations of this function will use pointer casting and must make
    /// sure that the proper types are expected as they are provided by Godot.
    unsafe fn ptrcall_read(arg: sys::GDNativeTypePtr) -> Self;

    unsafe fn ptrcall_read_init(init: impl FnOnce(sys::GDNativeTypePtr)) -> Self {
        let mut arg = MaybeUninit::uninit();
        init(arg.as_mut_ptr() as *mut _);

        //let arg = arg.assume_init();
        //Self::ptrcall_read(arg)
        arg.assume_init()
    }

    /// Write a value to a ptrcall argument or return value.
    ///
    /// # Safety
    ///
    /// Implementations of this function will use pointer casting and must make
    /// sure that the proper types are provided as they are expected by Godot.
    unsafe fn ptrcall_write(self, ret: sys::GDNativeTypePtr);
}

// Blanket implementation for all `GodotFfi` classes
impl<T: GodotFfi> PtrCall for T {
    unsafe fn ptrcall_read(arg: sys::GDNativeTypePtr) -> Self {
        Self::from_sys(arg)
    }

    unsafe fn ptrcall_read_init(init: impl FnOnce(sys::GDNativeTypePtr)) -> Self {
        Self::from_sys_init(init)
    }

    unsafe fn ptrcall_write(self, ret: sys::GDNativeTypePtr) {
        self.write_sys(ret);
        std::mem::forget(self); // TODO double-check
    }
}

macro_rules! impl_ptrcall_num {
    ($t:ty) => {
        impl PtrCall for $t {
            unsafe fn ptrcall_read(arg: sys::GDNativeTypePtr) -> Self {
                *(arg as *mut $t)
            }

            unsafe fn ptrcall_write(self, ret: sys::GDNativeTypePtr) {
                *(ret as *mut $t) = self;
            }
        }
    };
}

impl_ptrcall_num!(u8);
impl_ptrcall_num!(u16);
impl_ptrcall_num!(u32);
impl_ptrcall_num!(u64);

impl_ptrcall_num!(i8);
impl_ptrcall_num!(i16);
impl_ptrcall_num!(i32);
impl_ptrcall_num!(i64);

impl_ptrcall_num!(f32);
impl_ptrcall_num!(f64);

impl PtrCall for () {
    unsafe fn ptrcall_read(_arg: sys::GDNativeTypePtr) -> Self {}

    unsafe fn ptrcall_write(self, _arg: sys::GDNativeTypePtr) {
        // do nothing
    }
}
