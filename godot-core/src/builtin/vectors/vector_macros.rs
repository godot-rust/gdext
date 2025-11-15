/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

#![macro_use]

/// Implements a single unary operator for a vector type. Only used for `Neg` at the moment.
macro_rules! impl_vector_unary_operator {
    (
        // Name of the vector type.
        $Vector:ty,
        // Names of the components, with parentheses, for example `(x, y)`.
        ($($components:ident),*),
        // Name of the operator trait, for example `Neg`.
        $Operator:ident,
        // Name of the function on the operator trait, for example `neg`.
        $func:ident
    ) => {
        impl std::ops::$Operator for $Vector {
            type Output = Self;
            fn $func(mut self) -> Self::Output {
                $(
                    self.$components = self.$components.$func();
                )*
                self
            }
        }
    }
}

/// Implements a component-wise single infix binary operator between two vectors.
macro_rules! impl_vector_vector_binary_operator {
    (
        // Name of the vector type.
        $Vector:ty,
        // Names of the components, with parentheses, for example `(x, y)`.
        ($($components:ident),*),
        // Name of the operator trait, for example `Add`.
        $Operator:ident,
        // Name of the function on the operator trait, for example `add`.
        $func:ident
    ) => {
        impl std::ops::$Operator for $Vector {
            type Output = Self;
            fn $func(mut self, rhs: $Vector) -> Self::Output {
                $(
                    self.$components = self.$components.$func(rhs.$components);
                )*
                self
            }
        }
    }
}

/// Implements a component-wise single infix binary operator between a vector on the left and a
/// scalar on the right-hand side.
macro_rules! impl_vector_scalar_binary_operator {
    (
        // Name of the vector type.
        $Vector:ty,
        // Type of each individual component, for example `i32`.
        $Scalar:ty,
        // Names of the components, with parentheses, for example `(x, y)`.
        ($($components:ident),*),
        // Name of the operator trait, for example `Add`.
        $Operator:ident,
        // Name of the function on the operator trait, for example `add`.
        $func:ident
    ) => {
        impl std::ops::$Operator<$Scalar> for $Vector {
            type Output = Self;
            fn $func(mut self, rhs: $Scalar) -> Self::Output {
                $(
                    self.$components = self.$components.$func(rhs);
                )*
                self
            }
        }
    }
}

/// Implements a component-wise single infix binary operator between a scalar on the left and a
/// vector on the right-hand side.
macro_rules! impl_scalar_vector_binary_operator {
    (
        // Name of the vector type.
        $Vector:ty,
        // Type of each individual component, for example `i32`.
        $Scalar:ty,
        // Names of the components, with parentheses, for example `(x, y)`.
        ($($components:ident),*),
        // Name of the operator trait, for example `Add`.
        $Operator:ident,
        // Name of the function on the operator trait, for example `add`.
        $func:ident
    ) => {
        impl std::ops::$Operator<$Vector> for $Scalar {
            type Output = $Vector;
            fn $func(self, mut rhs: $Vector) -> Self::Output {
                $(
                    rhs.$components = rhs.$components.$func(self);
                )*
                rhs
            }
        }
    }
}

/// Implements a single arithmetic assignment operator for a vector type, with a vector on the
/// right-hand side.
macro_rules! impl_vector_vector_assign_operator {
    (
        // Name of the vector type.
        $Vector:ty,
        // Names of the components, with parentheses, for example `(x, y)`.
        ($($components:ident),*),
        // Name of the operator trait, for example `AddAssign`.
        $Operator:ident,
        // Name of the function on the operator trait, for example `add_assign`.
        $func:ident
    ) => {
        impl std::ops::$Operator for $Vector {
            fn $func(&mut self, rhs: $Vector) {
                $(
                    self.$components.$func(rhs.$components);
                )*
            }
        }
    }
}

/// Implements a single arithmetic assignment operator for a vector type, with a scalar on the
/// right-hand side.
macro_rules! impl_vector_scalar_assign_operator {
    (
        // Name of the vector type.
        $Vector:ty,
        // Type of each individual component, for example `i32`.
        $Scalar:ty,
        // Names of the components, with parentheses, for example `(x, y)`.
        ($($components:ident),*),
        // Name of the operator trait, for example `AddAssign`.
        $Operator:ident,
        // Name of the function on the operator trait, for example `add_assign`.
        $func:ident
    ) => {
        impl std::ops::$Operator<$Scalar> for $Vector {
            fn $func(&mut self, rhs: $Scalar) {
                $(
                    self.$components.$func(rhs);
                )*
            }
        }
    }
}

/// Implements a reduction (sum or product) over an iterator of vectors.
macro_rules! impl_iter_vector_reduction {
    (
        // Name of the vector type.
        $Vector:ty,
        // Name of the reduction trait: `Sum` or `Product`.
        $Operator:ident,
        // Name of the function on the operator trait, for example `add`.
        $func:ident
    ) => {
        impl std::iter::$Operator<Self> for $Vector {
            #[doc = concat!("Element-wise ", stringify!($func), " of all vectors in the iterator.")]
            fn $func<I>(iter: I) -> Self
            where
                I: Iterator<Item = Self>,
            {
                Self::from_glam(iter.map(Self::to_glam).$func())
            }
        }

        impl<'a> std::iter::$Operator<&'a Self> for $Vector {
            #[doc = concat!("Element-wise ", stringify!($func), " of all vectors in the iterator.")]
            fn $func<I>(iter: I) -> Self
            where
                I: Iterator<Item = &'a Self>,
            {
                Self::from_glam(iter.map(|x| Self::to_glam(*x)).$func())
            }
        }
    };
}

