use crate::PtrCallArg;

#[cfg(not(feature = "real_is_double"))]
pub type Vector3 = glam::f32::Vec3;
#[cfg(feature = "real_is_double")]
pub type Vector3 = glam::f64::DVec3;

impl PtrCallArg for Vector3 {
    unsafe fn from_ptr_call_arg(arg: *const gdext_sys::GDNativeTypePtr) -> Self {
        *(*arg as *const Vector3)
    }

    unsafe fn to_ptr_call_arg(self, arg: gdext_sys::GDNativeTypePtr) {
        *(arg as *mut Vector3) = self;
    }
}
