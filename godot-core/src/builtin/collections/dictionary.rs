/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::cell::OnceCell;
use std::marker::PhantomData;
use std::{fmt, ptr};

use godot_ffi as sys;
use sys::types::OpaqueDictionary;
use sys::{GodotFfi, ffi_methods, interface_fn};

use super::any_dictionary::AnyDictionary;
use crate::builtin::{AnyArray, Array, VarArray, Variant, VariantType, inner};
use crate::meta;
use crate::meta::{AsVArg, Element, ElementType, ExtVariantType, FromGodot, ToGodot};

/// Godot's `Dictionary` type.
///
/// Ordered associative hash-table, mapping keys to values. Corresponds to GDScript type `Dictionary[K, V]`.
///
/// `Dictionary<K, V>` can only hold keys of type `K` and values of type `V` (except for Godot < 4.4, see below).
/// The key type `K` and value type `V` can be anything implementing the [`Element`] trait.
/// Untyped dictionaries are represented as `Dictionary<Variant, Variant>`, which is aliased as [`VarDictionary`].
///
/// Check out the [book](https://godot-rust.github.io/book/godot-api/builtins.html#arrays-and-dictionaries) for a tutorial on dictionaries.
///
/// # Untyped example
/// ```no_run
/// # use godot::prelude::*;
/// // Create untyped dictionary and add key-values pairs.
/// let mut dict = VarDictionary::new();
/// dict.set("str", "Hello");
/// dict.set("num", 23);
///
/// // For untyped dictionaries, keys don't need to be strings.
/// let coord = Vector2i::new(0, 1);
/// dict.set(coord, "Tile77");
///
/// // Or create the same dictionary in a single expression.
/// let dict = vdict! {
///    "str": "Hello",
///    "num": 23,
///    coord: "Tile77",
/// };
///
/// // Access elements.
/// let value: Variant = dict.at("str");
/// let value: GString = dict.at("str").to(); // Variant::to() extracts GString.
/// let maybe: Option<Variant> = dict.get("absent_key");
///
/// // Iterate over key-value pairs as (K, V) -- here (Variant, Variant).
/// for (key, value) in dict.iter_shared() {
///     println!("{key} => {value}");
/// }
///
/// // Clone dictionary (shares the reference), and overwrite elements through clone.
/// let mut cloned = dict.clone();
/// cloned.remove("num");
///
/// // Overwrite with set(); use insert() to get the previous value.
/// let prev = cloned.insert("str", "Goodbye"); // prev == Some("Hello")
///
/// // Changes will be reflected in the original dictionary.
/// assert_eq!(dict.at("str"), "Goodbye".to_variant());
/// assert_eq!(dict.get("num"), None);
/// ```
///
// TODO(v0.5): support enums -- https://github.com/godot-rust/gdext/issues/353.
// # Typed example
// ```no_run
// # use godot::prelude::*;
//
// // Define a Godot-exported enum.
// #[derive(GodotConvert)]
// #[godot(via = GString)]
// enum Tile { GRASS, ROCK, WATER }
//
// let mut tiles = Dictionary::<Vector2i, Tile>::new();
// tiles.set(Vector2i::new(1, 2), Tile::GRASS);
// tiles.set(Vector2i::new(1, 3), Tile::WATER);
//
// // Create the same dictionary in a single expression.
// let tiles = dict! {
//    (Vector2i::new(1, 2)): Tile::GRASS,
//    (Vector2i::new(1, 3)): Tile::WATER,
// };
//
// // Element access is now strongly typed.
// let value = dict.at(Vector2i::new(1, 3)); // type Tile.
// ```
///
/// # Compatibility
/// **Godot 4.4+**: Dictionaries are fully typed at compile time and runtime. Type information is enforced by GDScript
/// and visible in the editor.
///
/// **Before Godot 4.4**: Type safety is enforced only on the Rust side. GDScript sees all dictionaries as untyped, and type information is not
/// available in the editor. When assigning dictionaries from GDScript to typed Rust ones, panics may occur on access if the type is incorrect.
/// For more defensive code, `VarDictionary` is recommended.
///
/// # Thread safety
/// The same principles apply as for [`crate::builtin::Array`]. Consult its documentation for details.
///
/// # Godot docs
/// [`Dictionary` (stable)](https://docs.godotengine.org/en/stable/classes/class_dictionary.html)
pub struct Dictionary<K: Element, V: Element> {
    opaque: OpaqueDictionary,
    _phantom: PhantomData<(K, V)>,

    /// Lazily computed and cached element type information for the key type.
    cached_key_type: OnceCell<ElementType>,

    /// Lazily computed and cached element type information for the value type.
    cached_value_type: OnceCell<ElementType>,
}

/// Untyped Godot `Dictionary`.
///
/// Alias for `Dictionary<Variant, Variant>`. This provides an untyped dictionary that can store any key-value pairs.
/// Available on all Godot versions.
pub type VarDictionary = Dictionary<Variant, Variant>;

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Implementation

impl<K: Element, V: Element> Dictionary<K, V> {
    pub(super) fn from_opaque(opaque: OpaqueDictionary) -> Self {
        Self {
            opaque,
            _phantom: PhantomData,
            cached_key_type: OnceCell::new(),
            cached_value_type: OnceCell::new(),
        }
    }

    /// Creates a new dictionary for [`GodotFfi::new_with_init()`], without setting a type yet.
    pub(super) fn new_uncached_type(init_fn: impl FnOnce(sys::GDExtensionTypePtr)) -> Self {
        let mut result = unsafe {
            Self::new_with_uninit(|self_ptr| {
                let ctor = sys::builtin_fn!(dictionary_construct_default);
                ctor(self_ptr, std::ptr::null_mut());
            })
        };
        init_fn(result.sys_mut());
        result
    }

    /// Constructs an empty typed `Dictionary`.
    pub fn new() -> Self {
        let mut dict = Self::default();
        dict.init_inner_type();
        dict
    }

