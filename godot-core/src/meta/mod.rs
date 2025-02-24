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
//! "T-compatible" arguments into `T`. This library specializes this idea with two traits:
//!
//! - [`AsArg<T>`] allows argument conversions from arguments into `T`. This is most interesting in the context of strings (so you can pass
//!   `&str` to a function expecting `GString`), but is generic to support e.g. array insertion.
//! - [`AsObjectArg<T>`] is a more specialized version of `AsArg` that is used for object arguments, i.e. `Gd<T>`.

mod args;
mod array_type_info;
mod class_name;
mod godot_convert;
mod method_info;
mod property_info;
mod sealed;
mod signature;
mod traits;

pub mod error;

pub use args::*;
pub use class_name::ClassName;
pub use godot_convert::{FromGodot, GodotConvert, ToGodot};
pub use traits::{ArrayElement, GodotType, PackedArrayElement};

pub(crate) use array_type_info::ArrayTypeInfo;
pub(crate) use traits::{
    element_godot_type_name, element_variant_type, GodotFfiVariant, GodotNullableFfi,
};

use crate::registry::method::MethodParamOrReturnInfo;

pub(crate) use crate::{
    arg_into_owned, arg_into_ref, declare_arg_method, impl_asarg_by_ref, impl_asarg_by_value,
    impl_godot_as_self,
};

#[doc(hidden)]
pub use signature::*;

#[cfg(feature = "trace")] #[cfg_attr(published_docs, doc(cfg(feature = "trace")))]
pub use signature::trace;

pub use method_info::MethodInfo;
pub use property_info::{PropertyHintInfo, PropertyInfo};

// ----------------------------------------------------------------------------------------------------------------------------------------------

/// Clean up various resources at end of usage.
///
/// # Safety
/// Must not use meta facilities (e.g. `ClassName`) after this call.
pub(crate) unsafe fn cleanup() {
    class_name::cleanup();
}
