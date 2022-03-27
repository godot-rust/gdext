use crate::PtrCallArg;

#[cfg(not(feature = "real_is_double"))]
pub type Vector3 = glam::f32::Vec3;
#[cfg(feature = "real_is_double")]
pub type Vector3 = glam::f64::DVec3;

impl PtrCallArg for Vector3 {
    unsafe fn ptrcall_read(arg: gdext_sys::GDNativeTypePtr) -> Self {
        *(arg as *const Vector3)
    }

    unsafe fn ptrcall_write(self, ret: gdext_sys::GDNativeTypePtr) {
        *(ret as *mut Vector3) = self;
    }
}
