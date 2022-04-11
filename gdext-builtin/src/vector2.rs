use crate::godot_ffi::GodotFfi;
use crate::impl_ffi_as_value;

#[cfg(not(feature = "real_is_double"))]
pub type Vector2 = glam::f32::Vec2;
#[cfg(feature = "real_is_double")]
pub type Vector2 = glam::f64::DVec2;

impl GodotFfi for Vector2 {
    impl_ffi_as_value!();
}
