/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! Meta-information about Godot types, their properties and conversions between them.
//!
//! # Conversions between types
//!
//! ## Godot representation
//!
//! The library provides two traits [`FromGodot`] and [`ToGodot`], which are used at the Rust <-> Godot boundary, both in user-defined functions
//! ([`#[func]`](../register/attr.godot_api.html#user-defined-functions)) and engine APIs ([`godot::classes` module](crate::classes)).
//! Their `to_godot()` and `from_godot()` methods convert types from/to their _closest possible Godot type_ (e.g. `GString` instead of Rust
//! `String`). You usually don't need to call these methods yourself, they are automatically invoked when passing objects to/from Godot.
//!
//! Most often, the two traits appear in pairs, however there are cases where only one of the two is implemented. For example, `&str` implements
//! `ToGodot` but not `FromGodot`. Additionally, [`GodotConvert`] acts as a supertrait of both [`FromGodot`] and [`ToGodot`]. Its sole purpose
//! is to define the "closest possible Godot type" [`GodotConvert::Via`].
//!
//! For fallible conversions, you can use [`FromGodot::try_from_godot()`].
//!
//! ## Variants
//!
//! [`ToGodot`] and [`FromGodot`] also implement a conversion to/from [`Variant`][crate::builtin::Variant], which is the most versatile Godot
//! type. This conversion is available via `to_variant()` and `from_variant()` methods. These methods are also available directly on `Variant`
//! itself, via `to()`, `try_to()` and `from()` functions.
//!
//! ## Class conversions
//!
//! Godot classes exist in a hierarchy. In OOP, it is usually possible to represent pointers to derived objects as pointer to their bases.
//! For conversions between base and derived class objects, you can use `Gd` methods [`cast()`][crate::obj::Gd::cast],
//! [`try_cast()`][crate::obj::Gd::try_cast] and [`upcast()`][crate::obj::Gd::upcast]. Upcasts are infallible.
//!
//! ## Argument conversions
//!
//! Rust does not support implicit conversions, however it has something very close: the `impl Into<T>` idiom, which can be used to convert
//! "T-compatible" arguments into `T`.
//!
//! This library specializes this idea with the trait [`AsArg<T>`]. `AsArg` allows argument conversions from arguments into `T`.
//! This is most interesting in the context of strings (so you can pass `&str` to a function expecting `GString`) and objects (pass
//! `&Gd<Node2D>` to a function expecting `Node2D` objects).

mod args;
mod class_id;
mod element_type;
mod godot_convert;
mod method_info;
mod object_to_owned;
mod param_tuple;
mod property_info;
mod raw_ptr;
mod signature;
mod traits;
mod uniform_object_deref;

pub(crate) mod sealed;

pub mod error;
pub mod inspect;
pub(crate) mod signed_range;

// Public re-exports
pub use args::*;
pub use class_id::ClassId;
pub use element_type::{ElementScript, ElementType};
pub use godot_convert::{EngineFromGodot, EngineToGodot, FromGodot, GodotConvert, ToGodot};
pub use method_info::MethodInfo;
pub use object_to_owned::ObjectToOwned;
pub use param_tuple::{InParamTuple, OutParamTuple, ParamTuple, TupleFromGodot};
pub use property_info::{PropertyHintInfo, PropertyInfo};
pub use raw_ptr::{FfiRawPointer, RawPtr};
#[cfg(feature = "trace")]
pub use signature::trace;
#[doc(hidden)]
pub use signature::*;
pub use signed_range::{wrapped, SignedRange};
pub use traits::{ArrayElement, GodotImmutable, GodotType, PackedArrayElement};
pub use uniform_object_deref::UniformObjectDeref;

// Public due to signals emit() needing it. Should be made pub(crate) again if that changes.
pub use crate::arg_into_owned;

// Crate-local re-exports
mod reexport_crate {
    pub(crate) use super::traits::{
        element_godot_type_name, element_variant_type, ffi_variant_type, ExtVariantType,
        GodotFfiVariant, GodotNullableFfi,
    };
    // Private imports for this module only.
    pub(super) use crate::registry::method::MethodParamOrReturnInfo;
    pub(crate) use crate::{arg_into_ref, declare_arg_method, impl_godot_as_self};
}
pub(crate) use reexport_crate::*;

// ----------------------------------------------------------------------------------------------------------------------------------------------

/// Clean up various resources at end of usage.
///
/// # Safety
/// Must not use meta facilities (e.g. `ClassId`) after this call.
pub(crate) unsafe fn cleanup() {
    class_id::cleanup();
}