/// Implements all common arithmetic operators on a built-in vector type.
macro_rules! impl_vector_operators {
    (
        // Name of the vector type to be implemented, for example `Vector2`.
        $Vector:ty,
        // Type of each individual component, for example `real`.
        $Scalar:ty,
        // Names of the components, with parentheses, for example `(x, y)`.
        ($($components:ident),*)
    ) => {
        impl_vector_unary_operator!($Vector, ($($components),*), Neg, neg);
        impl_vector_vector_binary_operator!($Vector, ($($components),*), Add, add);
        impl_vector_vector_binary_operator!($Vector, ($($components),*), Sub, sub);
        impl_vector_vector_binary_operator!($Vector, ($($components),*), Mul, mul);
        impl_vector_scalar_binary_operator!($Vector, $Scalar, ($($components),*), Mul, mul);
        impl_scalar_vector_binary_operator!($Vector, $Scalar, ($($components),*), Mul, mul);
        impl_vector_vector_binary_operator!($Vector, ($($components),*), Div, div);
        impl_vector_scalar_binary_operator!($Vector, $Scalar, ($($components),*), Div, div);
        impl_iter_vector_reduction!($Vector, Sum, sum);
        impl_iter_vector_reduction!($Vector, Product, product);
        impl_vector_vector_assign_operator!($Vector, ($($components),*), AddAssign, add_assign);
        impl_vector_vector_assign_operator!($Vector, ($($components),*), SubAssign, sub_assign);
        impl_vector_vector_assign_operator!($Vector, ($($components),*), MulAssign, mul_assign);
        impl_vector_scalar_assign_operator!($Vector, $Scalar, ($($components),*), MulAssign, mul_assign);
        impl_vector_vector_assign_operator!($Vector, ($($components),*), DivAssign, div_assign);
        impl_vector_scalar_assign_operator!($Vector, $Scalar, ($($components),*), DivAssign, div_assign);
    }
}

/// Implements `Index` and `IndexMut` for a vector type, using an enum to indicate the desired axis.
macro_rules! impl_vector_index {
    (
        // Name of the vector type to be implemented, for example `Vector2`.
        $Vector:ty,
        // Type of each individual component, for example `real`.
        $Scalar:ty,
        // Names of the components, with parentheses, for example `(x, y)`.
        ($( $components:ident ),*),
        // Name of the enum type for the axes, for example `Vector2Axis`.
        $AxisEnum:ty,
        // Names of the enum variants, with parenthes, for example `(X, Y)`.
        ($( $axis_variants:ident ),*)
    ) => {
        impl std::ops::Index<$AxisEnum> for $Vector {
            type Output = $Scalar;
            fn index(&self, axis: $AxisEnum) -> &$Scalar {
                match axis {
                    $(<$AxisEnum>::$axis_variants => &self.$components),*
                }
            }
        }

        impl std::ops::IndexMut<$AxisEnum> for $Vector {
            fn index_mut(&mut self, axis: $AxisEnum) -> &mut $Scalar {
                match axis {
                    $(<$AxisEnum>::$axis_variants => &mut self.$components),*
                }
            }
        }
    }
}

/// Implements constants that are present on floating-point and integer vectors.
macro_rules! impl_vector_consts {
    (
        // Type of target component, for example `real`.
        $Scalar:ty
    ) => {
        /// Zero vector, a vector with all components set to `0`.
        pub const ZERO: Self = Self::splat(0 as $Scalar);

        /// One vector, a vector with all components set to `1`.
        pub const ONE: Self = Self::splat(1 as $Scalar);
    };
}

/// Implements constants that are present only on floating-point vectors.
macro_rules! impl_float_vector_consts {
    () => {
        /// Infinity vector, a vector with all components set to `real::INFINITY`.
        pub const INF: Self = Self::splat(real::INFINITY);
    };
}

/// Implements constants that are present only on integer vectors.
macro_rules! impl_integer_vector_consts {
    () => {
        /// Min vector, a vector with all components equal to [`i32::MIN`]. Can be used as a negative integer equivalent of `real::INF`.
        pub const MIN: Self = Self::splat(i32::MIN);

        /// Max vector, a vector with all components equal to [`i32::MAX`]. Can be used as an integer equivalent of `real::INF`.
        pub const MAX: Self = Self::splat(i32::MAX);
    };
}

