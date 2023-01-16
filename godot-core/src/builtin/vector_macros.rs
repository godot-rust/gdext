/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

#![macro_use]

/// Helper for `impl_vector` to create format strings for `format!` and friends.
macro_rules! format_string {
    ($a:ident, $b:ident) => { "({}, {})" };
    ($a:ident, $b:ident, $c: ident) => { "({}, {}, {})" };
    ($a:ident, $b:ident, $c: ident, $d:ident) => { "({}, {}, {}, {})" };
}

/// Helper for `impl_vector` to implement a set of operators for a vector type.
macro_rules! impl_vector_operators {
    (
        // Name of the vector type.
        $vector:ty,
        // Type of each individual component, for example `i32`.
        $component_type:ty,
        // Name of the operator trait.
        $operator:ident
    ) => {
        // `paste` is used for conversion to snake-case: AddAssign -> add_assign.
        paste::paste! {
            // vector + vector
            impl std::ops::$operator for $vector {
                type Output = Self;
                fn [<$operator:snake>](self, rhs: $vector) -> Self::Output {
                    self.0.[<$operator:snake>](rhs.0).into()
                }
            }

            // vector + scalar
            impl std::ops::$operator<$component_type> for $vector {
                type Output = Self;
                fn [<$operator:snake>](self, rhs: $component_type) -> Self::Output {
                    self.0.[<$operator:snake>](rhs).into()
                }
            }

            // scalar + vector
            impl std::ops::$operator<$vector> for $component_type {
                type Output = $vector;
                fn [<$operator:snake>](self, rhs: $vector) -> Self::Output {
                    self.[<$operator:snake>](rhs.0).into()
                }
            }

            // vector += vector
            impl std::ops::[<$operator Assign>] for $vector {
                fn [<$operator:snake _assign>](&mut self, rhs: $vector) {
                    self.0.[<$operator:snake _assign>](rhs.0);
                }
            }

            // vector += scalar
            impl std::ops::[<$operator Assign>]<$component_type> for $vector {
                fn [<$operator:snake _assign>](&mut self, rhs: $component_type) {
                    self.0.[<$operator:snake _assign>](rhs);
                }
            }
        }
    }
}

/// Implements the basics of a built-in vector type.
macro_rules! impl_vector {
    (
        // Name of the vector type to be created.
        $vector:ident,
        // Name of the inner (wrapped) type, typically from glam.
        $inner_type:ty,
        // Type of each individual component, for example `i32`.
        $component_type:ty,
        // Names of the components, for example `(x, y)`.
        ($($components:ident),*)$(,)?
    ) => {
        // Inside a `paste!` invocation, everything between [<...>] gets concatenated.
        paste::paste! {
            #[derive(Default, Copy, Clone, Debug, PartialEq)]
            #[repr(C)]
            pub struct $vector($inner_type);

            impl $vector {
                /// Creates a new vector with the given components.
                #[inline]
                pub const fn new($($components: $component_type),*) -> Self {
                    Self($inner_type::new($($components),*))
                }

                /// Creates a new vector with all components set to `v`.
                #[inline]
                pub const fn splat(v: $component_type) -> Self {
                    Self($inner_type::splat(v))
                }

                $(
                    #[doc = "Returns the `" $components "` component of this vector."]
                    #[inline]
                    pub fn $components(&self) -> $component_type {
                        self.0.$components
                    }

                    #[doc = "Sets the `" $components "` component of this vector."]
                    #[inline]
                    pub fn [<set_ $components>](&mut self, $components: $component_type) {
                        self.0.$components = $components;
                    }
                )*
            }

            impl From<$inner_type> for $vector {
                /// Wraps an inner type in a Godot vector.
                fn from(inner: $inner_type) -> Self {
                    Self(inner)
                }
            }

            impl From<$vector> for $inner_type {
                /// Unwraps a Godot vector into its inner type.
                fn from(vector: $vector) -> Self {
                    vector.0
                }
            }

            impl std::fmt::Display for $vector {
                /// Formats this vector in the same way the Godot engine would.
                fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                    write!(
                        f,
                        format_string!($($components),*),
                        $(self.$components()),*)
                }
            }

            impl_vector_operators!($vector, $component_type, Add);
            impl_vector_operators!($vector, $component_type, Sub);
            impl_vector_operators!($vector, $component_type, Mul);
            impl_vector_operators!($vector, $component_type, Div);

            impl std::ops::Neg for $vector {
                type Output = Self;
                fn neg(self) -> Self {
                    self.0.neg().into()
                }
            }

            impl godot_ffi::GodotFfi for $vector {
                godot_ffi::ffi_methods! { type godot_ffi::GDExtensionTypePtr = *mut Self; .. }
            }
        }
    }
}

/// Implements `From` that does a component-wise cast to convert one vector type to another.
macro_rules! impl_vector_from {
    (
        // Name of the vector type.
        $vector:ty,
        // Name of the original type.
        $from:ty,
        // Type of target component, for example `Real`.
        $component_type:ty,
        // Names of the components, for example `(x, y)`.
        ($($components:ident),*)$(,)?
    ) => {
        paste::paste! {
            impl From<$from> for $vector {
                #[doc = "Converts a `" $from "` into a `" $vector "`. Note that this might be a lossy operation."]
                fn from(from: $from) -> Self {
                    Self::new($(from.$components() as $component_type),*)
                }
            }
        }
    }
}

/// Implements common constants and methods for floating-point type vectors.
macro_rules! impl_float_vector {
    (
        // Name of the vector type.
        $vector:ty,
        // Type of target component, for example `Real`.
        $component_type:ty
    ) => {
        impl $vector {
            /// Zero vector, a vector with all components set to `0.0`.
            pub const ZERO: Self = Self::splat(0.0);

            /// One vector, a vector with all components set to `1.0`.
            pub const ONE: Self = Self::splat(1.0);

            /// Infinity vector, a vector with all components set to `INFIINTY`.
            pub const INF: Self = Self::splat(<$component_type>::INFINITY);

            /// Returns the length (magnitude) of this vector.
            #[inline]
            pub fn length(&self) -> $component_type {
                self.0.length()
            }

            /// Returns the vector scaled to unit length. Equivalent to `self / self.length()`. See
            /// also `is_normalized()`.
            #[inline]
            pub fn normalized(&self) -> Self {
                self.0.normalize().into()
            }
        }
    }
}
