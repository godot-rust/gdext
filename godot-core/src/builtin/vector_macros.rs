/*
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
        // Type of each individual component, for example `i32`.
        $Scalar:ty,
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
        // Type of each individual component, for example `i32`.
        $Scalar:ty,
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
        // Type of each individual component, for example `i32`.
        $Scalar:ty,
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
        impl_vector_unary_operator!($Vector, $Scalar, ($($components),*), Neg, neg);
        impl_vector_vector_binary_operator!($Vector, $Scalar, ($($components),*), Add, add);
        impl_vector_vector_binary_operator!($Vector, $Scalar, ($($components),*), Sub, sub);
        impl_vector_vector_binary_operator!($Vector, $Scalar, ($($components),*), Mul, mul);
        impl_vector_scalar_binary_operator!($Vector, $Scalar, ($($components),*), Mul, mul);
        impl_scalar_vector_binary_operator!($Vector, $Scalar, ($($components),*), Mul, mul);
        impl_vector_vector_binary_operator!($Vector, $Scalar, ($($components),*), Div, div);
        impl_vector_scalar_binary_operator!($Vector, $Scalar, ($($components),*), Div, div);
        impl_vector_vector_assign_operator!($Vector, $Scalar, ($($components),*), AddAssign, add_assign);
        impl_vector_vector_assign_operator!($Vector, $Scalar, ($($components),*), SubAssign, sub_assign);
        impl_vector_vector_assign_operator!($Vector, $Scalar, ($($components),*), MulAssign, mul_assign);
        impl_vector_scalar_assign_operator!($Vector, $Scalar, ($($components),*), MulAssign, mul_assign);
        impl_vector_vector_assign_operator!($Vector, $Scalar, ($($components),*), DivAssign, div_assign);
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
        ($($components:ident),*),
        // Name of the enum type for the axes, for example `Vector2Axis`.
        $AxisEnum:ty,
        // Names of the enum variants, with parenthes, for example `(X, Y)`.
        ($($AxisVariants:ident),*)
    ) => {
        impl std::ops::Index<$AxisEnum> for $Vector {
            type Output = $Scalar;
            fn index(&self, axis: $AxisEnum) -> &$Scalar {
                match axis {
                    $(<$AxisEnum>::$AxisVariants => &self.$components),*
                }
            }
        }

        impl std::ops::IndexMut<$AxisEnum> for $Vector {
            fn index_mut(&mut self, axis: $AxisEnum) -> &mut $Scalar {
                match axis {
                    $(<$AxisEnum>::$AxisVariants => &mut self.$components),*
                }
            }
        }
    }
}

/// Implements functions on vector types which make sense for both floating-point and integer
/// vectors.
macro_rules! impl_common_vector_fns {
    (
        // Name of the vector type.
        $Vector:ty,
        // Type of target component, for example `real`.
        $Scalar:ty
    ) => {
        impl $Vector {
            /// Returns a new vector with all components in absolute values (i.e. positive or
            /// zero).
            #[inline]
            pub fn abs(self) -> Self {
                Self::from_glam(self.to_glam().abs())
            }

            /// Returns a new vector containing the minimum of the two vectors, component-wise.
            #[inline]
            pub fn coord_min(self, other: Self) -> Self {
                self.glam2(&other, |a, b| a.min(b))
            }

            /// Returns a new vector containing the maximum of the two vectors, component-wise.
            #[inline]
            pub fn coord_max(self, other: Self) -> Self {
                self.glam2(&other, |a, b| a.max(b))
            }
        }
    };
}

/// Implements common constants and methods for floating-point type vectors. Works for any vector
/// type that has `to_glam` and `from_glam` functions.
macro_rules! impl_float_vector_fns {
    (
        // Name of the vector type.
        $Vector:ty,
        // Type of target component, for example `real`.
        $Scalar:ty
    ) => {
        impl $Vector {
            /// Returns the length (magnitude) of this vector.
            #[inline]
            pub fn length(self) -> $Scalar {
                self.to_glam().length()
            }

            /// Returns the vector scaled to unit length. Equivalent to `self / self.length()`. See
            /// also `is_normalized()`.
            ///
            /// If the vector is zero, the result is also zero.
            #[inline]
            pub fn normalized(self) -> Self {
                Self::from_glam(self.to_glam().normalize_or_zero())
            }
        }
    };
}
