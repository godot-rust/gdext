use crate::PtrCallArg;

#[cfg(not(feature = "real_is_double"))]
pub type Vector2 = glam::f32::Vec2;
#[cfg(feature = "real_is_double")]
pub type Vector2 = glam::f64::DVec2;

impl PtrCallArg for Vector2 {
    unsafe fn ptrcall_read(arg: gdext_sys::GDNativeTypePtr) -> Self {
        *(arg as *const Vector2)
    }

    unsafe fn ptrcall_write(self, ret: gdext_sys::GDNativeTypePtr) {
        *(ret as *mut Vector2) = self;
    }
}
