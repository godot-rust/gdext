/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
use crate::obj::bounds::{DeclEngine, DeclUser};
use crate::obj::{Gd, GdMut, GdRef, GodotClass, WithBaseField};
use std::ops::{Deref, DerefMut};

/// Provides a unified way of both user and engine classes to be explicitly dereferenced.
///
/// This is mainly used by the `connect_**` functions of [`TypedSignal`](crate::registry::signal::TypedSignal).
///
/// # Motivation
/// Although both user and engine classes are often wrapped in a `Gd<T>`, dereferencing them is done differently depending
/// on whether they are made by the user or engine:
/// - `Gd<EngineClass>` can be de-refed directly into `&EngineClass` and `&mut EngineClass`.
/// - `Gd<UserClass>` must first go through [`bind()`](Gd::bind)/[`bind_mut()`](Gd::bind_mut), which can finally
///   be de-refed into `&UserClass` and `&mut UserClass`, respectively.
///
/// Without this trait, there's no clear/generic way of writing functions that can accept both user and engine classes
/// but need to deref said classes in some way.
///
/// [`GodotDeref`](Self) solves this explicitly handling each category in a different way, but still resulting
/// in a variable that can be de-refed into `&T`/`&mut T`.
///
/// # Generic param `Decl`
/// Rustc [does not acknowledge associated type bounds when checking for overlapping impls](https://github.com/rust-lang/rust/issues/20400),
/// this parameter is essentially used to create 2 different traits, one for each "category" (user or engine).
///
/// Despite being 2 different traits, a function can accept both by simply being generic over `Decl`:
/// ```rust ignore
/// fn o_minus<Decl, C: GodotDeref<Decl>>(obj: &Gd<C>) {
///     let ref_provider = obj.get_ref();  
///     let obj_ref: &C = & *ref_provider; // regardless of `Decl`, we can still deref since the bounds on `TargetRef`/`TargetMut` enforce that.
/// }
/// ```
///
// The crate `https://crates.io/crates/disjoint_impls` handles this in a more user-friendly way, we should
// consider using it if disjoint impls are going to be frequently used.
#[allow(clippy::needless_lifetimes)] // False positive.
pub trait GodotDeref<Decl>: GodotClass {
    type TargetRef<'a>: Deref<Target = Self>;
    type TargetMut<'a>: DerefMut<Target = Self>;

    fn get_ref<'a>(gd: &'a Gd<Self>) -> Self::TargetRef<'a>;
    fn get_mut<'a>(gd: &'a mut Gd<Self>) -> Self::TargetMut<'a>;
}

#[allow(clippy::needless_lifetimes)] // False positive.
impl<T: GodotClass<Declarer = DeclEngine>> GodotDeref<DeclEngine> for T {
    type TargetRef<'a> = Gd<T>;
    type TargetMut<'a> = Gd<T>;

    fn get_ref<'a>(gd: &'a Gd<Self>) -> Self::TargetRef<'a> {
        gd.clone()
    }
    fn get_mut<'a>(gd: &'a mut Gd<Self>) -> Self::TargetMut<'a> {
        gd.clone()
    }
}

#[allow(clippy::needless_lifetimes)] // False positive.
impl<T: WithBaseField> GodotDeref<DeclUser> for T {
    type TargetRef<'a> = GdRef<'a, T>;
    type TargetMut<'a> = GdMut<'a, T>;

    fn get_ref<'a>(gd: &'a Gd<Self>) -> Self::TargetRef<'a> {
        gd.bind()
    }
    fn get_mut<'a>(gd: &'a mut Gd<Self>) -> Self::TargetMut<'a> {
        gd.bind_mut()
    }
}
