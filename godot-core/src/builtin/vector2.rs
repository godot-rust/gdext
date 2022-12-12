/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use godot_ffi as sys;
use sys::{ffi_methods, GodotFfi};

type Inner = glam::f32::Vec2;
//type Inner = glam::f64::DVec2;

#[derive(Default, Copy, Clone, Debug, PartialEq)]
#[repr(C)]
pub struct Vector2 {
    inner: Inner,
}

impl Vector2 {
    pub fn new(x: f32, y: f32) -> Self {
        Self {
            inner: Inner::new(x, y),
        }
    }

    pub fn from_inner(inner: Inner) -> Self {
        Self { inner }
    }

    /// only for testing
    pub fn inner(self) -> Inner {
        self.inner
    }

    // Hacks for example
    // pub fn length(self) -> f32 {
    //     self.inner.length()
    // }
    // pub fn normalized(self) -> Vector2 {
    //     Self::from_inner(self.inner.normalize())
    // }
    pub fn rotated(self, angle: f32) -> Self {
        Self::from_inner(glam::Affine2::from_angle(angle).transform_vector2(self.inner))
    }
}

impl GodotFfi for Vector2 {
    ffi_methods! { type sys::GDExtensionTypePtr = *mut Self; .. }
}

impl std::fmt::Display for Vector2 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.inner.fmt(f)
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

type IInner = glam::IVec2;

#[derive(Default, Copy, Clone, Debug, Eq, PartialEq)]
#[repr(C)]
pub struct Vector2i {
    inner: IInner,
}

impl Vector2i {
    pub fn new(x: i32, y: i32) -> Self {
        Self {
            inner: IInner::new(x, y),
        }
    }
}

impl GodotFfi for Vector2i {
    ffi_methods! { type sys::GDExtensionTypePtr = *mut Self; .. }
}

impl std::fmt::Display for Vector2i {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.inner.fmt(f)
    }
}
