/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

mod impls;

use crate::builtin::Variant;
use crate::meta::error::ConvertError;
use crate::meta::shape::GodotShape;
use crate::meta::traits::GodotFfiVariant;
use crate::meta::{ArgPassing, GodotType, ThreadSafety, ToArg};

/// Indicates that a type can be passed to/from Godot, either directly or through an intermediate "via" type.
///
/// The associated type `Via` specifies _how_ this type is passed across the FFI boundary to/from Godot.
/// Generally [`ToGodot`] needs to be implemented to pass a type to Godot, and [`FromGodot`] to receive this type from Godot.
///
/// [`GodotType`] is a stronger bound than [`GodotConvert`], since it expresses that a type is _directly_ representable
/// in Godot (without intermediate "via"). Every `GodotType` also implements `GodotConvert` with `Via = Self`.
///
/// Please read the [`godot::meta` module docs](index.html) for further information about conversions.
///
/// # u64
/// The type `u64` is **not** supported by `ToGodot` and `FromGodot` traits. You can thus not pass it in `#[func]` parameters/return types.
///
/// The reason is that Godot's `Variant` type, and therefore also GDScript, only support _signed_ 64-bit integers (`i64`).
/// Implicitly wrapping `u64` to `i64` would be surprising behavior, as the value could suddenly change for large numbers.
/// As such, godot-rust leaves this decision to users: it's possible to define a newtype around `u64` with custom `ToGodot`/`FromGodot` impls.
#[doc(alias = "via", alias = "transparent")]
#[diagnostic::on_unimplemented(
    message = "`GodotConvert` is needed for `#[func]` parameters/returns, as well as `#[var]` and `#[export]` properties",
    note = "check following errors for more information"
)]
pub trait GodotConvert {
    /// The type through which `Self` is represented in Godot.
    type Via: GodotType;

    /// Which "shape" this type has for property registration (e.g. builtin, enum, ...).
    ///
    /// godot-rust derives property hints, class names, usage flags, and element metadata from this.
    fn godot_shape() -> GodotShape;
}

/// Defines the canonical conversion to Godot for a type.
///
/// It is assumed that all the methods return equal values given equal inputs. Additionally, it is assumed
/// that if [`FromGodot`] is implemented, converting to Godot and back again will return a value equal to the
/// starting value.
///
/// Violating these assumptions is safe but will give unexpected results.
///
/// Please read the [`godot::meta` module docs](index.html) for further information about conversions.
///
/// This trait can be derived using the [`#[derive(GodotConvert)]`](../register/derive.GodotConvert.html) macro.
///
/// # `Result<T, E>`
/// It is possible to return `Result<T, E>` from `#[func]`, when `T: ToGodot` and [`E: ErrorToGodot`][crate::meta::error::ErrorToGodot].
/// However, `Result<T, E>` currently does not implement `ToGodot` itself, as it is not generally infallible.
///
/// # Panics
/// Currently, the methods `to_godot()`, `to_godot_owned()` and `to_variant()` are infallible and never panic, i.e. you can convert every value
/// to a Godot representation. If new types are supported in the future that may not satisfy this (example: `Result<T, E>`), it's possible
/// that panics are introduced _only for those new types_.
#[diagnostic::on_unimplemented(
    message = "passing type `{Self}` to Godot requires `ToGodot` trait, which is usually provided by the library",
    note = "ToGodot is implemented for built-in types (i32, Vector2, GString, …). For objects, use Gd<T> instead of T.",
    note = "if you really need a custom representation (for non-class types), implement ToGodot manually or use #[derive(GodotConvert)].",
    note = "see also: https://godot-rust.github.io/docs/gdext/master/godot/meta"
)]
pub trait ToGodot: Sized + GodotConvert {
    /// Whether arguments of this type are passed by value or by reference.
    ///
    /// Can be either [`ByValue`][crate::meta::ByValue] or [`ByRef`][crate::meta::ByRef]. In most cases, you need `ByValue`.
    ///
    /// Select `ByValue` if:
    /// - `Self` is `Copy` (e.g. `i32`, `f64`, `Vector2`, `Color`, etc).
    /// - You need a conversion (e.g. `Self = MyString`, `Via = GString`).
    /// - You like the simple life and can't be bothered with lifetimes.
    ///
    /// Select `ByRef` if:
    /// - Performance of argument passing is very important and you have measured it.
    /// - You store a cached value which can be borrowed (e.g. `&GString`).
    ///
    /// Will auto-implement [`AsArg<T>`][crate::meta::AsArg] for either `T` (by-value) or for `&T` (by-reference).
    /// This has an influence on contexts such as [`Array::push()`][crate::builtin::Array::push], the [`array![...]`][crate::builtin::array]
    /// macro or generated signal `emit()` signatures.
    type Pass: ArgPassing;