/// Implements constants present on 2D vectors.
macro_rules! impl_vector2x_consts {
    (
        // Type of target component, for example `real`.
        $Scalar:ty
    ) => {
        /// Left unit vector. Represents the direction of left.
        pub const LEFT: Self = Self::new(-1 as $Scalar, 0 as $Scalar);

        /// Right unit vector. Represents the direction of right.
        pub const RIGHT: Self = Self::new(1 as $Scalar, 0 as $Scalar);

        /// Up unit vector. Y is down in 2D, so this vector points -Y.
        pub const UP: Self = Self::new(0 as $Scalar, -1 as $Scalar);

        /// Down unit vector. Y is down in 2D, so this vector points +Y.
        pub const DOWN: Self = Self::new(0 as $Scalar, 1 as $Scalar);
    };
}

/// Implements constants present on 3D vectors.
macro_rules! impl_vector3x_consts {
    (
        // Type of target component, for example `real`.
        $Scalar:ty
    ) => {
        /// Unit vector in -X direction. Can be interpreted as left in an untransformed 3D world.
        pub const LEFT: Self = Self::new(-1 as $Scalar, 0 as $Scalar, 0 as $Scalar);

        /// Unit vector in +X direction. Can be interpreted as right in an untransformed 3D world.
        pub const RIGHT: Self = Self::new(1 as $Scalar, 0 as $Scalar, 0 as $Scalar);

        /// Unit vector in +Y direction. Typically interpreted as up in a 3D world.
        pub const UP: Self = Self::new(0 as $Scalar, 1 as $Scalar, 0 as $Scalar);

        /// Unit vector in -Y direction. Typically interpreted as down in a 3D world.
        pub const DOWN: Self = Self::new(0 as $Scalar, -1 as $Scalar, 0 as $Scalar);

        /// Unit vector in -Z direction. Can be interpreted as “into the screen” in an untransformed 3D world.
        pub const FORWARD: Self = Self::new(0 as $Scalar, 0 as $Scalar, -1 as $Scalar);

        /// Unit vector in +Z direction. Can be interpreted as “out of the screen” in an untransformed 3D world.
        pub const BACK: Self = Self::new(0 as $Scalar, 0 as $Scalar, 1 as $Scalar);
    };
}

macro_rules! shared_vector_docs {
    () => {
        "Conversions are provided via various `from_*` and `to_*` functions, not via the `From` trait. This encourages `new()` as the main \
         way to construct vectors, is explicit about the conversion taking place, needs no type inference, and works in `const` contexts."
    };
}

macro_rules! tuple_type {
    ($Scalar:ty; $x:ident, $y:ident) => {
        ($Scalar, $Scalar)
    };
    ($Scalar:ty; $x:ident, $y:ident, $z:ident) => {
        ($Scalar, $Scalar, $Scalar)
    };
    ($Scalar:ty; $x:ident, $y:ident, $z:ident, $w:ident) => {
        ($Scalar, $Scalar, $Scalar, $Scalar)
    };
}

macro_rules! array_type {
    ($Scalar:ty; $x:ident, $y:ident) => {
        [$Scalar; 2]
    };
    ($Scalar:ty; $x:ident, $y:ident, $z:ident) => {
        [$Scalar; 3]
    };
    ($Scalar:ty; $x:ident, $y:ident, $z:ident, $w:ident) => {
        [$Scalar; 4]
    };
}

