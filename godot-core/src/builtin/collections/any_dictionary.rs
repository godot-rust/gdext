/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! Type-erased dictionary.

use std::fmt;

use godot_ffi as sys;
use sys::{GodotFfi, ffi_methods};

use super::dictionary::{Iter, Keys};
use crate::builtin::*;
use crate::meta;
use crate::meta::error::ConvertError;
use crate::meta::{
    ArrayElement, AsVArg, ElementType, GodotConvert, GodotFfiVariant, GodotType, ToGodot,
};
use crate::registry::property::SimpleVar;

/// Covariant `Dictionary` that can be typed or untyped.
///
/// Unlike [`Dictionary<K, V>`], which carries compile-time type information, `AnyDictionary` is a type-erased version of dictionaries.
/// It can point to any `Dictionary<K, V>`, for both typed and untyped dictionaries.
///
/// # Covariance
/// In GDScript, the subtyping relationship is modeled incorrectly for typed dictionaries:
/// ```gdscript
/// var typed: Dictionary[String, int] = {"one": 1, "two": 2}
/// var untyped: Dictionary = typed   # Implicit "upcast" to Dictionary[Variant, Variant].
///
/// untyped["hello"] = "world"        # Not detected by GDScript parser (may fail at runtime).
/// ```
///
/// godot-rust on the other hand introduces a new type `AnyDictionary`, which can store _any_ dictionary, typed or untyped.
/// `AnyDictionary` provides operations that are valid regardless of the type, e.g. `len()`, `is_empty()` or `clear()`.
/// Methods which would return specific types on `Dictionary<K, V>` exist on `AnyDictionary` but return `Variant`.
///
/// `AnyDictionary` does not provide any operations where data flows _into_ to the dictionary, such as `set()` or `insert()`.
/// Note that this does **not** mean that `AnyDictionary` is an immutable view; mutating methods that are agnostic to element types (or where
/// data only flows _out_), are still available. Examples are `clear()` and `remove()`.
///
/// ## Conversions
/// - Use [`try_cast_dictionary::<K, V>()`][Self::try_cast_dictionary] to convert to a typed `Dictionary<K, V>`.
/// - Use [`try_cast_var_dictionary()`][Self::try_cast_var_dictionary] to convert to an untyped `VarDictionary`.
#[derive(PartialEq)]
#[repr(transparent)] // Guarantees same layout as VarDictionary, enabling Deref from Dictionary<K, V> (K/V have no influence on layout).
pub struct AnyDictionary {
    dict: VarDictionary,
}

impl AnyDictionary {
    pub(super) fn from_typed_or_untyped<K: ArrayElement, V: ArrayElement>(
        dict: Dictionary<K, V>,
    ) -> Self {
        // SAFETY: Dictionary<Variant, Variant> is not accessed as such, but immediately wrapped in AnyDictionary.
        let inner = unsafe { std::mem::transmute::<Dictionary<K, V>, VarDictionary>(dict) };

        Self { dict: inner }
    }

    /// Creates an empty untyped `AnyDictionary`.
    pub(crate) fn new_untyped() -> Self {
        Self {
            dict: VarDictionary::default(),
        }
    }

    fn from_opaque(opaque: sys::types::OpaqueDictionary) -> Self {
        Self {
            dict: VarDictionary::from_opaque(opaque),
        }
    }

    /// ⚠️ Returns the value for the given key, or panics.
    ///
    /// To check for presence, use [`get()`][Self::get].
    ///
    /// # Panics
    /// If there is no value for the given key. Note that this is distinct from a `NIL` value, which is returned as `Variant::nil()`.
    pub fn at(&self, key: impl AsVArg<Variant>) -> Variant {
        self.dict.at(key)
    }

    /// Returns the value for the given key, or `None`.
    ///
    /// Note that `NIL` values are returned as `Some(Variant::nil())`, while absent values are returned as `None`.
    pub fn get(&self, key: impl AsVArg<Variant>) -> Option<Variant> {
        self.dict.get(key)
    }

    /// Returns `true` if the dictionary contains the given key.
    ///
    /// _Godot equivalent: `has`_
    #[doc(alias = "has")]
    pub fn contains_key(&self, key: impl AsVArg<Variant>) -> bool {
        self.dict.contains_key(key)
    }

