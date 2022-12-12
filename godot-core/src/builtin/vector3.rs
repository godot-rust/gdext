/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use godot_ffi as sys;
use sys::{ffi_methods, GodotFfi};

type Inner = glam::f32::Vec3;
// type Inner = glam::f64::DVec3;

#[derive(Default, Copy, Clone, Debug, PartialEq)]
#[repr(C)]
pub struct Vector3 {
    inner: Inner,
}

impl Vector3 {
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Self {
            inner: Inner::new(x, y, z),
        }
    }
}

impl GodotFfi for Vector3 {
    ffi_methods! { type sys::GDExtensionTypePtr = *mut Self; .. }
}

impl std::fmt::Display for Vector3 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        //let Inner {x, y, z} = self.inner;
        //write!(f, "({x}, {y}, {z})")
        self.inner.fmt(f)
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

type IInner = glam::IVec3;

#[derive(Default, Copy, Clone, Debug, Eq, PartialEq)]
#[repr(C)]
pub struct Vector3i {
    inner: IInner,
}

impl Vector3i {
    pub fn new(x: i32, y: i32, z: i32) -> Self {
        Self {
            inner: IInner::new(x, y, z),
        }
    }
}

impl GodotFfi for Vector3i {
    ffi_methods! { type sys::GDExtensionTypePtr = *mut Self; .. }
}

impl std::fmt::Display for Vector3i {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.inner.fmt(f)
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

// TODO auto-generate this, alongside all the other builtin type's enums

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(i32)]
pub enum Vector3Axis {
    X,
    Y,
    Z,
}

impl GodotFfi for Vector3Axis {
    ffi_methods! { type sys::GDExtensionTypePtr = *mut Self; .. }
}