/// Implements functions that are present on floating-point and integer vectors.
macro_rules! impl_vector_fns {
    (
        // Name of the vector type.
        $Vector:ty,
        // Name of the glam vector type.
        $GlamVector:ty,
        // Type of target component, for example `real`.
        $Scalar:ty,
        // Names of the components, with parentheses, for example `(x, y)`.
        ($($comp:ident),*)
    ) => {
        /// # Constructors and general vector functions
        /// The following associated functions and methods are available on all vectors (2D, 3D, 4D; float and int).
        impl $Vector {
            /// Creates a vector with the given components.
            #[inline]
            pub const fn new($($comp: $Scalar),*) -> Self {
                Self {
                    $( $comp ),*
                }
            }

            /// Creates a vector with all components set to `v`.
            #[inline]
            pub const fn splat(v: $Scalar) -> Self {
                Self {
                    $( $comp: v ),*
                }
            }

            /// Creates a vector from the given tuple.
            #[inline]
            pub const fn from_tuple(tuple: tuple_type!($Scalar; $($comp),*)) -> Self {
                let ( $($comp,)* ) = tuple;
                Self::new( $($comp),* )
            }

            /// Creates a vector from the given array.
            #[inline]
            pub const fn from_array(array: array_type!($Scalar; $($comp),*)) -> Self {
                let [ $($comp,)* ] = array;
                Self::new( $($comp),* )
            }

            /// Returns a tuple with the components of the vector.
            #[inline]
            pub const fn to_tuple(&self) -> tuple_type!($Scalar; $($comp),*) {
                ( $(self.$comp,)* )
            }

            /// Returns an array with the components of the vector.
            #[inline]
            pub const fn to_array(&self) -> array_type!($Scalar; $($comp),*) {
                [ $(self.$comp,)* ]
            }

            /// Converts the corresponding `glam` type to `Self`.
            pub(crate) fn from_glam(v: $GlamVector) -> Self {
                Self::new(
                    $( v.$comp ),*
                )
            }

            /// Converts `self` to the corresponding `glam` type.
            pub(crate) fn to_glam(self) -> $GlamVector {
                <$GlamVector>::new(
                    $( self.$comp ),*
                )
            }

            /// Returns a new vector with all components in absolute values (i.e. positive or
            /// zero).
            #[inline]
            pub fn abs(self) -> Self {
                Self::from_glam(self.to_glam().abs())
            }

            /// Returns a new vector with all components clamped between the components of `min` and `max`.
            ///
            /// # Panics
            /// If `min` > `max`, `min` is NaN, or `max` is NaN.
            #[inline]
            pub fn clamp(self, min: Self, max: Self) -> Self {
                Self::from_glam(self.to_glam().clamp(min.to_glam(), max.to_glam()))
            }

            /// Returns the length (magnitude) of this vector.
            #[inline]
            pub fn length(self) -> real {
                // does the same as glam's length() but also works for integer vectors
                (self.length_squared() as real).sqrt()
            }

            /// Squared length (squared magnitude) of this vector.
            ///
            /// Runs faster than [`length()`][Self::length], so prefer it if you need to compare vectors or need the
            /// squared distance for some formula.
            #[inline]
            pub fn length_squared(self) -> $Scalar {
                self.to_glam().length_squared()
            }

            /// Returns a new vector containing the minimum of the two vectors, component-wise.
            ///
            #[doc = concat!("You may consider using the fully-qualified syntax `", stringify!($Vector), "::coord_min(a, b)` for symmetry.")]
            #[inline]
            pub fn coord_min(self, other: Self) -> Self {
                self.glam2(&other, |a, b| a.min(b))
            }

            /// Returns a new vector containing the maximum of the two vectors, component-wise.
            ///
            #[doc = concat!("You may consider using the fully-qualified syntax `", stringify!($Vector), "::coord_max(a, b)` for symmetry.")]
            #[inline]
            pub fn coord_max(self, other: Self) -> Self {
                self.glam2(&other, |a, b| a.max(b))
            }

            /// Returns a new vector with each component set to 1 if the component is positive, -1 if negative, and 0 if zero.
            #[inline]
            pub fn sign(self) -> Self {
                #[inline]
                fn f(c: $Scalar) -> $Scalar {
                    let r = c.partial_cmp(&(0 as $Scalar)).unwrap_or_else(|| panic!("Vector component {c} isn't signed!"));
                    match r {
                        Ordering::Equal => 0 as $Scalar,
                        Ordering::Greater => 1 as $Scalar,
                        Ordering::Less => -1 as $Scalar,
                    }
                }

                Self::new(
                    $( f(self.$comp) ),*
                )
            }
        }
    }
}

pub(super) fn snap_one(mut value: i32, step: i32) -> i32 {
    assert!(
        value != i32::MIN || step != -1,
        "snapped() called on vector component i32::MIN with step component -1"
    );

    if step != 0 {
        // Can overflow if step / 2 + value is not in range of i32.
        let a = (step / 2).checked_add(value).expect(
            "snapped() overflowed, this happened because step / 2 + component is not in range of i32",
        );

        // Manually implement `a.div_floor(step)` since Rust's native method is still unstable, as of 1.79.0.

        // Can overflow with a == i32::MIN and step == -1 when value == i32::MIN.
        let mut d = a / step;
        // Can't overflow because if a == i32::MIN and step == -1, value == -2147483647.5 which is impossible.
        let r = a % step;
        if (r > 0 && step < 0) || (r < 0 && step > 0) {
            // Can't overflow because if d == i32::MIN than a == i32::MIN and step == 1 and value == -2147483648.5 which is impossible.
            d -= 1;
        }

        value = step * d;
    }

    value
}

/// Implements functions that are present only on integer vectors.
macro_rules! inline_impl_integer_vector_fns {
    (
        // Name of the float-equivalent vector type.
        $VectorFloat:ty,
        // Names of the components, for example `x, y`.
        $($comp:ident),*
    ) => {
        /// Returns the distance between this vector and `to`.
        ///
        /// Where possible, prefer [`distance_squared_to()`][Self::distance_squared_to] for precision and performance.
        #[inline]
        pub fn distance_to(self, to: Self) -> real {
            (to - self).length()
        }

        /// Returns the squared distance between this vector and `to`.
        ///
        /// Faster than [`distance_to()`][Self::distance_to], so prefer it if you need to compare distances, or need the squared distance
        /// in a formula.
        #[inline]
        pub fn distance_squared_to(self, to: Self) -> i32 {
            (to - self).length_squared() as i32
        }

        /// Returns `self` with each component limited to a range defined by `min` and `max`.
        ///
        /// # Panics
        /// If `min > max` on any axis.
        #[inline]
        pub fn clampi(self, min: i32, max: i32) -> Self {
            Self::new(
                $(
                    self.$comp.clamp(min, max)
                ),*
            )
        }

        /// Returns a new vector with each component set to the minimum of `self` and `with`.
        #[inline]
        pub fn mini(self, with: i32) -> Self {
            Self::new(
                $(
                    self.$comp.min(with)
                ),*
            )
        }

        /// Returns a new vector with each component set to the maximum of `self` and `with`.
        #[inline]
        pub fn maxi(self, with: i32) -> Self {
            Self::new(
                $(
                    self.$comp.max(with)
                ),*
            )
        }

        /// A new vector with each component snapped to the closest multiple of the corresponding
        /// component in `step`.
        ///
        /// # Panics
        /// On under- or overflow:
        /// - If any component of `self` is [`i32::MIN`] while the same component on `step` is `-1`.
        /// - If any component of `self` plus half of the same component of `step` is not in range on [`i32`].
        #[inline]
        pub fn snapped(self, step: Self) -> Self {
            use crate::builtin::vectors::vector_macros::snap_one;

            Self::new(
                $(
                    snap_one(self.$comp, step.$comp)
                ),*
            )
        }

        /// A new vector with each component snapped to the closest multiple of `step`.
        ///
        /// # Panics
        /// On under- or overflow (see [`snapped()`][Self::snapped] for details).
        #[inline]
        pub fn snappedi(self, step: i32) -> Self {
            self.snapped(Self::splat(step))
        }

        /// Converts to a vector with floating-point [`real`](type.real.html) components, using `as` casts.
        #[inline]
        pub const fn cast_float(self) -> $VectorFloat {
            <$VectorFloat>::new( $(self.$comp as real),* )
        }
    };
}

