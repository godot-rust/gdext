#[cfg(not(feature = "real_is_double"))]
pub type Vector3 = glam::f32::Vec3;
#[cfg(feature = "real_is_double")]
pub type Vector3 = glam::f64::DVec3;