    /// Returns `true` if the dictionary contains all the given keys.
    ///
    /// _Godot equivalent: `has_all`_
    #[doc(alias = "has_all")]
    pub fn contains_all_keys(&self, keys: &VarArray) -> bool {
        self.dict.contains_all_keys(keys)
    }

    /// Returns the number of entries in the dictionary.
    ///
    /// _Godot equivalent: `size`_
    #[doc(alias = "size")]
    pub fn len(&self) -> usize {
        self.dict.len()
    }

    /// Returns true if the dictionary is empty.
    pub fn is_empty(&self) -> bool {
        self.dict.is_empty()
    }

    /// Returns a 32-bit integer hash value representing the dictionary and its contents.
    ///
    /// Dictionaries with equal content will always produce identical hash values. However, the reverse is not true:
    /// Different dictionaries can have identical hash values due to hash collisions.
    pub fn hash_u32(&self) -> u32 {
        self.dict.hash_u32()
    }

    /// Reverse-search a key by its value.
    ///
    /// Unlike Godot, this will return `None` if the key does not exist and `Some(key)` if found.
    ///
    /// This operation is rarely needed and very inefficient. If you find yourself needing it a lot, consider
    /// using a `HashMap` or `Dictionary` with the inverse mapping.
    ///
    /// _Godot equivalent: `find_key`_
    #[doc(alias = "find_key")]
    pub fn find_key_by_value(&self, value: impl AsVArg<Variant>) -> Option<Variant> {
        self.dict.find_key_by_value(value)
    }

    /// Removes all key-value pairs from the dictionary.
    pub fn clear(&mut self) {
        self.dict.clear()
    }

    /// Removes a key from the dictionary, and returns the value associated with the key if it was present.
    ///
    /// This is a covariant-safe operation that removes data without adding typed data.
    ///
    /// _Godot equivalent: `erase`_
    #[doc(alias = "erase")]
    pub fn remove(&mut self, key: impl AsVArg<Variant>) -> Option<Variant> {
        self.dict.remove(key)
    }

    /// Alias for [`remove()`][Self::remove].
    ///
    /// _Godot equivalent: `erase`_
    pub fn erase(&mut self, key: impl AsVArg<Variant>) -> Option<Variant> {
        self.remove(key)
    }

    /// Creates a new `Array` containing all the keys currently in the dictionary.
    ///
    /// _Godot equivalent: `keys`_
    #[doc(alias = "keys")]
    pub fn keys_array(&self) -> AnyArray {
        // Array can still be typed; so AnyArray is the only sound return type.
        // Do not use dict.keys_array() which assumes Variant typing.
        self.dict.as_inner().keys().upcast_any_array()
    }

    /// Creates a new `Array` containing all the values currently in the dictionary.
    ///
    /// _Godot equivalent: `values`_
    #[doc(alias = "values")]
    pub fn values_array(&self) -> AnyArray {
        // Array can still be typed; so AnyArray is the only sound return type.
        // Do not use dict.values_array() which assumes Variant typing.
        self.dict.as_inner().values().upcast_any_array()
    }

    /// Returns a shallow copy, sharing reference types (`Array`, `Dictionary`, `Object`...) with the original dictionary.
    ///
    /// This operation retains the dynamic key/value types: copying `Dictionary<K, V>` will yield another `Dictionary<K, V>`.
    ///
    /// To create a deep copy, use [`duplicate_deep()`][Self::duplicate_deep] instead.
    /// To create a new reference to the same dictionary data, use [`clone()`][Clone::clone].
    pub fn duplicate_shallow(&self) -> AnyDictionary {
        self.dict.duplicate_shallow().upcast_any_dictionary()
    }

    /// Returns a deep copy, duplicating nested `Array`/`Dictionary` elements but keeping `Object` elements shared.
    ///
    /// This operation retains the dynamic key/value types: copying `Dictionary<K, V>` will yield another `Dictionary<K, V>`.
    ///
    /// To create a shallow copy, use [`duplicate_shallow()`][Self::duplicate_shallow] instead.
    /// To create a new reference to the same dictionary data, use [`clone()`][Clone::clone].
    pub fn duplicate_deep(&self) -> Self {
        self.dict.duplicate_deep().upcast_any_dictionary()
    }