macro_rules! impl_float_vector_fns {
    (
        // Name of the vector type.
        $Vector:ty,
        // Name of the integer-equivalent vector type.
        $VectorInt:ty,
        // Names of the components, with parentheses, for example `(x, y)`.
        ($($comp:ident),*)
    ) => {
        /// # Float-specific functions
        ///
        /// The following methods are only available on floating-point vectors.
        impl $Vector {
            /// Converts to a vector with integer components, using `as` casts.
            pub const fn cast_int(self) -> $VectorInt {
                <$VectorInt>::new( $(self.$comp as i32),* )
            }

            /// Returns a new vector with all components rounded down (towards negative infinity).
            #[inline]
            pub fn floor(self) -> Self {
                Self::from_glam(self.to_glam().floor())
            }

            /// Returns a new vector with all components rounded up (towards positive infinity).
            #[inline]
            pub fn ceil(self) -> Self {
                Self::from_glam(self.to_glam().ceil())
            }

            /// Cubic interpolation between `self` and `b` using `pre_a` and `post_b` as handles,
            /// and returns the result at position `weight`.
            ///
            /// `weight` is on the range of 0.0 to 1.0, representing the amount of interpolation.
            #[inline]
            pub fn cubic_interpolate(self, b: Self, pre_a: Self, post_b: Self, weight: real) -> Self {
                Self::new(
                    $(
                        self.$comp.cubic_interpolate(b.$comp, pre_a.$comp, post_b.$comp, weight)
                    ),*
                )
            }

            /// Cubic interpolation between `self` and `b` using `pre_a` and `post_b` as handles,
            /// and returns the result at position `weight`.
            ///
            /// `weight` is on the range of 0.0 to 1.0, representing the amount of interpolation.
            /// It can perform smoother interpolation than [`cubic_interpolate()`][Self::cubic_interpolate] by the time values.
            #[inline]
            #[allow(clippy::too_many_arguments)]
            pub fn cubic_interpolate_in_time(
                self,
                b: Self,
                pre_a: Self,
                post_b: Self,
                weight: real,
                b_t: real,
                pre_a_t: real,
                post_b_t: real,
            ) -> Self {
                Self::new(
                    $(
                        self.$comp.cubic_interpolate_in_time(
                            b.$comp, pre_a.$comp, post_b.$comp, weight, b_t, pre_a_t, post_b_t,
                        )
                    ),*
                )
            }

            /// Returns the normalized vector pointing from this vector to `to` or [`None`], if `self` and `to` are equal.
            ///
            /// This is equivalent to using `(b - a).try_normalized()`. See also [`direction_to()`][Self::direction_to].
            #[inline]
            pub fn try_direction_to(self, to: Self) -> Option<Self> {
                (to - self).try_normalized()
            }

            /// ⚠️ Returns the normalized vector pointing from this vector to `to`.
            ///
            /// This is equivalent to using `(b - a).normalized()`. See also [`try_direction_to()`][Self::try_direction_to].
            ///
            /// # Panics
            /// If `self` and `to` are equal.
            #[inline]
            pub fn direction_to(self, to: Self) -> Self {
                self.try_direction_to(to).expect("direction_to() called on equal vectors")
            }

            /// Returns the squared distance between this vector and `to`.
            ///
            /// Faster than [`distance_to()`][Self::distance_to], so prefer it if you need to compare distances, or need the squared distance
            /// in a formula.
            #[inline]
            pub fn distance_squared_to(self, to: Self) -> real {
                (to - self).length_squared()
            }

            /// Returns the distance between this vector and `to`.
            ///
            /// Where possible, prefer [`distance_squared_to()`][Self::distance_squared_to] for performance reasons.
            #[inline]
            pub fn distance_to(self, to: Self) -> real {
                (to - self).length()
            }

            /// Returns the dot product of this vector and `with`.
            #[inline]
            pub fn dot(self, with: Self) -> real {
                self.to_glam().dot(with.to_glam())
            }

            /// Returns true if each component of this vector is finite.
            #[inline]
            pub fn is_finite(self) -> bool {
                self.to_glam().is_finite()
            }

            /// Returns `true` if the vector is normalized, i.e. its length is approximately equal to 1.
            #[inline]
            pub fn is_normalized(self) -> bool {
                self.to_glam().is_normalized()
            }

            /// Returns `true` if this vector's values are approximately zero.
            ///
            /// This method is faster than using `approx_eq()` with one value as a zero vector.
            #[inline]
            pub fn is_zero_approx(self) -> bool {
                $( self.$comp.is_zero_approx() )&&*
            }

            /// Returns the result of the linear interpolation between this vector and `to` by amount `weight`.
            ///
            /// `weight` is on the range of `0.0` to `1.0`, representing the amount of interpolation.
            #[inline]
            pub fn lerp(self, other: Self, weight: real) -> Self {
                Self::new(
                    $( self.$comp.lerp(other.$comp, weight) ),*
                )
            }

            /// Returns the vector scaled to unit length or [`None`], if called on a zero vector.
            ///
            /// Computes `self / self.length()`. See also [`normalized()`][Self::normalized] and [`is_normalized()`][Self::is_normalized].
            #[inline]
            pub fn try_normalized(self) -> Option<Self> {
                if self == Self::ZERO {
                    return None;
                }

                // Copy Godot's implementation since it's faster than using glam's normalize_or_zero().
                Some(self / self.length())
            }

            /// ⚠️ Returns the vector scaled to unit length.
            ///
            /// Computes `self / self.length()`. See also [`try_normalized()`][Self::try_normalized] and [`is_normalized()`][Self::is_normalized].
            ///
            /// # Panics
            /// If called on a zero vector.
            #[inline]
            pub fn normalized(self) -> Self {
                self.try_normalized().expect("normalized() called on zero vector")
            }

            /// Returns the vector scaled to unit length or [`Self::ZERO`], if called on a zero vector.
            ///
            /// Computes `self / self.length()`. See also [`try_normalized()`][Self::try_normalized] and [`is_normalized()`][Self::is_normalized].
            #[inline]
            pub fn normalized_or_zero(self) -> Self {
                self.try_normalized().unwrap_or_default()
            }

            /// Returns a vector composed of the [`FloatExt::fposmod()`] of this vector's components and `pmod`.
            #[inline]
            pub fn posmod(self, pmod: real) -> Self {
                Self::new(
                    $( self.$comp.fposmod(pmod) ),*
                )
            }

            /// Returns a vector composed of the [`FloatExt::fposmod()`] of this vector's components and `modv`'s components.
            #[inline]
            pub fn posmodv(self, modv: Self) -> Self {
                Self::new(
                    $( self.$comp.fposmod(modv.$comp) ),*
                )
            }

            /// Returns a new vector with all components rounded to the nearest integer, with halfway cases rounded away from zero.
            #[inline]
            pub fn round(self) -> Self {
                Self::from_glam(self.to_glam().round())
            }

            /// A new vector with each component snapped to the closest multiple of the corresponding
            /// component in `step`.
            #[inline]
            pub fn snapped(self, step: Self) -> Self {
                Self::new(
                    $(
                        self.$comp.snapped(step.$comp)
                    ),*
                )
            }
        }

        impl $crate::builtin::math::ApproxEq for $Vector {
            /// Returns `true` if this vector and `to` are approximately equal.
            #[inline]
            #[doc(alias = "is_equal_approx")]
            fn approx_eq(&self, other: &Self) -> bool {
                $( self.$comp.approx_eq(&other.$comp) )&&*
            }
        }
    };
}