    /// ⚠️ Returns the value for the given key, or panics.
    ///
    /// If you want to check for presence, use [`get()`][Self::get]. For `V=Variant`, you can additionally use [`get_or_nil()`][Self::get_or_nil].
    ///
    /// # Panics
    /// If there is no value for the given key. Note that this is distinct from a `NIL` value, which is returned as `Variant::nil()`.
    pub fn at(&self, key: impl AsVArg<K>) -> V {
        meta::varg_into_ref!(key: K);
        let key_variant = key.to_variant();
        if self.as_inner().has(&key_variant) {
            self.get_or_panic(key_variant)
        } else {
            panic!("key {key_variant:?} missing in dictionary: {self:?}")
        }
    }

    /// Returns the value for the given key, or `None`.
    ///
    /// Note that `NIL` values are returned as `Some(V::from_variant(...))`, while absent values are returned as `None`.
    /// If you want to treat both as `NIL`, use [`get_or_nil()`][Self::get_or_nil].
    ///
    /// When you are certain that a key is present, use [`at()`][`Self::at`] instead.
    ///
    /// This can be combined with Rust's `Option` methods, e.g. `dict.get(key).unwrap_or(default)`.
    pub fn get(&self, key: impl AsVArg<K>) -> Option<V> {
        meta::varg_into_ref!(key: K);
        let key_variant = key.to_variant();
        if self.as_inner().has(&key_variant) {
            Some(self.get_or_panic(key_variant))
        } else {
            None
        }
    }

    /// Returns the value at the key, converted to `V`. Panics on conversion failure.
    fn get_or_panic(&self, key: Variant) -> V {
        V::from_variant(&self.as_inner().get(&key, &Variant::nil()))
    }

    // TODO(v0.5): avoid double FFI round-trip (has + get); consider using get(key, sentinel) pattern.
    /// Gets and removes the old value for a key, if it exists.
    fn take_old_value(&self, key_variant: &Variant) -> Option<V> {
        self.as_inner()
            .has(key_variant)
            .then(|| self.get_or_panic(key_variant.clone()))
    }

    /// Gets a value and ensures the key is set, inserting default if key is absent.
    ///
    /// If the `key` exists in the dictionary, this behaves like [`get()`][Self::get], and the existing value is returned.
    /// Otherwise, the `default` value is inserted and returned.
    ///
    /// _Godot equivalent: `get_or_add`_
    #[doc(alias = "get_or_add")]
    pub fn get_or_insert(&mut self, key: impl AsVArg<K>, default: impl AsVArg<V>) -> V {
        self.balanced_ensure_mutable();

        meta::varg_into_ref!(key: K);
        meta::varg_into_ref!(default: V);

        let key_variant = key.to_variant();

        // Godot 4.3+: delegate to native get_or_add().
        #[cfg(since_api = "4.3")]
        {
            let default_variant = default.to_variant();

            let result = self.as_inner().get_or_add(&key_variant, &default_variant);
            V::from_variant(&result)
        }

        // Polyfill for Godot versions before 4.3.
        #[cfg(before_api = "4.3")]
        {
            if self.as_inner().has(&key_variant) {
                self.get_or_panic(key_variant)
            } else {
                let default_variant = default.to_variant();

                // SAFETY: K and V strongly typed.
                unsafe { self.set_variant(key_variant, default_variant.clone()) };

                // Variant roundtrip to avoid V: Clone bound. Inefficient but old Godot version.
                V::from_variant(&default_variant)
            }
        }
    }

    /// Returns `true` if the dictionary contains the given key.
    ///
    /// _Godot equivalent: `has`_
    #[doc(alias = "has")]
    pub fn contains_key(&self, key: impl AsVArg<K>) -> bool {
        meta::varg_into_ref!(key: K);
        let key = key.to_variant();
        self.as_inner().has(&key)
    }

    /// Returns `true` if the dictionary contains all the given keys.
    ///
    /// _Godot equivalent: `has_all`_
    #[doc(alias = "has_all")]
    pub fn contains_all_keys(&self, keys: &VarArray) -> bool {
        self.as_inner().has_all(keys)
    }

    /// Returns the number of entries in the dictionary.
    ///
    /// _Godot equivalent: `size`_
    #[doc(alias = "size")]
    pub fn len(&self) -> usize {
        self.as_inner().size().try_into().unwrap()
    }

    /// Returns true if the dictionary is empty.
    pub fn is_empty(&self) -> bool {
        self.as_inner().is_empty()
    }

    /// Reverse-search a key by its value.
    ///
    /// Unlike Godot, this will return `None` if the key does not exist and `Some(key)` if found.
    ///
    /// This operation is rarely needed and very inefficient. If you find yourself needing it a lot, consider
    /// using a `HashMap` or `Dictionary` with the inverse mapping (`V` -> `K`).
    ///
    /// _Godot equivalent: `find_key`_
    #[doc(alias = "find_key")]
    pub fn find_key_by_value(&self, value: impl AsVArg<V>) -> Option<K>
    where
        K: FromGodot,
    {
        meta::varg_into_ref!(value: V);
        let key = self.as_inner().find_key(&value.to_variant());

        if !key.is_nil() || self.as_inner().has(&key) {
            Some(K::from_variant(&key))
        } else {
            None
        }
    }

    /// Removes all key-value pairs from the dictionary.
    pub fn clear(&mut self) {
        self.balanced_ensure_mutable();
        self.as_inner().clear()
    }

