/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::cmp::Ordering;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::marker::PhantomData;

use crate::meta::{ClassId, GodotConvert, PropertyHintInfo};
use crate::registry::property::{Export, Var};

/// A zero-sized type for creating a property without a backing field, accessible only through custom getter/setter functions.
///
/// This must be used in a struct deriving [`GodotClass`](../register/derive.GodotClass.html) and requires that the field has
/// an explicit [`#[var]` attribute](../register/derive.GodotClass.html#register-properties--var) with a custom getter,
/// and optionally a custom setter. Both getter and setter operate on the specified type `T`.
///
/// (Note that write-only properties, with a setter but not a getter, are not currently supported.
/// Godot doesn't fully support them either, silently returning null instead of an error if the property is being read.)
///
/// # Example
///
/// Suppose you have a field `text` whose value you want to keep as a Rust `String` rather than a Godot `GString`,
/// accepting the performance penalty for conversions whenever the property is accessed from Godot:
///
/// ```
/// # use godot::prelude::*;
/// #[derive(GodotClass)]
/// #[class(init)]
/// struct Banner {
///     #[var(get = get_text, set = set_text)]
///     text: PhantomVar<GString>,
///
///     text_string: String,
/// }
///
/// #[godot_api]
/// impl Banner {
///     #[func]
///     fn get_text(&self) -> GString {
///         GString::from(&self.text_string)
///     }
///
///     #[func]
///     fn set_text(&mut self, text: GString) {
///         self.text_string = String::from(&text);
///     }
/// }
/// ```
///
/// This field can now be accessed from GDScript as `banner.text`.
// Bounds for T are somewhat un-idiomatically directly on the type, rather than impls.
// This improves error messages in IDEs when using the type as a field.
pub struct PhantomVar<T: GodotConvert + Var>(PhantomData<T>);

impl<T: GodotConvert + Var> GodotConvert for PhantomVar<T> {
    type Via = <T as GodotConvert>::Via;
}

// `PhantomVar` supports only part of `Var`, but it has to implement it, otherwise we cannot implement `Export` either.
// The `GodotClass` derive macro should ensure that the `Var` implementation is not used.
impl<T: GodotConvert + Var> Var for PhantomVar<T> {
    // Needs to be the inner type, because type-checking on user-defined getters/setters is based on this associated type.
    // In practice, #[var(pub)] cannot be used with PhantomVar.
    type PubType = T;

    fn var_get(_field: &Self) -> Self::Via {
        unreachable!("PhantomVar requires custom getter")
    }

    fn var_set(_field: &mut Self, _value: Self::Via) {
        unreachable!("PhantomVar requires custom setter")
    }

    fn var_pub_get(_field: &Self) -> Self::PubType {
        unreachable!("PhantomVar cannot be used with #[var(pub)]")
    }

    fn var_pub_set(_field: &mut Self, _value: Self::PubType) {
        unreachable!("PhantomVar cannot be used with #[var(pub)]")
    }

    fn var_hint() -> PropertyHintInfo {
        <T as Var>::var_hint()
    }
}

// Reuse values from `T`, if any.
impl<T: GodotConvert + Var + Export> Export for PhantomVar<T> {
    fn export_hint() -> PropertyHintInfo {
        <T as Export>::export_hint()
    }

    fn as_node_class() -> Option<ClassId> {
        <T as Export>::as_node_class()
    }
}

impl<T: GodotConvert + Var> Default for PhantomVar<T> {
    fn default() -> Self {
        Self(Default::default())
    }
}

// Like `PhantomData` from the Rust standard library, `PhantomVar` implements many common traits like `Eq` and `Hash`
// to allow these traits to be derived on containing structs as well.

impl<T: GodotConvert + Var> Clone for PhantomVar<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: GodotConvert + Var> Copy for PhantomVar<T> {}

impl<T: GodotConvert + Var> fmt::Debug for PhantomVar<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_tuple("PhantomVar").finish()
    }
}

impl<T: GodotConvert + Var> PartialEq for PhantomVar<T> {
    fn eq(&self, _other: &Self) -> bool {
        true
    }
}

impl<T: GodotConvert + Var> Eq for PhantomVar<T> {}

impl<T: GodotConvert + Var> PartialOrd for PhantomVar<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<T: GodotConvert + Var> Ord for PhantomVar<T> {
    fn cmp(&self, _other: &Self) -> Ordering {
        Ordering::Equal
    }
}

impl<T: GodotConvert + Var> Hash for PhantomVar<T> {
    fn hash<H: Hasher>(&self, _state: &mut H) {}
}

// SAFETY: This type contains no data.
unsafe impl<T: GodotConvert + Var> Send for PhantomVar<T> {}

// SAFETY: This type contains no data.
unsafe impl<T: GodotConvert + Var> Sync for PhantomVar<T> {}

/// This type exists only as a place to add `compile_fail` doctests for `PhantomVar`, which do not need to be in the public documentation.
///
/// Omitting the `#[var]` attribute is an error:
///
/// ```compile_fail
/// # use godot::prelude::*;
/// #[derive(GodotClass)]
/// #[class(init)]
/// struct Oops {
///     missing_var: PhantomVar<i64>,
/// }
/// ```
///
/// Declaring `#[var]` without a getter and/or setter is an error:
///
/// ```compile_fail
/// # use godot::prelude::*;
/// #[derive(GodotClass)]
/// #[class(init)]
/// struct Oops {
///     #[var]
///     missing_get_set: PhantomVar<i64>,
/// }
/// ```
///
/// Declaring `#[var]` without a getter is an error:
///
/// ```compile_fail
/// # use godot::prelude::*;
/// #[derive(GodotClass)]
/// #[class(init)]
/// struct Oops {
///     #[var(set = setter)]
///     missing_get: PhantomVar<i64>,
/// }
///
/// #[godot_api]
/// impl Oops {
///     #[func]
///     fn setter(&mut self, value: i64) {
///     }
/// }
/// ```
///
/// Declaring `#[var]` with a default getter is an error:
///
/// ```compile_fail
/// # use godot::prelude::*;
/// #[derive(GodotClass)]
/// #[class(init)]
/// struct Oops {
///     #[var(get, set = setter)]
///     default_get: PhantomVar<i64>,
/// }
///
/// #[godot_api]
/// impl Oops {
///     #[func]
///     fn setter(&mut self, value: i64) {
///     }
/// }
/// ```
///
/// Declaring `#[var]` with a default setter is an error:
///
/// ```compile_fail
/// # use godot::prelude::*;
/// #[derive(GodotClass)]
/// #[class(init)]
/// struct Oops {
///     #[var(get = getter, set)]
///     missing_set: PhantomVar<i64>,
/// }
///
/// #[godot_api]
/// impl Oops {
///     #[func]
///     fn getter(&self) -> i64 {
///         0
///     }
/// }
/// ```
#[allow(dead_code)]
struct PhantomVarDoctests;
