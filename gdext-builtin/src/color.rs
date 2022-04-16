use crate::sys::{impl_ffi_as_value, GodotFfi};

#[repr(C)]
#[derive(Copy, Clone)]
struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Color {
    #[allow(dead_code)]
    pub fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }
}

impl GodotFfi for Color {
    impl_ffi_as_value!();
}