    /// Whether arguments of this type are thread-safe or not.
    ///
    /// Can be either [`ThreadSafeArg`](crate::meta::ThreadSafeArg) or [`NonThreadSafeArg`](crate::meta::NonThreadSafeArg). Only engine
    /// types make use of `NonThreadSafeArg`, all user defined types should use `ThreadSafeArg` by deriving [`GodotConvert`] or by manually
    /// implementing this trait. The use of `ThreadSafeArg` also requires the type to be [`Send`]. Non [`Send`] user defined types are
    /// currenlty not supported.
    type Threads: ThreadSafety;

    /// Converts this type to Godot representation, optimizing for zero-copy when possible.
    ///
    /// # Return type
    /// - For `Pass = ByValue`, returns owned `Self::Via`.
    /// - For `Pass = ByRef`, returns borrowed `&Self::Via`.
    fn to_godot(&self) -> ToArg<'_, Self::Via, Self::Pass>;

    /// Converts this type to owned Godot representation.
    ///
    /// Always returns `Self::Via`, cloning if necessary for ByRef types.
    fn to_godot_owned(&self) -> Self::Via {
        Self::Pass::ref_to_owned_via(self)
    }

    /// Converts this type to a [Variant].
    // Exception safety: introducing a panic would have invariant implications, e.g. in Array::resize().
    fn to_variant(&self) -> Variant {
        Self::Pass::ref_to_variant(self)
    }
}

/// Defines the canonical conversion from Godot for a type.
///
/// It is assumed that all the methods return equal values given equal inputs. Additionally, it is assumed
/// that if [`ToGodot`] is implemented, converting to Godot and back again will return a value equal to the
/// starting value.
///
/// Violating these assumptions is safe but will give unexpected results.
///
/// Please read the [`godot::meta` module docs](index.html) for further information about conversions.
///
/// This trait can be derived using the [`#[derive(GodotConvert)]`](../register/derive.GodotConvert.html) macro.
#[diagnostic::on_unimplemented(
    message = "receiving type `{Self}` from Godot requires `FromGodot` trait, which is usually provided by the library",
    note = "FromGodot is implemented for built-in types (i32, Vector2, GString, …). For objects, use Gd<T> instead of T.",
    note = "if you really need a custom representation (for non-class types), implement FromGodot manually or use #[derive(GodotConvert)]",
    note = "see also: https://godot-rust.github.io/docs/gdext/master/godot/meta"
)]
pub trait FromGodot: Sized + GodotConvert {
    /// Converts the Godot representation to this type, returning `Err` on failure.
    fn try_from_godot(via: Self::Via) -> Result<Self, ConvertError>;

    /// ⚠️ Converts the Godot representation to this type.
    ///
    /// # Panics
    /// If the conversion fails.
    fn from_godot(via: Self::Via) -> Self {
        Self::try_from_godot(via)
            .unwrap_or_else(|err| panic!("FromGodot::from_godot() failed: {err}"))
    }

    /// Performs the conversion from a [`Variant`], returning `Err` on failure.
    fn try_from_variant(variant: &Variant) -> Result<Self, ConvertError> {
        let ffi = <Self::Via as GodotType>::Ffi::ffi_from_variant(variant)?;

        let via = Self::Via::try_from_ffi(ffi)?;
        Self::try_from_godot(via)
    }