    /// Set a key to a given value.
    ///
    /// If you are interested in the previous value, use [`insert()`][Self::insert] instead.
    ///
    /// For `VarDictionary` (or partially-typed dictionaries with `Variant` key/value), this method
    /// accepts any `impl ToGodot` for the Variant positions, thanks to blanket `AsVArg<Variant>` impls.
    ///
    /// _Godot equivalent: `dict[key] = value`_
    pub fn set(&mut self, key: impl AsVArg<K>, value: impl AsVArg<V>) {
        self.balanced_ensure_mutable();

        meta::varg_into_ref!(key: K);
        meta::varg_into_ref!(value: V);

        // SAFETY: K and V strongly typed.
        unsafe { self.set_variant(key.to_variant(), value.to_variant()) };
    }

    /// Insert a value at the given key, returning the previous value for that key (if available).
    ///
    /// If you don't need the previous value, use [`set()`][Self::set] instead.
    #[must_use]
    pub fn insert(&mut self, key: impl AsVArg<K>, value: impl AsVArg<V>) -> Option<V> {
        self.balanced_ensure_mutable();

        meta::varg_into_ref!(key: K);
        meta::varg_into_ref!(value: V);

        let key_variant = key.to_variant();
        let old_value = self.take_old_value(&key_variant);

        // SAFETY: K and V strongly typed.
        unsafe { self.set_variant(key_variant, value.to_variant()) };

        old_value
    }

    /// Removes a key from the map, and returns the value associated with
    /// the key if the key was in the dictionary.
    ///
    /// _Godot equivalent: `erase`_
    #[doc(alias = "erase")]
    pub fn remove(&mut self, key: impl AsVArg<K>) -> Option<V> {
        self.balanced_ensure_mutable();

        meta::varg_into_ref!(key: K);

        let key_variant = key.to_variant();
        let old_value = self.take_old_value(&key_variant);
        self.as_inner().erase(&key_variant);
        old_value
    }

    crate::declare_hash_u32_method! {
        /// Returns a 32-bit integer hash value representing the dictionary and its contents.
    }

    /// Creates a new `Array` containing all the keys currently in the dictionary.
    ///
    /// _Godot equivalent: `keys`_
    #[doc(alias = "keys")]
    pub fn keys_array(&self) -> Array<K> {
        self.as_inner().keys().cast_array()
    }

    /// Creates a new `Array` containing all the values currently in the dictionary.
    ///
    /// _Godot equivalent: `values`_
    #[doc(alias = "values")]
    pub fn values_array(&self) -> Array<V> {
        self.as_inner().values().cast_array()
    }

    /// Copies all keys and values from `other` into `self`.
    ///
    /// If `overwrite` is true, it will overwrite pre-existing keys.
    ///
    /// _Godot equivalent: `merge`_
    #[doc(alias = "merge")]
    pub fn extend_dictionary(&mut self, other: &Self, overwrite: bool) {
        self.balanced_ensure_mutable();
        self.as_inner().merge(other, overwrite)
    }

    /// Deep copy, duplicating nested collections.
    ///
    /// All nested arrays and dictionaries are duplicated and will not be shared with the original dictionary.
    /// Note that any `Object`-derived elements will still be shallow copied.
    ///
    /// To create a shallow copy, use [`Self::duplicate_shallow()`] instead.
    /// To create a new reference to the same dictionary data, use [`clone()`][Clone::clone].
    ///
    /// _Godot equivalent: `dict.duplicate(true)`_
    pub fn duplicate_deep(&self) -> Self {
        self.as_inner().duplicate(true).cast_dictionary::<K, V>()
    }

    /// Shallow copy, copying elements but sharing nested collections.
    ///
    /// All dictionary keys and values are copied, but any reference types (such as `Array`, `Dictionary` and `Gd<T>` objects)
    /// will still refer to the same value.
    ///
    /// To create a deep copy, use [`Self::duplicate_deep()`] instead.
    /// To create a new reference to the same dictionary data, use [`clone()`][Clone::clone].
    ///
    /// _Godot equivalent: `dict.duplicate(false)`_
    pub fn duplicate_shallow(&self) -> Self {
        self.as_inner().duplicate(false).cast_dictionary::<K, V>()
    }

