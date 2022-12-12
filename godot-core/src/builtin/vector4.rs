/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use godot_ffi as sys;
use sys::{ffi_methods, GodotFfi};

type Inner = glam::f32::Vec4;
//type Inner = glam::f64::DVec4;

#[derive(Default, Copy, Clone, Debug, PartialEq)]
#[repr(C)]
pub struct Vector4 {
    inner: Inner,
}

impl Vector4 {
    pub fn new(x: f32, y: f32, z: f32, w: f32) -> Self {
        Self {
            inner: Inner::new(x, y, z, w),
        }
    }
}

impl GodotFfi for Vector4 {
    ffi_methods! { type sys::GDExtensionTypePtr = *mut Self; .. }
}

impl std::fmt::Display for Vector4 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.inner.fmt(f)
    }
}

type IInner = glam::IVec4;

#[derive(Default, Copy, Clone, Debug)]
#[repr(C)]
pub struct Vector4i {
    inner: IInner,
}

impl Vector4i {
    pub fn new(x: i32, y: i32, z: i32, w: i32) -> Self {
        Self {
            inner: IInner::new(x, y, z, w),
        }
    }
}

impl GodotFfi for Vector4i {
    ffi_methods! { type sys::GDExtensionTypePtr = *mut Self; .. }
}

impl std::fmt::Display for Vector4i {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.inner.fmt(f)
    }
}