macro_rules! impl_vector2x_fns {
    (
        // Name of the vector type.
        $Vector:ty,
        // Name of the 3D-equivalent vector type.
        $Vector3D:ty,
        // Type of target component, for example `real`.
        $Scalar:ty
    ) => {
        /// # 2D functions
        /// The following methods are only available on 2D vectors (for both float and int).
        impl $Vector {
            /// Returns the aspect ratio of this vector, the ratio of [`Self::x`] to [`Self::y`].
            #[inline]
            pub fn aspect(self) -> real {
                self.x as real / self.y as real
            }

            /// Returns the axis of the vector's highest value. See [`Vector2Axis`] enum. If all components are equal, this method returns [`None`].
            ///
            /// To mimic Godot's behavior, unwrap this function's result with `unwrap_or(Vector2Axis::X)`.
            ///
            #[doc = concat!("*Godot equivalent: `", stringify!($Vector), ".max_axis_index`*")]
            #[inline]
            #[doc(alias = "max_axis_index")]
            pub fn max_axis(self) -> Option<Vector2Axis> {
                match self.x.partial_cmp(&self.y) {
                    Some(Ordering::Less) => Some(Vector2Axis::Y),
                    Some(Ordering::Equal) => None,
                    Some(Ordering::Greater) => Some(Vector2Axis::X),
                    _ => None,
                }
            }

            /// Returns the axis of the vector's lowest value. See [`Vector2Axis`] enum. If all components are equal, this method returns [`None`].
            ///
            /// To mimic Godot's behavior, unwrap this function's result with `unwrap_or(Vector2Axis::Y)`.
            ///
            #[doc = concat!("*Godot equivalent: `", stringify!($Vector), ".min_axis_index`*")]
            #[inline]
            #[doc(alias = "min_axis_index")]
            pub fn min_axis(self) -> Option<Vector2Axis> {
                match self.x.partial_cmp(&self.y) {
                    Some(Ordering::Less) => Some(Vector2Axis::X),
                    Some(Ordering::Equal) => None,
                    Some(Ordering::Greater) => Some(Vector2Axis::Y),
                    _ => None,
                }
            }
        }

        impl $crate::builtin::SwizzleToVector for ($Scalar, $Scalar) {
            type Output = $Vector;
            fn swizzle_to_vector(self) -> $Vector {
                <$Vector>::new(self.0, self.1)
            }
        }
    };
}

