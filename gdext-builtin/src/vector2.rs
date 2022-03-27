use crate::godot_ffi::GodotFfi;

#[cfg(not(feature = "real_is_double"))]
pub type Vector2 = glam::f32::Vec2;
#[cfg(feature = "real_is_double")]
pub type Vector2 = glam::f64::DVec2;

impl GodotFfi for Vector2 {
    unsafe fn from_sys(opaque_ptr: *mut std::ffi::c_void) -> Self {
        *(opaque_ptr as *mut Vector2)
    }

    unsafe fn from_sys_init(init: impl FnOnce(*mut std::ffi::c_void)) -> Self {
        let mut raw = std::mem::MaybeUninit::<Self>::uninit();
        init(raw.as_mut_ptr() as *mut std::ffi::c_void);

        raw.assume_init()
    }

    fn sys(&self) -> *mut std::ffi::c_void {
        self as *const Vector2 as *mut std::ffi::c_void
    }

    unsafe fn write_sys(&self, dst: *mut std::ffi::c_void) {
        *(dst as *mut Vector2) = *self;
    }
}