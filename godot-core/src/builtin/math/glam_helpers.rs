/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

// TODO this is experimental -- do not refactor existing types to this yet
// Need to see if ergonomics are worth the generic complexity.
//
// Nice:
//   self.glam2(&with, |a, b| a.dot(b))
//   self.glam2(&with, glam::f32::Quat::dot)
//
// Alternative with only conversions:
//   self.glam().dot(b.glam())
//   GlamType::dot(self.glam(), b.glam())

use crate::builtin::real;

pub(crate) trait GlamConv {
    type Glam: GlamType<Mapped = Self>;

    #[inline]
    fn to_glam(&self) -> Self::Glam {
        Self::Glam::from_front(self)
    }

    #[inline]
    fn glam<F, R>(&self, unary_fn: F) -> R::Mapped
    where
        R: GlamType,
        F: FnOnce(Self::Glam) -> R,
    {
        let arg = Self::Glam::from_front(self);
        let result = unary_fn(arg);

        result.to_front()
    }

    #[inline]
    fn glam2<F, P, R>(&self, rhs: &P, binary_fn: F) -> R::Mapped
    where
        P: GlamConv,
        R: GlamType,
        F: FnOnce(Self::Glam, P::Glam) -> R,
    {
        let arg0 = Self::Glam::from_front(self);
        let arg1 = P::Glam::from_front(rhs);

        let result = binary_fn(arg0, arg1);
        result.to_front()
    }
}

pub(crate) trait GlamType {
    type Mapped;

    fn to_front(&self) -> Self::Mapped;
    fn from_front(mapped: &Self::Mapped) -> Self;
}

macro_rules! impl_glam_map_self {
    ($T:ty) => {
        impl GlamType for $T {
            type Mapped = $T;

            fn to_front(&self) -> $T {
                *self
            }

            fn from_front(mapped: &$T) -> Self {
                *mapped
            }
        }
    };
}

impl_glam_map_self!(real);
impl_glam_map_self!(bool);
impl_glam_map_self!((real, real, real));