    /// Returns an iterator over the key-value pairs of the `Dictionary`.
    ///
    /// The pairs are each of type `(Variant, Variant)`. Each pair references the original dictionary, but instead of a `&`-reference
    /// to key-value pairs as you might expect, the iterator returns a (cheap, shallow) copy of each key-value pair.
    ///
    /// Note that it's possible to modify the dictionary through another reference while iterating over it. This will not result in
    /// unsoundness or crashes, but will cause the iterator to behave in an unspecified way.
    ///
    /// Use `dict.iter_shared().typed::<K, V>()` to iterate over `(K, V)` pairs instead.
    pub fn iter_shared(&self) -> Iter<'_> {
        Iter::new(self)
    }

    /// Returns an iterator over the keys in a `Dictionary`.
    ///
    /// The keys are each of type `Variant`. Each key references the original `Dictionary`, but instead of a `&`-reference to keys pairs
    /// as you might expect, the iterator returns a (cheap, shallow) copy of each key pair.
    ///
    /// Note that it's possible to modify the `Dictionary` through another reference while iterating over it. This will not result in
    /// unsoundness or crashes, but will cause the iterator to behave in an unspecified way.
    ///
    /// Use `dict.keys_shared().typed::<K>()` to iterate over `K` keys instead.
    pub fn keys_shared(&self) -> Keys<'_> {
        Keys::new(self)
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
        self.as_inner().make_read_only();
        self
    }

    /// Returns true if the dictionary is read-only.
    ///
    /// See [`into_read_only()`][Self::into_read_only].
    /// In GDScript, dictionaries are automatically read-only if declared with the `const` keyword.
    pub fn is_read_only(&self) -> bool {
        self.as_inner().is_read_only()
    }

    /// Converts this typed `Dictionary<K, V>` into an `AnyDictionary`.
    ///
    /// Typically, you can use deref coercion to convert `&Dictionary<K, V>` to `&AnyDictionary`.
    /// This method is useful if you need `AnyDictionary` by value.
    /// It consumes `self` to avoid incrementing the reference count; use `clone()` if you use the original dictionary further.
    pub fn upcast_any_dictionary(self) -> AnyDictionary {
        AnyDictionary::from_typed_or_untyped(self)
    }

    /// Best-effort mutability check.
    ///
    /// # Panics (safeguards-balanced)
    /// If the dictionary is marked as read-only.
    fn balanced_ensure_mutable(&self) {
        sys::balanced_assert!(
            !self.is_read_only(),
            "mutating operation on read-only dictionary"
        );
    }

    /// Returns the runtime element type information for keys in this dictionary.
    ///
    /// # Compatibility
    ///
    /// **Godot 4.4+**: Returns the type information stored in the Godot engine.
    ///
    /// **Before Godot 4.4**: Returns the Rust-side compile-time type `K` as `ElementType::Untyped` for `Variant`,
    /// or the appropriate typed `ElementType` for other types. Since typed dictionaries are not supported by the
    /// engine before 4.4, all dictionaries appear untyped to Godot regardless of this value.
    pub fn key_element_type(&self) -> ElementType {
        #[cfg(since_api = "4.4")]
        {
            ElementType::get_or_compute_cached(
                &self.cached_key_type,
                || self.as_inner().get_typed_key_builtin(),
                || self.as_inner().get_typed_key_class_name(),
                || self.as_inner().get_typed_key_script(),
            )
        }

        #[cfg(before_api = "4.4")]
        {
            // Return Rust's compile-time type info (cached).
            self.cached_key_type
                .get_or_init(|| ElementType::of::<K>())
                .clone()
        }
    }

    /// Returns the runtime element type information for values in this dictionary.
    ///
    /// # Compatibility
    ///
    /// **Godot 4.4+**: Returns the type information stored in the Godot engine.
    ///
    /// **Before Godot 4.4**: Returns the Rust-side compile-time type `V` as `ElementType::Untyped` for `Variant`,
    /// or the appropriate typed `ElementType` for other types. Since typed dictionaries are not supported by the
    /// engine before 4.4, all dictionaries appear untyped to Godot regardless of this value.
    pub fn value_element_type(&self) -> ElementType {
        #[cfg(since_api = "4.4")]
        {
            ElementType::get_or_compute_cached(
                &self.cached_value_type,
                || self.as_inner().get_typed_value_builtin(),
                || self.as_inner().get_typed_value_class_name(),
                || self.as_inner().get_typed_value_script(),
            )
        }

        #[cfg(before_api = "4.4")]
        {
            // Return Rust's compile-time type info (cached).
            self.cached_value_type
                .get_or_init(|| ElementType::of::<V>())
                .clone()
        }
    }

    #[doc(hidden)]
    pub fn as_inner(&self) -> inner::InnerDictionary<'_> {
        inner::InnerDictionary::from_outer_typed(self)
    }

    /// Get the pointer corresponding to the given key in the dictionary.
    ///
    /// If there exists no value at the given key, a `NIL` variant will be inserted for that key.
    fn get_ptr_mut(&mut self, key: Variant) -> sys::GDExtensionVariantPtr {
        // Never a null pointer, since entry either existed already or was inserted above.
        // SAFETY: accessing an unknown key _mutably_ creates that entry in the dictionary, with value `NIL`.
        unsafe { interface_fn!(dictionary_operator_index)(self.sys_mut(), key.var_sys()) }
    }

    /// Sets a key-value pair at the variant level.
    ///
    /// # Safety
    /// `key` must hold type `K` and `value` must hold type `V`.
    unsafe fn set_variant(&mut self, key: Variant, value: Variant) {
        let ptr = self.get_ptr_mut(key);

        // SAFETY: `get_ptr_mut` always returns a valid pointer (creates entry if key is absent).
        unsafe { value.move_into_var_ptr(ptr) };
    }

    /// Execute a function that creates a new dictionary, transferring cached element types if available.
    fn with_cache(self, source: &Self) -> Self {
        ElementType::transfer_cache(&source.cached_key_type, &self.cached_key_type);
        ElementType::transfer_cache(&source.cached_value_type, &self.cached_value_type);
        self
    }

    /// Checks that the inner dictionary has the correct types set for storing keys of type `K` and values of type `V`.
    ///
    /// Only performs runtime checks on Godot 4.4+, where typed dictionaries are supported by the engine.
    /// Before 4.4, this always succeeds since there are no engine-side types to check against.
    #[cfg(since_api = "4.4")]
    fn with_checked_type(self) -> Result<Self, meta::error::ConvertError> {
        use crate::meta::error::{DictionaryMismatch, FromGodotError};

        let actual_key = self.key_element_type();
        let actual_value = self.value_element_type();
        let expected_key = ElementType::of::<K>();
        let expected_value = ElementType::of::<V>();

        if actual_key.is_compatible_with(&expected_key)
            && actual_value.is_compatible_with(&expected_value)
        {
            Ok(self)
        } else {
            let mismatch = DictionaryMismatch {
                expected_key,
                expected_value,
                actual_key,
                actual_value,
            };
            Err(FromGodotError::BadDictionaryType(mismatch).into_error(self))
        }
    }

    fn as_any_ref(&self) -> &AnyDictionary {
        // SAFETY:
        // - Dictionary<K, V> and VarDictionary have identical memory layout.
        // - AnyDictionary provides no "in" operations (moving data in) that could violate covariance.
        unsafe { std::mem::transmute::<&Dictionary<K, V>, &AnyDictionary>(self) }
    }

    fn as_any_mut(&mut self) -> &mut AnyDictionary {
        // SAFETY:
        // - Dictionary<K, V> and VarDictionary have identical memory layout.
        // - AnyDictionary is #[repr(transparent)] around VarDictionary.
        // - Mutable operations on AnyDictionary work with Variant values, maintaining type safety through balanced_ensure_mutable() checks.
        unsafe { std::mem::transmute::<&mut Dictionary<K, V>, &mut AnyDictionary>(self) }
    }

    /// # Safety
    /// Does not validate the dictionary key/value types; `with_checked_type()` should be called afterward.
    // Visibility: shared with AnyDictionary.
    pub(super) unsafe fn unchecked_from_variant(
        variant: &Variant,
    ) -> Result<Self, meta::error::ConvertError> {
        use crate::builtin::VariantType;
        use crate::meta::error::FromVariantError;

        let variant_type = variant.get_type();
        if variant_type != VariantType::DICTIONARY {
            return Err(FromVariantError::BadType {
                expected: VariantType::DICTIONARY,
                actual: variant_type,
            }
            .into_error(variant.clone()));
        }

        let result = unsafe {
            Self::new_with_uninit(|self_ptr| {
                let converter = sys::builtin_fn!(dictionary_from_variant);
                converter(self_ptr, sys::SysPtr::force_mut(variant.var_sys()));
            })
        };

        Ok(result)
    }

    /// Initialize the typed dictionary with key and value type information.
    ///
    /// On Godot 4.4+, this calls `dictionary_set_typed()` to inform the engine about types.
    /// On earlier versions, this only initializes the Rust-side type cache.
    fn init_inner_type(&mut self) {
        let key_elem_ty = ElementType::of::<K>();
        let value_elem_ty = ElementType::of::<V>();

        // Cache types on Rust side (for all versions) -- they are Copy.
        self.cached_key_type.get_or_init(|| key_elem_ty);
        self.cached_value_type.get_or_init(|| value_elem_ty);

        // If both are untyped (Variant), skip initialization.
        if !key_elem_ty.is_typed() && !value_elem_ty.is_typed() {
            return;
        }

        // Godot 4.4+: Set type information in the engine.
        #[cfg(since_api = "4.4")]
        {
            // Script is always nil for compile-time types (only relevant for GDScript class_name types).
            let script = Variant::nil();

            let empty_string_name = crate::builtin::StringName::default();
            let key_class_name = key_elem_ty.class_name_sys_or(&empty_string_name);
            let value_class_name = value_elem_ty.class_name_sys_or(&empty_string_name);

            // SAFETY: Valid pointers are passed in.
            // Relevant for correctness, not safety: the dictionary is a newly created, empty, untyped dictionary.
            unsafe {
                interface_fn!(dictionary_set_typed)(
                    self.sys_mut(),
                    key_elem_ty.variant_type().sys(),
                    key_class_name,
                    script.var_sys(),
                    value_elem_ty.variant_type().sys(),
                    value_class_name,
                    script.var_sys(),
                );
            }
        }

        // Before Godot 4.4: No engine-side typing, only Rust-side (already cached above).
        #[cfg(before_api = "4.4")]
        {
            // Types are already cached at the beginning of this function.
            // No additional work needed - Rust-only type safety.
        }
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// V=Variant specialization

impl<K: Element> Dictionary<K, Variant> {
    /// Returns the value for the given key, or `Variant::nil()` if the key is absent.
    ///
    /// This does _not_ distinguish between absent keys and keys mapped to `NIL` -- both return `Variant::nil()`.
    /// Use [`get()`][Self::get] if you need to tell them apart.
    ///
    /// _Godot equivalent: `dict.get(key)` (1-arg overload)_
    ///
    /// # `AnyDictionary`
    /// This method is deliberately absent from [`AnyDictionary`][super::AnyDictionary]. Because `Dictionary<K, V>` implements
    /// `Deref<Target = AnyDictionary>`, any method on `AnyDictionary` is inherited by _all_ dictionaries -- including typed ones
    /// like `Dictionary<K, i64>`, where a `Variant` return would be surprising.
    pub fn get_or_nil(&self, key: impl AsVArg<K>) -> Variant {
        meta::varg_into_ref!(key: K);
        let key_variant = key.to_variant();
        self.as_inner().get(&key_variant, &Variant::nil())
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Traits

unsafe impl<K: Element, V: Element> GodotFfi for Dictionary<K, V> {
    const VARIANT_TYPE: ExtVariantType = ExtVariantType::Concrete(sys::VariantType::DICTIONARY);

    ffi_methods! { type sys::GDExtensionTypePtr = *mut Opaque;
        fn new_from_sys;
        fn new_with_uninit;
        fn sys;
        fn sys_mut;
        fn from_arg_ptr;
        fn move_return_ptr;
    }

    /// Constructs a valid Godot dictionary as ptrcall destination, without caching the element types.
    ///
    /// See `Array::new_with_init` for rationale.
    unsafe fn new_with_init(init_fn: impl FnOnce(sys::GDExtensionTypePtr)) -> Self {
        Self::new_uncached_type(init_fn)
    }
}

impl<K: Element, V: Element> std::ops::Deref for Dictionary<K, V> {
    type Target = AnyDictionary;

    fn deref(&self) -> &Self::Target {
        self.as_any_ref()
    }
}

impl<K: Element, V: Element> std::ops::DerefMut for Dictionary<K, V> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.as_any_mut()
    }
}

// Compile-time validation of layout compatibility.
sys::static_assert_eq_size_align!(Dictionary<i64, bool>, VarDictionary);
sys::static_assert_eq_size_align!(Dictionary<crate::builtin::GString, f32>, VarDictionary);
sys::static_assert_eq_size_align!(VarDictionary, AnyDictionary);

impl<K: Element, V: Element> Default for Dictionary<K, V> {
    #[inline]
    fn default() -> Self {
        // Create an empty untyped dictionary first (typing happens in new()).
        unsafe {
            Self::new_with_uninit(|self_ptr| {
                let ctor = sys::builtin_fn!(dictionary_construct_default);
                ctor(self_ptr, ptr::null_mut())
            })
        }
    }
}

impl<K: Element, V: Element> Drop for Dictionary<K, V> {
    fn drop(&mut self) {
        // SAFETY: destructor is valid for self.
        unsafe { sys::builtin_fn!(dictionary_destroy)(self.sys_mut()) }
    }
}

impl<K: Element, V: Element> PartialEq for Dictionary<K, V> {
    fn eq(&self, other: &Self) -> bool {
        // SAFETY: equality check is valid.
        unsafe {
            let mut result = false;
            sys::builtin_call! {
                dictionary_operator_equal(self.sys(), other.sys(), result.sys_mut())
            }
            result
        }
    }
}

// Note: PartialOrd is intentionally NOT implemented for Dictionary.
// Unlike arrays, dictionaries do not have a natural ordering in Godot (no dictionary_operator_less).

impl<K: Element, V: Element> fmt::Debug for Dictionary<K, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.to_variant().stringify())
    }
}

