/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::builtin::real::Real;

impl_vector!(Vector4, crate::builtin::real::Vec4, Real, (x, y, z, w));
impl_float_vector!(Vector4, Real);
impl_vector_from!(Vector4, Vector4i, Real, (x, y, z, w));

impl_vector!(Vector4i, glam::IVec4, i32, (x, y, z, w));
impl_vector_from!(Vector4i, Vector4, i32, (x, y, z, w));