macro_rules! impl_vector3x_fns {
    (
        // Name of the vector type.
        $Vector:ty,
        // Name of the vector type.
        $Vector2D:ty,
        // Type of target component, for example `real`.
        $Scalar:ty
    ) => {
        /// # 3D functions
        /// The following methods are only available on 3D vectors (for both float and int).
        impl $Vector {
            /// Returns the axis of the vector's highest value. See [`Vector3Axis`] enum. If all components are equal, this method returns [`None`].
            ///
            /// To mimic Godot's behavior, unwrap this function's result with `unwrap_or(Vector3Axis::X)`.
            #[inline]
            #[doc(alias = "max_axis_index")]
            pub fn max_axis(self) -> Option<Vector3Axis> {
                match self.x.partial_cmp(&self.y) {
                    Some(Ordering::Less) => match self.y.partial_cmp(&self.z) {
                        Some(Ordering::Less) => Some(Vector3Axis::Z),
                        Some(Ordering::Equal) => None,
                        Some(Ordering::Greater) => Some(Vector3Axis::Y),
                        _ => None,
                    },
                    Some(Ordering::Equal) => match self.x.partial_cmp(&self.z) {
                        Some(Ordering::Less) => Some(Vector3Axis::Z),
                        _ => None,
                    },
                    Some(Ordering::Greater) => match self.x.partial_cmp(&self.z) {
                        Some(Ordering::Less) => Some(Vector3Axis::Z),
                        Some(Ordering::Equal) => None,
                        Some(Ordering::Greater) => Some(Vector3Axis::X),
                        _ => None,
                    },
                    _ => None,
                }
            }

            /// Returns the axis of the vector's lowest value. See [`Vector3Axis`] enum. If all components are equal, this method returns [`None`].
            ///
            /// To mimic Godot's behavior, unwrap this function's result with `unwrap_or(Vector3Axis::Z)`.
            #[inline]
            #[doc(alias = "min_axis_index")]
            pub fn min_axis(self) -> Option<Vector3Axis> {
                match self.x.partial_cmp(&self.y) {
                    Some(Ordering::Less) => match self.x.partial_cmp(&self.z) {
                        Some(Ordering::Less) => Some(Vector3Axis::X),
                        Some(Ordering::Equal) => None,
                        Some(Ordering::Greater) => Some(Vector3Axis::Z),
                        _ => None,
                    },
                    Some(Ordering::Equal) => match self.x.partial_cmp(&self.z) {
                        Some(Ordering::Greater) => Some(Vector3Axis::Z),
                        _ => None,
                    },
                    Some(Ordering::Greater) => match self.y.partial_cmp(&self.z) {
                        Some(Ordering::Less) => Some(Vector3Axis::Y),
                        Some(Ordering::Equal) => None,
                        Some(Ordering::Greater) => Some(Vector3Axis::Z),
                        _ => None,
                    },
                    _ => None,
                }
            }
        }

        impl $crate::builtin::SwizzleToVector for ($Scalar, $Scalar, $Scalar) {
            type Output = $Vector;
            fn swizzle_to_vector(self) -> $Vector {
                <$Vector>::new(self.0, self.1, self.2)
            }
        }
    };
}