impl<K: Element, V: Element> fmt::Display for Dictionary<K, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{{ ")?;
        for (count, (key, value)) in self.iter_shared().enumerate() {
            if count != 0 {
                write!(f, ", ")?;
            }
            write!(f, "{key}: {value}")?;
        }
        write!(f, " }}")
    }
}

impl<K: Element, V: Element> Clone for Dictionary<K, V> {
    fn clone(&self) -> Self {
        // SAFETY: `self` is a valid dictionary, since we have a reference that keeps it alive.
        let result = unsafe {
            Self::new_with_uninit(|self_ptr| {
                let ctor = sys::builtin_fn!(dictionary_construct_copy);
                let args = [self.sys()];
                ctor(self_ptr, args.as_ptr());
            })
        };
        result.with_cache(self)
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Conversion traits

/// Insert iterator range into dictionary.
///
/// Inserts all key-value pairs from the iterator into the dictionary. Previous values for keys appearing
/// in `iter` will be overwritten.
impl<K: Element, V: Element> Extend<(K, V)> for Dictionary<K, V> {
    fn extend<I: IntoIterator<Item = (K, V)>>(&mut self, iter: I) {
        for (k, v) in iter.into_iter() {
            // Inline set logic to avoid generic owned_into_varg() (which can't resolve T::Pass).
            self.balanced_ensure_mutable();

            // SAFETY: K and V strongly typed.
            unsafe { self.set_variant(k.to_variant(), v.to_variant()) };
        }
    }
}

/// Creates a `Dictionary` from an iterator over key-value pairs.
impl<K: Element, V: Element> FromIterator<(K, V)> for Dictionary<K, V> {
    fn from_iter<I: IntoIterator<Item = (K, V)>>(iter: I) -> Self {
        let mut dict = Dictionary::new();
        dict.extend(iter);
        dict
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// GodotConvert/ToGodot/FromGodot for Dictionary<K, V>

impl<K: Element, V: Element> meta::sealed::Sealed for Dictionary<K, V> {}

impl<K: Element, V: Element> meta::GodotConvert for Dictionary<K, V> {
    type Via = Self;
}

impl<K: Element, V: Element> ToGodot for Dictionary<K, V> {
    type Pass = meta::ByRef;

    fn to_godot(&self) -> &Self::Via {
        self
    }
}

impl<K: Element, V: Element> FromGodot for Dictionary<K, V> {
    fn try_from_godot(via: Self::Via) -> Result<Self, meta::error::ConvertError> {
        // For typed dictionaries, we should validate that the types match.
        // VarDictionary (K=V=Variant) always matches.
        Ok(via)
    }
}

impl<K: Element, V: Element> meta::GodotFfiVariant for Dictionary<K, V> {
    fn ffi_to_variant(&self) -> Variant {
        unsafe {
            Variant::new_with_var_uninit(|variant_ptr| {
                let converter = sys::builtin_fn!(dictionary_to_variant);
                converter(variant_ptr, sys::SysPtr::force_mut(self.sys()));
            })
        }
    }

    fn ffi_from_variant(variant: &Variant) -> Result<Self, meta::error::ConvertError> {
        // SAFETY: if conversion succeeds, we call with_checked_type() afterwards.
        let result = unsafe { Self::unchecked_from_variant(variant) }?;

        // On Godot 4.4+, check that the runtime types match the compile-time types.
        #[cfg(since_api = "4.4")]
        {
            result.with_checked_type()
        }

        #[cfg(before_api = "4.4")]
        Ok(result)
    }
}

impl<K: Element, V: Element> meta::GodotType for Dictionary<K, V> {
    type Ffi = Self;

    type ToFfi<'f>
        = meta::RefArg<'f, Dictionary<K, V>>
    where
        Self: 'f;

    fn to_ffi(&self) -> Self::ToFfi<'_> {
        meta::RefArg::new(self)
    }

    fn into_ffi(self) -> Self::Ffi {
        self
    }

    fn try_from_ffi(ffi: Self::Ffi) -> Result<Self, meta::error::ConvertError> {
        Ok(ffi)
    }

    fn godot_type_name() -> String {
        "Dictionary".to_string()
    }

    fn property_hint_info() -> meta::PropertyHintInfo {
        // On Godot 4.4+, typed dictionaries use DICTIONARY_TYPE hint.
        #[cfg(since_api = "4.4")]
        if is_dictionary_typed::<K, V>() {
            return meta::PropertyHintInfo::var_dictionary_element::<K, V>();
        }

        // Untyped dictionary or before 4.4: no hints.
        meta::PropertyHintInfo::none()
    }
}

impl<K: Element, V: Element> Element for Dictionary<K, V> {}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Var/Export implementations for Dictionary<K, V>

/// Check if Dictionary<K, V> is typed (at least one of K or V is not Variant).
#[inline]
fn is_dictionary_typed<K: Element, V: Element>() -> bool {
    // Nil means "untyped" or "Variant" in Godot.
    meta::element_variant_type::<K>() != VariantType::NIL
        || meta::element_variant_type::<V>() != VariantType::NIL
}

impl<K: Element, V: Element> crate::registry::property::Var for Dictionary<K, V> {
    type PubType = Self;

    fn var_get(field: &Self) -> Self::Via {
        field.clone()
    }

    fn var_set(field: &mut Self, value: Self::Via) {
        *field = value;
    }

    fn var_pub_get(field: &Self) -> Self::PubType {
        field.clone()
    }

    fn var_pub_set(field: &mut Self, value: Self::PubType) {
        *field = value;
    }

    fn var_hint() -> meta::PropertyHintInfo {
        // On Godot 4.4+, typed dictionaries use DICTIONARY_TYPE hint.
        #[cfg(since_api = "4.4")]
        if is_dictionary_typed::<K, V>() {
            return meta::PropertyHintInfo::var_dictionary_element::<K, V>();
        }

        // Untyped dictionary or before 4.4: no hints.
        meta::PropertyHintInfo::none()
    }
}

impl<K, V> crate::registry::property::Export for Dictionary<K, V>
where
    K: Element + crate::registry::property::Export,
    V: Element + crate::registry::property::Export,
{
    fn export_hint() -> meta::PropertyHintInfo {
        // VarDictionary: use "Dictionary".
        if !is_dictionary_typed::<K, V>() {
            return meta::PropertyHintInfo::type_name::<VarDictionary>();
        }

        // On Godot 4.4+, typed dictionaries use DICTIONARY_TYPE hint for export.
        #[cfg(since_api = "4.4")]
        return meta::PropertyHintInfo::export_dictionary_element::<K, V>();

        // Before 4.4, no engine-side typed dictionary hints.
        #[cfg(before_api = "4.4")]
        meta::PropertyHintInfo::none()
    }
}

impl<K: Element, V: Element> crate::registry::property::BuiltinExport for Dictionary<K, V> {}

// ----------------------------------------------------------------------------------------------------------------------------------------------

/// Internal helper for different iterator impls -- not an iterator itself.
struct DictionaryIter<'a> {
    last_key: Option<Variant>,
    dictionary: &'a AnyDictionary,
    is_first: bool,
    next_idx: usize,
}

impl<'a> DictionaryIter<'a> {
    fn new(dictionary: &'a AnyDictionary) -> Self {
        Self {
            last_key: None,
            dictionary,
            is_first: true,
            next_idx: 0,
        }
    }

    fn next_key(&mut self) -> Option<Variant> {
        let new_key = if self.is_first {
            self.is_first = false;
            Self::call_init(self.dictionary)
        } else {
            Self::call_next(self.dictionary, self.last_key.take()?)
        };

        if self.next_idx < self.dictionary.len() {
            self.next_idx += 1;
        }

        self.last_key.clone_from(&new_key);
        new_key
    }

    fn next_key_value(&mut self) -> Option<(Variant, Variant)> {
        let key = self.next_key()?;

        // SAFETY: has() and get() are read-only operations.
        let inner = unsafe { self.dictionary.as_inner_mut() };
        if !inner.has(&key) {
            return None;
        }

        let value = inner.get(&key, &Variant::nil());
        Some((key, value))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        // Check for underflow in case any entry was removed while iterating; i.e. next_index > dicitonary.len().
        let remaining = usize::saturating_sub(self.dictionary.len(), self.next_idx);

        (remaining, Some(remaining))
    }

    fn call_init(dictionary: &AnyDictionary) -> Option<Variant> {
        let variant: Variant = Variant::nil();
        let iter_fn = |dictionary, next_value: sys::GDExtensionVariantPtr, valid| unsafe {
            interface_fn!(variant_iter_init)(dictionary, sys::SysPtr::as_uninit(next_value), valid)
        };

        Self::ffi_iterate(iter_fn, dictionary, variant)
    }

    fn call_next(dictionary: &AnyDictionary, last_key: Variant) -> Option<Variant> {
        let iter_fn = |dictionary, next_value, valid| unsafe {
            interface_fn!(variant_iter_next)(dictionary, next_value, valid)
        };

        Self::ffi_iterate(iter_fn, dictionary, last_key)
    }

    /// Calls the provided Godot FFI function, in order to iterate the current state.
    ///
    /// # Safety:
    /// `iter_fn` must point to a valid function that interprets the parameters according to their type specification.
    fn ffi_iterate(
        iter_fn: unsafe fn(
            sys::GDExtensionConstVariantPtr,
            sys::GDExtensionVariantPtr,
            *mut sys::GDExtensionBool,
        ) -> sys::GDExtensionBool,
        dictionary: &AnyDictionary,
        mut next_value: Variant,
    ) -> Option<Variant> {
        let dictionary = dictionary.to_variant();
        let mut valid_u8: u8 = 0;

        // SAFETY:
        // `dictionary` is a valid dictionary since we have a reference to it,
        //    so this will call the implementation for dictionaries.
        // `last_key` is an initialized and valid `Variant`, since we own a copy of it.
        let has_next = unsafe {
            iter_fn(
                dictionary.var_sys(),
                next_value.var_sys_mut(),
                ptr::addr_of_mut!(valid_u8),
            )
        };
        let valid = u8_to_bool(valid_u8);
        let has_next = u8_to_bool(has_next);

        if has_next {
            assert!(valid);
            Some(next_value)
        } else {
            None
        }
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

/// Iterator over key-value pairs in a [`VarDictionary`].
///
/// See [`VarDictionary::iter_shared()`] for more information about iteration over dictionaries.
pub struct Iter<'a> {
    iter: DictionaryIter<'a>,
}

impl<'a> Iter<'a> {
    pub(super) fn new(dictionary: &'a AnyDictionary) -> Self {
        Self {
            iter: DictionaryIter::new(dictionary),
        }
    }
    //
    /// Creates an iterator that converts each `(Variant, Variant)` key-value pair into a `(K, V)` key-value
    /// pair, panicking upon conversion failure.
    pub fn typed<K: FromGodot, V: FromGodot>(self) -> TypedIter<'a, K, V> {
        TypedIter::from_untyped(self)
    }
}

impl Iterator for Iter<'_> {
    type Item = (Variant, Variant);

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next_key_value()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

/// Iterator over keys in a [`VarDictionary`].
///
/// See [`VarDictionary::keys_shared()`] for more information about iteration over dictionaries.
pub struct Keys<'a> {
    iter: DictionaryIter<'a>,
}

impl<'a> Keys<'a> {
    pub(super) fn new(dictionary: &'a AnyDictionary) -> Self {
        Self {
            iter: DictionaryIter::new(dictionary),
        }
    }

