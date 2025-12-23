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

use crate::builtin::*;
use crate::meta;
use crate::meta::error::ConvertError;
use crate::meta::inspect::ElementType;
use crate::meta::shape::GodotShape;
use crate::meta::{AsArg, Element, FromGodot, GodotConvert, GodotFfiVariant, GodotType, ToGodot};

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
///
/// ## `#[var]` and `#[export]`
/// `AnyDictionary` intentionally does not implement `Var` or `Export` traits, so you cannot use it in properties. GDScript and the editor would
/// treat this type as untyped `Dictionary`, which would break type safety if the dictionary is typed at runtime. Instead, use `VarDictionary`
/// or `Dictionary<K, V>` directly.
#[derive(PartialEq)]
#[repr(transparent)] // Guarantees same layout as VarDictionary, enabling Deref from Dictionary<K, V> (K/V have no influence on layout).
pub struct AnyDictionary {
    pub(crate) dict: VarDictionary,
}

impl AnyDictionary {
    pub(super) fn from_typed_or_untyped<K: Element, V: Element>(dict: Dictionary<K, V>) -> Self {
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
    pub fn at(&self, key: impl AsArg<Variant>) -> Variant {
        self.dict.at(key)
    }

    /// Returns the value for the given key, or `None`.
    ///
    /// Note that `NIL` values are returned as `Some(Variant::nil())`, while absent values are returned as `None`.
    pub fn get(&self, key: impl AsArg<Variant>) -> Option<Variant> {
        self.dict.get(key)
    }

    /// Returns `true` if the dictionary contains the given key.
    ///
    /// _Godot equivalent: `has`_
    #[doc(alias = "has")]
    pub fn contains_key(&self, key: impl AsArg<Variant>) -> bool {
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
    pub fn find_key_by_value(&self, value: impl AsArg<Variant>) -> Option<Variant> {
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
    pub fn remove(&mut self, key: impl AsArg<Variant>) -> Option<Variant> {
        self.dict.remove(key)
    }

    /// Alias for [`remove()`][Self::remove].
    ///
    /// _Godot equivalent: `erase`_
    pub fn erase(&mut self, key: impl AsArg<Variant>) -> Option<Variant> {
        self.remove(key)
    }

    /// Creates a new `Array` containing all the keys currently in the dictionary.
    ///
    /// _Godot equivalent: `keys`_
    #[doc(alias = "keys")]
    pub fn keys_array(&self) -> AnyArray {
        // Array can still be typed; so AnyArray is the only sound return type.
        // Do not use dict.keys_array() which assumes Variant typing.
        self.dict.as_inner().keys()
    }

    /// Creates a new `Array` containing all the values currently in the dictionary.
    ///
    /// _Godot equivalent: `values`_
    #[doc(alias = "values")]
    pub fn values_array(&self) -> AnyArray {
        // Array can still be typed; so AnyArray is the only sound return type.
        // Do not use dict.values_array() which assumes Variant typing.
        self.dict.as_inner().values()
    }

    /// Returns a shallow copy, sharing reference types (`Array`, `Dictionary`, `Object`...) with the original dictionary.
    ///
    /// This operation retains the dynamic key/value types: copying `Dictionary<K, V>` will yield another `Dictionary<K, V>`.
    ///
    /// To create a deep copy, use [`duplicate_deep()`][Self::duplicate_deep] instead.
    /// To create a new reference to the same dictionary data, use [`clone()`][Clone::clone].
    pub fn duplicate_shallow(&self) -> AnyDictionary {
        self.dict.as_inner().duplicate(false)
    }

    /// Returns a deep copy, duplicating nested `Array`/`Dictionary` elements but keeping `Object` elements shared.
    ///
    /// This operation retains the dynamic key/value types: copying `Dictionary<K, V>` will yield another `Dictionary<K, V>`.
    ///
    /// To create a shallow copy, use [`duplicate_shallow()`][Self::duplicate_shallow] instead.
    /// To create a new reference to the same dictionary data, use [`clone()`][Clone::clone].
    pub fn duplicate_deep(&self) -> Self {
        self.dict.as_inner().duplicate(true)
    }

    /// Returns an iterator over the key-value pairs as `(Variant, Variant)`.
    ///
    /// Each pair is a (cheap, shallow) copy from the original dictionary.
    ///
    /// Use `.typed::<K, V>()` on the returned iterator to convert to typed `(K, V)` pairs.
    ///
    /// Note that it's possible to modify the dictionary through another reference while iterating over it. This will not result in
    /// unsoundness or crashes, but will cause the iterator to behave in an unspecified way.
    pub fn iter_shared(&self) -> AnyDictIter<'_> {
        AnyDictIter {
            inner: super::dictionary::DictIter::new(self),
        }
    }

    /// Returns an iterator over the keys as `Variant`.
    ///
    /// Each key is a (cheap, shallow) copy from the original dictionary.
    ///
    /// Use `.typed::<K>()` on the returned iterator to convert to typed `K` keys.
    ///
    /// Note that it's possible to modify the dictionary through another reference while iterating over it. This will not result in
    /// unsoundness or crashes, but will cause the iterator to behave in an unspecified way.
    pub fn keys_shared(&self) -> AnyDictKeys<'_> {
        AnyDictKeys {
            inner: super::dictionary::DictKeys::new(self),
        }
    }

    /// Returns an iterator over the values as `Variant`.
    ///
    /// Each value is a (cheap, shallow) copy from the original dictionary.
    ///
    /// Use `.typed::<V>()` on the returned iterator to convert to typed `V` values.
    ///
    /// Note that it's possible to modify the dictionary through another reference while iterating over it. This will not result in
    /// unsoundness or crashes, but will cause the iterator to behave in an unspecified way.
    pub fn values_shared(&self) -> AnyDictValues<'_> {
        AnyDictValues {
            inner: super::dictionary::DictValues::new(self),
        }
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
    ///
    /// # Compatibility
    /// Always reflects the **runtime** engine-side type. Different behavior based on Godot runtime version (not the `api-4-*` flag):
    /// * **Since 4.4:** Returns the type information stored in the Godot engine. If this is a `Dictionary<K, V>`, returns a type representing `K`.
    /// * **Before 4.4:** Always returns [`ElementType::Untyped`], as the engine does not support typed dictionaries.
    pub fn key_element_type(&self) -> ElementType {
        #[cfg(since_api = "4.4")]
        {
            ElementType::get_or_compute_cached(
                &self.dict.cached_key_type,
                || self.dict.as_inner().get_typed_key_builtin(),
                || self.dict.as_inner().get_typed_key_class_name(),
                || self.dict.as_inner().get_typed_key_script(),
            )
        }

        #[cfg(before_api = "4.4")]
        Self::polyfill_element_type(self.to_variant(), &self.dict.cached_key_type, "key")
    }

    /// Returns the runtime element type information for values in this dictionary.
    ///
    /// The result is generally cached, so feel free to call this method repeatedly.
    ///
    /// # Compatibility
    /// Always reflects the **runtime** engine-side type. Different behavior based on Godot runtime version (not the `api-4-*` flag):
    /// * **Since 4.4:** Returns the type information stored in the Godot engine. If this is a `Dictionary<K, V>`, returns a type representing `V`.
    /// * **Before 4.4:** Always returns [`ElementType::Untyped`], as the engine does not support typed dictionaries.
    pub fn value_element_type(&self) -> ElementType {
        #[cfg(since_api = "4.4")]
        {
            ElementType::get_or_compute_cached(
                &self.dict.cached_value_type,
                || self.dict.as_inner().get_typed_value_builtin(),
                || self.dict.as_inner().get_typed_value_class_name(),
                || self.dict.as_inner().get_typed_value_script(),
            )
        }

        #[cfg(before_api = "4.4")]
        Self::polyfill_element_type(self.to_variant(), &self.dict.cached_value_type, "value")
    }

    /// Polyfill for `key_element_type()`/`value_element_type()` when compiled against pre-4.4 API.
    ///
    /// Uses a runtime check: if running on 4.4+, queries the engine via dynamic `Variant::call()`.
    /// Otherwise returns `Untyped`, since the engine does not support typed dictionaries.
    #[cfg(before_api = "4.4")]
    fn polyfill_element_type(
        dict_var: Variant,
        cache: &std::cell::OnceCell<ElementType>,
        what: &str,
    ) -> ElementType {
        // Pre-4.4 runtime: typed dicts not supported -> always untyped.
        if sys::GdextBuild::before_api("4.4") {
            // For dicts created via Dictionary::new(), Dictionary<K,V>::init_inner_type() already cached this.
            // For dicts received from GDScript, we cache it here on first query.
            return cache.get_or_init(|| ElementType::Untyped).clone();
        }

        // Running on 4.4+ binary: use dynamic calls via Variant API.
        let builtin_method = format!("get_typed_{what}_builtin");
        let class_name_method = format!("get_typed_{what}_class_name");
        let script_method = format!("get_typed_{what}_script");

        let get_typed_builtin = || {
            dict_var
                .call(&builtin_method, &[])
                .try_to::<i64>()
                .unwrap_or_else(|_| panic!("{builtin_method} returned non-integer"))
        };
        let get_typed_class_name = || {
            dict_var
                .call(&class_name_method, &[])
                .try_to::<StringName>()
                .unwrap_or_else(|_| panic!("{class_name_method} returned non-StringName"))
        };
        let get_typed_script = || dict_var.call(&script_method, &[]);

        ElementType::get_or_compute_cached(
            cache,
            get_typed_builtin,
            get_typed_class_name,
            get_typed_script,
        )
    }

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
    pub fn try_cast_dictionary<K: Element, V: Element>(self) -> Result<Dictionary<K, V>, Self> {
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

    /// Converts to `Dictionary<K, V>` if the runtime types match, panics otherwise.
    ///
    /// This is a convenience method that panics with a descriptive message if the cast fails.
    /// Use [`try_cast_dictionary()`][Self::try_cast_dictionary] for a non-panicking version.
    ///
    /// # Panics
    /// If the dictionary's dynamic key or value types do not match `K` and `V`.
    pub fn cast_dictionary<K: Element, V: Element>(self) -> Dictionary<K, V> {
        let from_key = self.key_element_type();
        let from_value = self.value_element_type();
        self.try_cast_dictionary::<K, V>().unwrap_or_else(|_| {
            panic!(
                "cast_dictionary_or_panic: expected key type {:?} and value type {:?}, got {:?} and {:?}",
                ElementType::of::<K>(),
                ElementType::of::<V>(),
                from_key,
                from_value,
            )
        })
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

    // Constructs a valid Godot dictionary as ptrcall destination, without caching element types.
    // See Array::new_with_init for rationale.
    unsafe fn new_with_init(init_fn: impl FnOnce(sys::GDExtensionTypePtr)) -> Self {
        let dict = VarDictionary::new_uncached_type(init_fn);

        Self { dict }
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

impl Element for AnyDictionary {}

impl GodotConvert for AnyDictionary {
    type Via = Self;

    fn godot_shape() -> GodotShape {
        GodotShape::of_builtin::<Self>()
    }
}

impl ToGodot for AnyDictionary {
    type Pass = meta::ByValue;

    fn to_godot(&self) -> meta::ToArg<'_, Self::Via, Self::Pass> {
        self.clone()
    }

    fn to_variant(&self) -> Variant {
        self.rust_to_variant()
    }
}

impl meta::FromGodot for AnyDictionary {
    fn try_from_godot(via: Self::Via) -> Result<Self, ConvertError> {
        Ok(via)
    }
}

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
}

impl GodotFfiVariant for AnyDictionary {
    fn rust_to_variant(&self) -> Variant {
        VarDictionary::rust_to_variant(&self.dict)
    }

    fn rust_from_variant(variant: &Variant) -> Result<Self, ConvertError> {
        // SAFETY: All element types are valid for AnyDictionary.
        let result = unsafe { VarDictionary::unchecked_from_variant(variant) };
        result.map(|inner| Self { dict: inner })
    }
}

impl<'a> IntoIterator for &'a AnyDictionary {
    type Item = (Variant, Variant);
    type IntoIter = AnyDictIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter_shared()
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// AnyDictionary iterator newtypes

/// Iterator over key-value pairs in an [`AnyDictionary`], yielding `(Variant, Variant)`.
///
/// Use [`.typed::<K, V>()`][Self::typed] to convert to a typed iterator.
pub struct AnyDictIter<'a> {
    inner: super::dictionary::DictIter<'a, Variant, Variant>,
}

impl<'a> AnyDictIter<'a> {
    /// Creates a typed iterator that converts each `(Variant, Variant)` pair into `(K, V)`,
    /// panicking upon conversion failure.
    pub fn typed<K: FromGodot, V: FromGodot>(self) -> super::dictionary::DictIter<'a, K, V> {
        self.inner.typed()
    }
}

impl Iterator for AnyDictIter<'_> {
    type Item = (Variant, Variant);

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

/// Iterator over keys in an [`AnyDictionary`], yielding `Variant`.
///
/// Use [`.typed::<K>()`][Self::typed] to convert to a typed iterator.
pub struct AnyDictKeys<'a> {
    inner: super::dictionary::DictKeys<'a, Variant>,
}

impl<'a> AnyDictKeys<'a> {
    /// Creates a typed iterator that converts each `Variant` key into `K`,
    /// panicking upon conversion failure.
    pub fn typed<K: FromGodot>(self) -> super::dictionary::DictKeys<'a, K> {
        self.inner.typed()
    }

    /// Returns an array of the keys.
    pub fn array(self) -> AnyArray {
        self.inner.array()
    }
}

impl Iterator for AnyDictKeys<'_> {
    type Item = Variant;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

/// Iterator over values in an [`AnyDictionary`], yielding `Variant`.
///
/// Use [`.typed::<V>()`][Self::typed] to convert to a typed iterator.
pub struct AnyDictValues<'a> {
    inner: super::dictionary::DictValues<'a, Variant>,
}

impl<'a> AnyDictValues<'a> {
    /// Creates a typed iterator that converts each `Variant` value into `V`,
    /// panicking upon conversion failure.
    pub fn typed<V: FromGodot>(self) -> super::dictionary::DictValues<'a, V> {
        self.inner.typed()
    }
}

impl Iterator for AnyDictValues<'_> {
    type Item = Variant;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}
