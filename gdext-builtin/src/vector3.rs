use gdext_sys as sys;
use sys::{impl_ffi_as_value, real, GodotFfi};

#[cfg(not(feature = "real_is_double"))]
type Inner = glam::f32::Vec3;
#[cfg(feature = "real_is_double")]
type Inner = glam::f64::DVec3;

#[derive(Default, Copy, Clone, Debug)]
#[repr(C)]
pub struct Vector3 {
    inner: Inner,
}

impl Vector3 {
    pub fn new(x: real, y: real, z: real) -> Self {
        Self {
            inner: Inner::new(x, y, z),
        }
    }
}

impl GodotFfi for Vector3 {
    impl_ffi_as_value!();
}
