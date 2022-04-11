use crate::{GodotFfi};

#[cfg(not(feature = "real_is_double"))]
pub type Vector3 = glam::f32::Vec3;
#[cfg(feature = "real_is_double")]
pub type Vector3 = glam::f64::DVec3;

/*impl PtrCallArg for Vector3 {
    unsafe fn ptrcall_read(arg: gdext_sys::GDNativeTypePtr) -> Self {
        *(arg as *const Vector3)
    }

    unsafe fn ptrcall_write(self, ret: gdext_sys::GDNativeTypePtr) {
        *(ret as *mut Vector3) = self;
    }
}*/

impl GodotFfi for Vector3 {
    unsafe fn from_sys(opaque_ptr: *mut std::ffi::c_void) -> Self {
        *(opaque_ptr as *mut Vector3)
    }

    unsafe fn from_sys_init(init: impl FnOnce(*mut std::ffi::c_void)) -> Self {
        let mut raw = std::mem::MaybeUninit::<Self>::uninit();
        init(raw.as_mut_ptr() as *mut std::ffi::c_void);

        raw.assume_init()
    }

    fn sys(&self) -> *mut std::ffi::c_void {
        self as *const Vector3 as *mut std::ffi::c_void
    }

    unsafe fn write_sys(&self, dst: *mut std::ffi::c_void) {
        *(dst as *mut Vector3) = *self;
    }
}
