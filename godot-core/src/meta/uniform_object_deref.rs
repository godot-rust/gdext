/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::ops::{Deref, DerefMut};

use crate::obj::bounds::{DeclEngine, DeclUser};
use crate::obj::{Gd, GdMut, GdRef, GodotClass, WithBaseField};

/// Unifies dereferencing of user and engine classes, as `&T`/`&mut T` and `Gd<T>`.
///
/// This is mainly used by the `connect_*` functions of [`TypedSignal`](crate::registry::signal::TypedSignal).
///
/// # Motivation
/// Although both user and engine classes are often wrapped in a `Gd<T>`, dereferencing them is done differently depending
/// on whether they are made by the user or engine:
/// - `Gd<EngineClass>` can be deref-ed directly into `&EngineClass` and `&mut EngineClass`.
/// - `Gd<UserClass>` must first go through [`bind()`](Gd::bind)/[`bind_mut()`](Gd::bind_mut), which can finally
///   be deref-ed into `&UserClass` and `&mut UserClass`, respectively.
///
/// Without this trait, there's no clear/generic way of writing functions that can accept both user and engine classes,
/// but need to deref said classes in some way.
///
/// [`UniformObjectDeref`](Self) solves this by explicitly handling each category in a different way, but still resulting
/// in a variable that can be deref-ed into `&T`/`&mut T`.
///
/// # Generic param `Declarer`
/// Rustc [does not acknowledge associated type bounds when checking for overlapping impls](https://github.com/rust-lang/rust/issues/20400),
/// this parameter is essentially used to create 2 different traits, one for each "category" (user or engine).
///
/// Despite being 2 different traits, a function can accept both by simply being generic over `Declarer`:
/// ```no_run
/// use godot::meta::UniformObjectDeref;
/// # use godot::prelude::*;
///
/// fn abstract_over_objects<Declarer, C>(obj: &Gd<C>)
/// where
///     C: UniformObjectDeref<Declarer>,
/// {
///     let ref_provider = UniformObjectDeref::object_as_ref(obj);
///     let obj_ref: &C = & *ref_provider;
///     // Regardless of `Declarer`, we can still deref, since the bounds on
///     // `TargetRef`/`TargetMut` enforce that.
/// }
///
/// #[derive(GodotClass)]
/// #[class(init)]
/// struct MyClass {
///     _base: Base<RefCounted>
/// }
///
/// fn main() {
///     let engine_obj: Gd<RefCounted> = RefCounted::new_gd();
///     let user_obj: Gd<MyClass> = MyClass::new_gd();
///
///     abstract_over_objects(&engine_obj);
///     abstract_over_objects(&user_obj);
/// }
/// ```
///
/// # Similar traits
/// - [`ObjectToOwned`][crate::meta::ObjectToOwned] provides conversion from `&self` or `&Gd<T>` to owned `Gd<T>`.
//
// The crate `https://crates.io/crates/disjoint_impls` handles this in a more user-friendly way, we should
// consider using it if disjoint impls are going to be frequently used.
//
// See also Declarer::DerefTarget in the library, which has a similar but different purpose: finding the nearest engine class
// (`&Node` for `Node`, `&Node` for `MyClass`).
#[allow(clippy::needless_lifetimes)] // False positive.
pub trait UniformObjectDeref<Declarer>: GodotClass {
    // Currently, only the mut parts are used within the library; but ref might be useful too.
    type TargetRef<'a>: Deref<Target = Self>;
    type TargetMut<'a>: DerefMut<Target = Self>;

    fn object_as_ref<'a>(gd: &'a Gd<Self>) -> Self::TargetRef<'a>;
    fn object_as_mut<'a>(gd: &'a mut Gd<Self>) -> Self::TargetMut<'a>;
}

#[allow(clippy::needless_lifetimes)] // False positive.
impl<T: GodotClass<Declarer = DeclEngine>> UniformObjectDeref<DeclEngine> for T {
    type TargetRef<'a> = Gd<T>;
    type TargetMut<'a> = Gd<T>;

    fn object_as_ref<'a>(gd: &'a Gd<Self>) -> Self::TargetRef<'a> {
        gd.clone()
    }
    fn object_as_mut<'a>(gd: &'a mut Gd<Self>) -> Self::TargetMut<'a> {
        gd.clone()
    }
}

#[allow(clippy::needless_lifetimes)] // False positive.
impl<T: WithBaseField> UniformObjectDeref<DeclUser> for T {
    type TargetRef<'a> = GdRef<'a, T>;
    type TargetMut<'a> = GdMut<'a, T>;

    fn object_as_ref<'a>(gd: &'a Gd<Self>) -> Self::TargetRef<'a> {
        gd.bind()
    }
    fn object_as_mut<'a>(gd: &'a mut Gd<Self>) -> Self::TargetMut<'a> {
        gd.bind_mut()
    }
}