macro_rules! impl_vector4x_fns {
    (
        // Name of the vector type.
        $Vector:ty,
        // Type of target component, for example `real`.
        $Scalar:ty
    ) => {
        /// # 4D functions
        /// The following methods are only available on 4D vectors (for both float and int).
        impl $Vector {
            /// Returns the axis of the vector's highest value. See [`Vector4Axis`] enum. If all components are equal, this method returns [`None`].
            ///
            /// To mimic Godot's behavior, unwrap this function's result with `unwrap_or(Vector4Axis::X)`.
            #[inline]
            #[doc(alias = "max_axis_index")]
            pub fn max_axis(self) -> Option<Vector4Axis> {
                let mut max_axis = Vector4Axis::X;
                let mut previous = None;
                let mut max_value = self.x;

                let components = [
                    (Vector4Axis::Y, self.y),
                    (Vector4Axis::Z, self.z),
                    (Vector4Axis::W, self.w),
                ];

                for (axis, value) in components {
                    if value >= max_value {
                        max_axis = axis;
                        previous = Some(max_value);
                        max_value = value;
                    }
                }

                (Some(max_value) != previous).then_some(max_axis)
            }

            /// Returns the axis of the vector's lowest value. See [`Vector4Axis`] enum. If all components are equal, this method returns [`None`].
            ///
            /// To mimic Godot's behavior, unwrap this function's result with `unwrap_or(Vector4Axis::W)`.
            #[inline]
            #[doc(alias = "min_axis_index")]
            pub fn min_axis(self) -> Option<Vector4Axis> {
                let mut min_axis = Vector4Axis::X;
                let mut previous = None;
                let mut min_value = self.x;

                let components = [
                    (Vector4Axis::Y, self.y),
                    (Vector4Axis::Z, self.z),
                    (Vector4Axis::W, self.w),
                ];

                for (axis, value) in components {
                    if value <= min_value {
                        min_axis = axis;
                        previous = Some(min_value);
                        min_value = value;
                    }
                }

                (Some(min_value) != previous).then_some(min_axis)
            }
        }

        impl $crate::builtin::SwizzleToVector for ($Scalar, $Scalar, $Scalar, $Scalar) {
            type Output = $Vector;
            fn swizzle_to_vector(self) -> $Vector {
                <$Vector>::new(self.0, self.1, self.2, self.3)
            }
        }
    };
}

macro_rules! impl_vector2_vector3_fns {
    (
        // Name of the vector type.
        $Vector:ty,
        // Names of the components, with parentheses, for example `(x, y, z, w)`.
        ($($comp:ident),*)
    ) => {
        /// # 2D and 3D functions
        /// The following methods are available on both 2D and 3D float vectors.
        impl $Vector {
           /// Returns the derivative at the given `t` on the [Bézier](https://en.wikipedia.org/wiki/B%C3%A9zier_curve)
           /// curve defined by this vector and the given `control_1`, `control_2`, and `end` points.
           #[inline]
           pub fn bezier_derivative(self, control_1: Self, control_2: Self, end: Self, t: real) -> Self {
               Self::new(
                    $(
                        self.$comp.bezier_derivative(control_1.$comp, control_2.$comp, end.$comp, t)
                    ),*
               )
           }

            /// Returns the point at the given `t` on the [Bézier](https://en.wikipedia.org/wiki/B%C3%A9zier_curve)
            /// curve defined by this vector and the given `control_1`, `control_2`, and `end` points.
            #[inline]
            pub fn bezier_interpolate(self, control_1: Self, control_2: Self, end: Self, t: real) -> Self {
                Self::new(
                    $(
                        self.$comp.bezier_interpolate(control_1.$comp, control_2.$comp, end.$comp, t)
                    ),*
                )
            }

            /// Returns a new vector "bounced off" from a plane defined by the given normal.
            ///
            /// # Panics
            /// If `n` is not normalized.
            #[inline]
            pub fn bounce(self, n: Self) -> Self {
                assert!(n.is_normalized(), "n is not normalized!");
                -self.reflect(n)
            }

            /// Returns the vector with a maximum length by limiting its length to `length`.
            #[inline]
            pub fn limit_length(self, length: Option<real>) -> Self {
                let length = length.unwrap_or(1.0);

                Self::from_glam(self.to_glam().clamp_length_max(length))

            }

            /// Returns a new vector moved toward `to` by the fixed `delta` amount. Will not go past the final value.
            #[inline]
            pub fn move_toward(self, to: Self, delta: real) -> Self {
                Self::from_glam(self.to_glam().move_towards(to.to_glam(), delta))
            }

            /// Returns the result of projecting the vector onto the given vector `b`.
            #[inline]
            pub fn project(self, b: Self) -> Self {
                Self::from_glam(self.to_glam().project_onto(b.to_glam()))
            }

            /// Returns the result of reflecting the vector defined by the given direction vector `n`.
            ///
            /// # Panics
            /// If `n` is not normalized.
            #[inline]
            pub fn reflect(self, n: Self) -> Self {
                assert!(n.is_normalized(), "n is not normalized!");
                2.0 * n * self.dot(n) - self
            }

            /// Returns a new vector slid along a plane defined by the given normal.
            ///
            /// # Panics
            /// If `n` is not normalized.
            #[inline]
            pub fn slide(self, n: Self) -> Self {
                assert!(n.is_normalized(), "n is not normalized!");
                self - n * self.dot(n)
            }
        }
    };
}

macro_rules! impl_vector3_vector4_fns {
    (
        // Name of the vector type.
        $Vector:ty,
        // Names of the components, with parentheses, for example `(x, y, z, w)`.
        ($($comp:ident),*)
    ) => {
        /// # 3D and 4D functions
        /// The following methods are available on both 3D and 4D float vectors.
        impl $Vector {
            /// Returns the reciprocal (inverse) of the vector. This is the same as `1.0/n` for each component.
            #[inline]
            #[doc(alias = "inverse")]
            pub fn recip(self) -> Self {
                Self::from_glam(self.to_glam().recip())
            }
        }
    };
}
