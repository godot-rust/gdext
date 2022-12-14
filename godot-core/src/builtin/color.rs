/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use godot_ffi as sys;
use sys::{ffi_methods, GodotFfi};

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Color {
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
    ffi_methods! { type sys::GDExtensionTypePtr = *mut Self; .. }
}

impl Default for Color {
    fn default() -> Self {
        Self::new(0.0, 0.0, 0.0, 1.0)
    }
}