    /// ⚠️ Performs the conversion from a [`Variant`].
    ///
    /// # Panics
    /// If the conversion fails.
    fn from_variant(variant: &Variant) -> Self {
        Self::try_from_variant(variant).unwrap_or_else(|err| {
            panic!("FromGodot::from_variant() failed -- {err}");
        })
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Engine conversion traits (for APIs and virtual methods, not user-facing #[func])

/// Engine-internal variant of [`ToGodot`], used for engine APIs and virtual methods.
///
/// This trait exists to support types like `u64` that work in engine contexts (backed by C++ `uint64_t`), but cannot be used in user-facing
/// `#[func]` methods (as they don't fit in GDScript/Variant, which can only store `i64`).
///
/// On the FFI level, `u64` are passed as `i64` (same bit pattern). The C++ side reinterprets the bits again as `uint64_t` in engine APIs
/// and bitfields. User-defined GDScript code generally does not get into contact with `i64` (except for bitfields).
///
/// For internal use only; see [`ToGodot`] for user-facing conversions.
#[doc(hidden)]
pub trait EngineToGodot: Sized + GodotConvert {
    /// Whether arguments of this type are passed by value or by reference.
    type Pass: ArgPassing;

    /// Converts this type to Godot representation, optimizing for zero-copy when possible.
    fn engine_to_godot(&self) -> ToArg<'_, Self::Via, Self::Pass>;

    /// Converts this type to owned Godot representation.
    fn engine_to_godot_owned(&self) -> Self::Via {
        Self::Pass::ref_to_owned_via(self)
    }

    fn engine_to_variant(&self) -> Variant;

    /// Consuming conversion to `Variant` for `#[func]` varcall return values. Relevant for `Result<T, E>`.
    ///
    /// Defaults to infallible [`Self::engine_to_variant()`] for types without `ToGodot` (e.g. `u64`). For `ToGodot` types,
    /// the blanket impl delegates to [`ToGodot::__godot_try_into_variant()`].
    //
    // Could alternatively be avoided by splitting Signature in-call methods into `in_varcall`/`in_ptrcall` (EngineToGodot, for virtual
    // methods + property accessors) and `in_func_varcall`/`in_func_ptrcall` (ToGodot, for #[func]). That avoids this trait method but
    // duplicates more code in signature.rs and requires is_func plumbing in the macro. Trying this resulted in ~120 additional LoC.
    fn engine_try_into_variant(
        self,
        _call_ctx: &crate::meta::CallContext,
    ) -> Result<Variant, crate::meta::error::CallError> {
        Ok(self.engine_to_variant())
    }

    /// Consuming conversion to the Godot `Via` type for `#[func]` ptrcall return values.
    ///
    /// Defaults to infallible [`Self::engine_to_godot_owned()`]. For `ToGodot` types,
    /// the blanket impl delegates to [`ToGodot::__godot_try_into_godot_owned()`].
    fn engine_try_into_godot_owned(
        self,
        _call_ctx: &crate::meta::CallContext,
    ) -> Result<Self::Via, crate::meta::error::CallError> {
        Ok(self.engine_to_godot_owned())
    }
}

// Blanket implementations: all user-facing types work in engine contexts.
impl<T: ToGodot> EngineToGodot for T {
    type Pass = T::Pass;

    fn engine_to_godot(&self) -> ToArg<'_, Self::Via, Self::Pass> {
        <T as ToGodot>::to_godot(self)
    }

    fn engine_to_godot_owned(&self) -> Self::Via {
        <T as ToGodot>::to_godot_owned(self)
    }

    fn engine_to_variant(&self) -> Variant {
        <T as ToGodot>::to_variant(self)
    }

    fn engine_try_into_variant(
        self,
        _call_ctx: &crate::meta::CallContext,
    ) -> Result<Variant, crate::meta::error::CallError> {
        Ok(self.to_variant())
    }

    fn engine_try_into_godot_owned(
        self,
        _call_ctx: &crate::meta::CallContext,
    ) -> Result<Self::Via, crate::meta::error::CallError> {
        Ok(self.to_godot_owned())
    }
}

/// Engine-internal variant of [`FromGodot`], used for engine APIs and virtual methods.
///
/// See [`EngineToGodot`] for rationale.
///
/// For internal use only; see [`FromGodot`] for user-facing conversions.
#[doc(hidden)]
pub trait EngineFromGodot: Sized + GodotConvert {
    /// Converts the Godot representation to this type, returning `Err` on failure.
    fn engine_try_from_godot(via: Self::Via) -> Result<Self, ConvertError>;

    fn engine_try_from_variant(variant: &Variant) -> Result<Self, ConvertError>;
}

impl<T: FromGodot> EngineFromGodot for T {
    fn engine_try_from_godot(via: Self::Via) -> Result<Self, ConvertError> {
        <T as FromGodot>::try_from_godot(via)
    }

    fn engine_try_from_variant(variant: &Variant) -> Result<Self, ConvertError> {
        <T as FromGodot>::try_from_variant(variant)
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Impls

#[macro_export]
macro_rules! impl_godot_as_self {
    ($T:ty: $Passing:ident) => {
        $crate::impl_godot_as_self!($T: $Passing, ThreadSafeArg);
    };

    ($T:ty: $Passing:ident, $Threads:ident) => {
        impl $crate::meta::GodotConvert for $T {
            type Via = $T;

            fn godot_shape() -> $crate::meta::shape::GodotShape {
                $crate::meta::shape::GodotShape::of_builtin::<$T>()
            }
        }

        $crate::impl_godot_as_self!(@to_godot $T: $Passing, $Threads);

        impl $crate::meta::FromGodot for $T {
            #[inline]
            fn try_from_godot(via: Self::Via) -> Result<Self, $crate::meta::error::ConvertError> {
                Ok(via)
            }
        }
    };

    (@to_godot $T:ty: ByValue, $Threads:ident) => {
        impl $crate::meta::ToGodot for $T {
            type Pass = $crate::meta::ByValue;
            type Threads = $crate::meta::$Threads;

            #[inline]
            fn to_godot(&self) -> Self::Via {
                self.clone()
            }
        }
    };

    (@to_godot $T:ty: ByRef, $Threads:ident) => {
        impl $crate::meta::ToGodot for $T {
            type Pass = $crate::meta::ByRef;
            type Threads = $crate::meta::$Threads;

            #[inline]
            fn to_godot(&self) -> &Self::Via {
                self
            }
        }
    };

    (@to_godot $T:ty: ByVariant, $Threads:ident) => {
        impl $crate::meta::ToGodot for $T {
            type Pass = $crate::meta::ByVariant;
            type Threads = $crate::meta::$Threads;

            #[inline]
            fn to_godot(&self) -> &Self::Via {
                self
            }
        }
    };
}