    /// Returns an iterator over the key-value pairs of the `Dictionary`.
    ///
    /// The pairs are each of type `(Variant, Variant)`. Each pair references the original dictionary, but instead of a `&`-reference
    /// to key-value pairs as you might expect, the iterator returns a (cheap, shallow) copy of each key-value pair.
    ///
    /// Note that it's possible to modify the dictionary through another reference while iterating over it. This will not result in
    /// unsoundness or crashes, but will cause the iterator to behave in an unspecified way.
    pub fn iter_shared(&self) -> Iter<'_> {
        self.dict.iter_shared()
    }

    /// Returns an iterator over the keys in the `Dictionary`.
    ///
    /// The keys are each of type `Variant`. Each key references the original `Dictionary`, but instead of a `&`-reference to keys
    /// as you might expect, the iterator returns a (cheap, shallow) copy of each key.
    ///
    /// Note that it's possible to modify the `Dictionary` through another reference while iterating over it. This will not result in
    /// unsoundness or crashes, but will cause the iterator to behave in an unspecified way.
    pub fn keys_shared(&self) -> Keys<'_> {
        self.dict.keys_shared()
    }

    /// Turns the dictionary into a shallow-immutable dictionary.
    ///
    /// Makes the dictionary read-only and returns the original dictionary. Disables modification of the dictionary's contents.
    /// Does not apply to nested content, e.g. elements of nested dictionaries.
    ///
    /// In GDScript, dictionaries are automatically read-only if declared with the `const` keyword.
    ///
    /// _Godot equivalent: `make_read_only`_
    #[doc(alias = "make_read_only")]
    pub fn into_read_only(self) -> Self {
        self.dict.as_inner().make_read_only();
        self
    }

    /// Returns `true` if the dictionary is read-only.
    ///
    /// See [`into_read_only()`][Self::into_read_only].
    pub fn is_read_only(&self) -> bool {
        self.dict.is_read_only()
    }

    /// Returns the runtime element type information for keys in this dictionary.
    ///
    /// The result is generally cached, so feel free to call this method repeatedly.
    pub fn key_element_type(&self) -> ElementType {
        self.dict.key_element_type()
    }

    /// Returns the runtime element type information for values in this dictionary.
    ///
    /// The result is generally cached, so feel free to call this method repeatedly.
    pub fn value_element_type(&self) -> ElementType {
        self.dict.value_element_type()
    }

    // TODO(v0.5): rename to `as_inner_unchecked` for consistency; `_mut` is misleading since receiver is `&self`.
    /// # Safety
    /// Must not be used for any "input" operations, moving elements into the dictionary -- this would break covariance.
    #[doc(hidden)]
    pub unsafe fn as_inner_mut(&self) -> inner::InnerDictionary<'_> {
        inner::InnerDictionary::from_outer(&self.dict)
    }

    /// Converts to `Dictionary<K, V>` if the runtime types match.
    ///
    /// If `K=Variant` and `V=Variant`, this will attempt to "downcast" to an untyped dictionary, identical to
    /// [`try_cast_var_dictionary()`][Self::try_cast_var_dictionary].
    ///
    /// Returns `Err(self)` if the dictionary's dynamic types differ from `K` and `V`. Check [`key_element_type()`][Self::key_element_type]
    /// and [`value_element_type()`][Self::value_element_type] before calling to determine what types the dictionary actually holds.
    ///
    /// Consumes `self`, to avoid incrementing reference-count and to be only callable on `AnyDictionary`, not `Dictionary`.
    /// Use `clone()` if you need to keep the original.
    pub fn try_cast_dictionary<K: ArrayElement, V: ArrayElement>(
        self,
    ) -> Result<Dictionary<K, V>, Self> {
        let from_key_type = self.dict.key_element_type();
        let from_value_type = self.dict.value_element_type();
        let to_key_type = ElementType::of::<K>();
        let to_value_type = ElementType::of::<V>();

        if from_key_type == to_key_type && from_value_type == to_value_type {
            // SAFETY: just checked types match.
            let dict = unsafe { std::mem::transmute::<VarDictionary, Dictionary<K, V>>(self.dict) };
            Ok(dict)
        } else {
            Err(self)
        }
    }

    /// Converts to an untyped `VarDictionary` if the dictionary is untyped.
    ///
    /// This is a shorthand for [`try_cast_dictionary::<Variant, Variant>()`][Self::try_cast_dictionary].
    ///
    /// Consumes `self`, to avoid incrementing reference-count. Use `clone()` if you need to keep the original.
    pub fn try_cast_var_dictionary(self) -> Result<VarDictionary, Self> {
        self.try_cast_dictionary::<Variant, Variant>()
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Traits

// SAFETY: See VarDictionary.
//
// We cannot provide GodotConvert with Via=VarDictionary, because ToGodot::to_godot() would otherwise enable a safe conversion from AnyDictionary to
// VarDictionary, which is not sound.
unsafe impl GodotFfi for AnyDictionary {
    const VARIANT_TYPE: sys::ExtVariantType =
        sys::ExtVariantType::Concrete(VariantType::DICTIONARY);

    // No Default trait, thus manually defining this and ffi_methods!.
    unsafe fn new_with_init(init_fn: impl FnOnce(sys::GDExtensionTypePtr)) -> Self {
        let mut result = Self::new_untyped();
        init_fn(result.sys_mut());
        result
    }

    // Manually forwarding these, since no Opaque.
    fn sys(&self) -> sys::GDExtensionConstTypePtr {
        self.dict.sys()
    }

    fn sys_mut(&mut self) -> sys::GDExtensionTypePtr {
        self.dict.sys_mut()
    }

    unsafe fn move_return_ptr(self, dst: sys::GDExtensionTypePtr, call_type: sys::PtrcallType) {
        unsafe { self.dict.move_return_ptr(dst, call_type) }
    }

    ffi_methods! { type sys::GDExtensionTypePtr = *mut Opaque;
        fn new_from_sys;
        fn new_with_uninit;
        fn from_arg_ptr;
    }
}

impl Clone for AnyDictionary {
    fn clone(&self) -> Self {
        Self {
            dict: self.dict.clone(),
        }
    }
}

impl meta::sealed::Sealed for AnyDictionary {}

impl ArrayElement for AnyDictionary {}

impl GodotConvert for AnyDictionary {
    type Via = Self;
}

impl ToGodot for AnyDictionary {
    type Pass = meta::ByValue;

    fn to_godot(&self) -> meta::ToArg<'_, Self::Via, Self::Pass> {
        self.clone()
    }

    fn to_variant(&self) -> Variant {
        self.ffi_to_variant()
    }
}