    /// Creates an iterator that will convert each `Variant` key into a key of type `K`,
    /// panicking upon failure to convert.
    pub fn typed<K: FromGodot>(self) -> TypedKeys<'a, K> {
        TypedKeys::from_untyped(self)
    }

    /// Returns an array of the keys.
    pub fn array(self) -> AnyArray {
        assert!(
            self.iter.is_first,
            "Keys::array() can only be called before iteration has started"
        );
        self.iter.dictionary.keys_array()
    }
}

impl Iterator for Keys<'_> {
    type Item = Variant;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next_key()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

/// [`VarDictionary`] iterator that converts each key-value pair into a typed `(K, V)`.
///
/// See [`VarDictionary::iter_shared()`] for more information about iteration over dictionaries.
pub struct TypedIter<'a, K, V> {
    iter: DictionaryIter<'a>,
    _k: PhantomData<K>,
    _v: PhantomData<V>,
}

impl<'a, K, V> TypedIter<'a, K, V> {
    fn from_untyped(value: Iter<'a>) -> Self {
        Self {
            iter: value.iter,
            _k: PhantomData,
            _v: PhantomData,
        }
    }
}

impl<K: FromGodot, V: FromGodot> Iterator for TypedIter<'_, K, V> {
    type Item = (K, V);

    fn next(&mut self) -> Option<Self::Item> {
        self.iter
            .next_key_value()
            .map(|(key, value)| (K::from_variant(&key), V::from_variant(&value)))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

/// [`VarDictionary`] iterator that converts each key into a typed `K`.
///
/// See [`VarDictionary::iter_shared()`] for more information about iteration over dictionaries.
pub struct TypedKeys<'a, K> {
    iter: DictionaryIter<'a>,
    _k: PhantomData<K>,
}

