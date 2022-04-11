use crate::godot_ffi::GodotFfi;
use crate::impl_ffi_as_value;

#[cfg(not(feature = "real_is_double"))]
pub type Vector3 = glam::f32::Vec3;
#[cfg(feature = "real_is_double")]
pub type Vector3 = glam::f64::DVec3;

impl GodotFfi for Vector3 {
    impl_ffi_as_value!();
}