impl meta::FromGodot for AnyDictionary {
    fn try_from_godot(via: Self::Via) -> Result<Self, ConvertError> {
        Ok(via)
    }
}

// TODO(v0.5): reconsider whether AnyDictionary should implement SimpleVar (and thus Var + Export).
// It allows exporting AnyDictionary as a property, but VarDictionary already serves that purpose.
// AnyArray has the same pattern -- if changed, update both.
impl SimpleVar for AnyDictionary {}

impl fmt::Debug for AnyDictionary {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.dict.fmt(f)
    }
}

impl fmt::Display for AnyDictionary {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.dict.fmt(f)
    }
}

impl GodotType for AnyDictionary {
    type Ffi = Self;

    type ToFfi<'f>
        = meta::RefArg<'f, AnyDictionary>
    where
        Self: 'f;

    fn to_ffi(&self) -> Self::ToFfi<'_> {
        meta::RefArg::new(self)
    }

    fn into_ffi(self) -> Self::Ffi {
        self
    }

    fn try_from_ffi(ffi: Self::Ffi) -> Result<Self, ConvertError> {
        Ok(ffi)
    }

    fn godot_type_name() -> String {
        VarDictionary::godot_type_name()
    }
}

impl GodotFfiVariant for AnyDictionary {
    fn ffi_to_variant(&self) -> Variant {
        VarDictionary::ffi_to_variant(&self.dict)
    }

    fn ffi_from_variant(variant: &Variant) -> Result<Self, ConvertError> {
        // SAFETY: All element types are valid for AnyDictionary.
        let result = unsafe { VarDictionary::unchecked_from_variant(variant) };
        result.map(|inner| Self { dict: inner })
    }
}