impl<'a, K> TypedKeys<'a, K> {
    fn from_untyped(value: Keys<'a>) -> Self {
        Self {
            iter: value.iter,
            _k: PhantomData,
        }
    }
}

impl<K: FromGodot> Iterator for TypedKeys<'_, K> {
    type Item = K;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next_key().map(|k| K::from_variant(&k))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Helper functions

fn u8_to_bool(u: u8) -> bool {
    match u {
        0 => false,
        1 => true,
        _ => panic!("Invalid boolean value {u}"),
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

/// Constructs typed [`Dictionary<K, V>`] literals, close to Godot's own syntax.
///
/// Any value can be used as a key, but to use an expression you need to surround it
/// in `()` or `{}`.
///
/// # Type annotation
/// The macro creates a typed `Dictionary<K, V>`. You must provide an explicit type annotation
/// to specify `K` and `V`. Keys must implement `AsArg<K>` and values must implement `AsArg<V>`.
///
/// # Example
/// ```no_run
/// use godot::builtin::{dict, Dictionary, GString, Variant};
///
/// // Type annotation required
/// let d: Dictionary<GString, i64> = dict! {
///     "key1": 10,
///     "key2": 20,
/// };
///
/// // Works with Variant values too
/// let d: Dictionary<GString, Variant> = dict! {
///     "str": "Hello",
///     "num": 23,
/// };
/// ```
///
/// # See also
///
/// For untyped dictionaries, use [`vdict!`][macro@crate::builtin::vdict].
/// For arrays, similar macros [`array!`][macro@crate::builtin::array] and [`varray!`][macro@crate::builtin::varray] exist.
#[macro_export]
macro_rules! dict {
    ($($key:tt: $value:expr),* $(,)?) => {
        {
            let mut d = $crate::builtin::Dictionary::new();
            $(
                // `cargo check` complains that `(1 + 2): true` has unused parentheses, even though it's not possible to omit those.
                #[allow(unused_parens)]
                d.set($key, $value);
            )*
            d
        }
    };
}

/// Constructs [`VarDictionary`] literals, close to Godot's own syntax.
///
/// Any value can be used as a key, but to use an expression you need to surround it
/// in `()` or `{}`.
///
/// # Example
/// ```no_run
/// use godot::builtin::{vdict, Variant};
///
/// let key = "my_key";
/// let d = vdict! {
///     "key1": 10,
///     "another": Variant::nil(),
///     key: true,
///     (1 + 2): "final",
/// };
/// ```
///
/// # See also
///
/// For typed dictionaries, use [`dict!`][macro@crate::builtin::dict].
/// For arrays, similar macros [`array!`][macro@crate::builtin::array] and [`varray!`][macro@crate::builtin::varray] exist.
// TODO(v0.5): unify vdict!/dict! macro implementations; vdict! manually calls to_variant() while dict! uses AsVArg.
#[macro_export]
macro_rules! vdict {
    ($($key:tt: $value:expr_2021),* $(,)?) => {
        {
            use $crate::meta::ToGodot as _;
            let mut dict = $crate::builtin::VarDictionary::new();
            $(
                // `cargo check` complains that `(1 + 2): true` has unused parens, even though it's not
                // possible to omit the parens.
                #[allow(unused_parens)]
                dict.set(&$key.to_variant(), &$value.to_variant());
            )*
            dict
        }
    };
}
