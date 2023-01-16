/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

pub use single::*;

mod single {
    /// Floating-point type used throughout the engine. This is the equivalent of `real_t` in the
    /// engine's C++ code.
    ///
    /// Currently, this is always `f32`; working with an engine compiled with `precision=double` is
    /// not supported yet.
    pub type Real = f32;

    pub(crate) type Vec2 = glam::f32::Vec2;
    pub(crate) type Vec3 = glam::f32::Vec3;
    pub(crate) type Vec4 = glam::f32::Vec4;
}

// mod double {
//     pub type Real = f64;
//     pub(crate) type Vec2 = glam::f64::DVec2;
//     pub(crate) type Vec3 = glam::f64::DVec3;
//     pub(crate) type Vec4 = glam::f64::DVec4;
// }
