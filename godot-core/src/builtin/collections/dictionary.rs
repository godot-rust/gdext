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
use crate::meta::inspect::ElementType;
use crate::meta::shape::{GodotElementShape, GodotShape};
use crate::meta::{AsArg, Element, ExtVariantType, FromGodot, ToGodot};
use crate::registry::info::ParamMetadata;
use crate::registry::property::{BuiltinExport, Export, Var};

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
/// # Typed example
/// ```no_run
/// # use godot::prelude::*;
///
/// // Define a Godot-exported enum.
/// #[derive(GodotConvert)]
/// #[godot(via = GString)]
/// enum Tile { GRASS, ROCK, WATER }
///
/// let mut tiles = Dictionary::<Vector2i, Tile>::new();
/// tiles.set(Vector2i::new(1, 2), Tile::GRASS);
/// tiles.set(Vector2i::new(1, 3), Tile::WATER);
///
/// // Create the same dictionary in a single expression.
/// let tiles: Dictionary<Vector2i, Tile> = dict! {
///    Vector2i::new(1, 2) => Tile::GRASS,
///    Vector2i::new(1, 3) => Tile::WATER,
/// };
///
/// // Element access is now strongly typed.
/// let value = tiles.at(Vector2i::new(1, 3)); // type Tile.
/// ```
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
    pub(super) cached_key_type: OnceCell<ElementType>,

    /// Lazily computed and cached element type information for the value type.
    pub(super) cached_value_type: OnceCell<ElementType>,
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
    pub fn at(&self, key: impl AsArg<K>) -> V {
        meta::arg_into_ref!(key: K);
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
    pub fn get(&self, key: impl AsArg<K>) -> Option<V> {
        meta::arg_into_ref!(key: K);
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

    // TODO(v0.6): avoid double FFI round-trip (has + get); consider using get(key, sentinel) pattern.
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
    pub fn get_or_insert(&mut self, key: impl AsArg<K>, default: impl AsArg<V>) -> V {
        self.balanced_ensure_mutable();

        meta::arg_into_ref!(key: K);
        meta::arg_into_ref!(default: V);

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
    pub fn contains_key(&self, key: impl AsArg<K>) -> bool {
        meta::arg_into_ref!(key: K);
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
    pub fn find_key_by_value(&self, value: impl AsArg<V>) -> Option<K>
    where
        K: FromGodot,
    {
        meta::arg_into_ref!(value: V);
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
    /// accepts any `impl ToGodot` for the Variant positions, thanks to blanket `AsArg<Variant>` impls.
    ///
    /// _Godot equivalent: `dict[key] = value`_
    pub fn set(&mut self, key: impl AsArg<K>, value: impl AsArg<V>) {
        self.balanced_ensure_mutable();

        meta::arg_into_ref!(key: K);
        meta::arg_into_ref!(value: V);

        // SAFETY: K and V strongly typed.
        unsafe { self.set_variant(key.to_variant(), value.to_variant()) };
    }

    /// Internal helper for the `dict!` macro; uses [`AsDirectElement`][meta::AsDirectElement] for unambiguous type inference.
    #[doc(hidden)]
    pub fn __macro_set_direct<Ke, Ve>(&mut self, key: Ke, value: Ve)
    where
        Ke: meta::AsDirectElement<K>,
        Ve: meta::AsDirectElement<V>,
    {
        self.set(key, value)
    }

    /// Insert a value at the given key, returning the previous value for that key (if available).
    ///
    /// If you don't need the previous value, use [`set()`][Self::set] instead.
    #[must_use]
    pub fn insert(&mut self, key: impl AsArg<K>, value: impl AsArg<V>) -> Option<V> {
        self.balanced_ensure_mutable();

        meta::arg_into_ref!(key: K);
        meta::arg_into_ref!(value: V);

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
    pub fn remove(&mut self, key: impl AsArg<K>) -> Option<V> {
        self.balanced_ensure_mutable();

        meta::arg_into_ref!(key: K);

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
    /// Each pair is a (cheap, shallow) copy of the key-value pair in the dictionary.
    ///
    /// Note that it's possible to modify the dictionary through another reference while iterating over it. This will not result in
    /// unsoundness or crashes, but will cause the iterator to behave in an unspecified way.
    pub fn iter_shared(&self) -> DictIter<'_, K, V> {
        DictIter::new(self)
    }

    /// Returns an iterator over the keys in the `Dictionary`.
    ///
    /// Each key is a (cheap, shallow) copy from the original dictionary.
    ///
    /// Note that it's possible to modify the `Dictionary` through another reference while iterating over it. This will not result in
    /// unsoundness or crashes, but will cause the iterator to behave in an unspecified way.
    pub fn keys_shared(&self) -> DictKeys<'_, K> {
        DictKeys::new(self)
    }

    /// Returns an iterator over the values in the `Dictionary`.
    ///
    /// Each value is a (cheap, shallow) copy from the original dictionary.
    ///
    /// Note that it's possible to modify the `Dictionary` through another reference while iterating over it. This will not result in
    /// unsoundness or crashes, but will cause the iterator to behave in an unspecified way.
    pub fn values_shared(&self) -> DictValues<'_, V> {
        DictValues::new(self)
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

    /// Shared logic for `init_inner_type`: caches types and calls `dictionary_set_typed`.
    ///
    /// The function pointer is passed in rather than looked up here because the lookup differs between API versions: as the FFI function is
    /// only available for API >= 4.4, we fetch by raw string before.
    //
    // TODO(v0.6): simplify once migrated to `gdextension_interface.json`.
    fn init_inner_type_with(&mut self, dictionary_set_typed: DictionarySetTyped) {
        let key_elem_ty = ElementType::of::<K>();
        let value_elem_ty = ElementType::of::<V>();

        // If both are untyped (Variant), skip initialization.
        if !key_elem_ty.is_typed() && !value_elem_ty.is_typed() {
            return;
        }

        // Cache types, since we know them at compile time.
        self.cached_key_type.get_or_init(|| key_elem_ty);
        self.cached_value_type.get_or_init(|| value_elem_ty);

        // Script is always nil for compile-time types (only relevant for GDScript class_name types).
        let script = Variant::nil();
        let empty_string_name = crate::builtin::StringName::default();
        let key_class_name = key_elem_ty.class_name_sys_or(&empty_string_name);
        let value_class_name = value_elem_ty.class_name_sys_or(&empty_string_name);

        // SAFETY: Valid pointers are passed in.
        // Relevant for correctness, not safety: the dictionary is a newly created, empty, untyped dictionary.
        unsafe {
            dictionary_set_typed(
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

    /// On Godot 4.4+, calls `dictionary_set_typed()` to inform the engine about types and caches the result.
    #[cfg(since_api = "4.4")]
    fn init_inner_type(&mut self) {
        self.init_inner_type_with(interface_fn!(dictionary_set_typed));
    }

    /// Compiled against pre-4.4 API: if running on Godot 4.4+, dynamically look up `dictionary_set_typed`
    /// and call it to register type info with the engine.
    #[cfg(before_api = "4.4")]
    fn init_inner_type(&mut self) {
        if !sys::GdextBuild::since_api("4.4") {
            // Pre-4.4 runtime: typed dicts not supported -> cache Untyped to avoid re-probing on each query.
            self.cached_key_type.get_or_init(|| ElementType::Untyped);
            self.cached_value_type.get_or_init(|| ElementType::Untyped);
            return;
        }

        // SAFETY: Binding has been initialized; `dictionary_set_typed` exists on 4.4+ runtime.
        let fptr = unsafe { sys::get_ffi_ptr_by_cstr(b"dictionary_set_typed\0") }
            .expect("dictionary_set_typed should be available on Godot 4.4+");

        // SAFETY: the function pointer has the correct signature (stable GDExtension ABI).
        let dictionary_set_typed: DictionarySetTyped = unsafe { std::mem::transmute(fptr) };

        self.init_inner_type_with(dictionary_set_typed);
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
    pub fn get_or_nil(&self, key: impl AsArg<K>) -> Variant {
        meta::arg_into_ref!(key: K);
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

impl<K: Element + fmt::Display, V: Element + fmt::Display> fmt::Display for Dictionary<K, V> {
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

impl<'a, K: Element, V: Element> IntoIterator for &'a Dictionary<K, V> {
    type Item = (K, V);
    type IntoIter = DictIter<'a, K, V>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter_shared()
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
            // Inline set logic to avoid generic owned_into_arg() (which can't resolve T::Pass).
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

    fn godot_shape() -> GodotShape {
        if !is_dictionary_typed::<K, V>() {
            return GodotShape::Builtin {
                variant_type: VariantType::DICTIONARY,
                metadata: ParamMetadata::NONE,
            };
        }

        GodotShape::TypedDictionary {
            key: GodotElementShape::new(K::godot_shape()),
            value: GodotElementShape::new(V::godot_shape()),
        }
    }
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
}

// Only implement for untyped dictionaries; typed dictionaries cannot be nested in typed containers.
// Analogous to how only `VarArray` (not `Array<T>`) implements `Element`.
impl Element for VarDictionary {}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Var/Export implementations for Dictionary<K, V>

/// Check if Dictionary<K, V> is typed (at least one of K or V is not Variant).
#[inline]
fn is_dictionary_typed<K: Element, V: Element>() -> bool {
    // Nil means "untyped" or "Variant" in Godot.
    meta::element_variant_type::<K>() != VariantType::NIL
        || meta::element_variant_type::<V>() != VariantType::NIL
}

// No Var bound on K, V.
impl<K: Element, V: Element> Var for Dictionary<K, V> {
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
}

impl<K, V> Export for Dictionary<K, V>
where
    K: Element + Export,
    V: Element + Export,
{
}

impl<K: Element, V: Element> BuiltinExport for Dictionary<K, V> {}

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

/// Iterator over key-value pairs in a [`Dictionary`].
///
/// Yields `(K, V)` pairs. Each pair is a (cheap, shallow) copy from the original dictionary.
///
/// Note that it's possible to modify the dictionary through another reference while iterating over it. This will not result in
/// unsoundness or crashes, but will cause the iterator to behave in an unspecified way.
pub struct DictIter<'a, K, V> {
    iter: DictionaryIter<'a>,
    _kv: PhantomData<(K, V)>,
}

impl<'a, K, V> DictIter<'a, K, V> {
    pub(super) fn new(dictionary: &'a AnyDictionary) -> Self {
        Self {
            iter: DictionaryIter::new(dictionary),
            _kv: PhantomData,
        }
    }
}

impl<'a> DictIter<'a, Variant, Variant> {
    /// Re-types this iterator to yield `(K, V)` instead of `(Variant, Variant)`.
    ///
    /// Only available on untyped `VarDictionary` and `AnyDictionary` iterators (i.e. `DictIter<'_, Variant, Variant>`), to prevent misleading API
    /// on already-typed iterators where the types are known at compile time.
    ///
    /// The conversion is performed by [`FromGodot`] on each key and value; panics on type mismatch.
    ///
    /// Preserves the current iteration position.
    pub fn typed<K: FromGodot, V: FromGodot>(self) -> DictIter<'a, K, V> {
        DictIter {
            iter: self.iter,
            _kv: PhantomData,
        }
    }
}

impl<K: FromGodot, V: FromGodot> Iterator for DictIter<'_, K, V> {
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

/// Iterator over keys in a [`Dictionary`].
///
/// Yields keys of type `K`. Each key is a (cheap, shallow) copy from the original dictionary.
///
/// Note that it's possible to modify the dictionary through another reference while iterating over it. This will not result in
/// unsoundness or crashes, but will cause the iterator to behave in an unspecified way.
pub struct DictKeys<'a, K> {
    iter: DictionaryIter<'a>,
    _k: PhantomData<K>,
}

impl<'a, K> DictKeys<'a, K> {
    pub(super) fn new(dictionary: &'a AnyDictionary) -> Self {
        Self {
            iter: DictionaryIter::new(dictionary),
            _k: PhantomData,
        }
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

impl<'a> DictKeys<'a, Variant> {
    /// Re-types this iterator to yield `K` instead of `Variant`.
    ///
    /// Only available on untyped `VarDictionary` and `AnyDictionary` key iterators (i.e. `DictIter<'_, Variant, Variant>`), to prevent misleading
    /// API on already-typed iterators where the types are known at compile time.
    ///
    /// The conversion is performed by [`FromGodot`]; panics on type mismatch.
    ///
    /// Preserves the current iteration position.
    pub fn typed<K: FromGodot>(self) -> DictKeys<'a, K> {
        DictKeys {
            iter: self.iter,
            _k: PhantomData,
        }
    }
}

impl<K: FromGodot> Iterator for DictKeys<'_, K> {
    type Item = K;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next_key().map(|k| K::from_variant(&k))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

/// Iterator over values in a [`Dictionary`].
///
/// Yields values of type `V`. Each value is a (cheap, shallow) copy from the original dictionary.
///
/// Note that it's possible to modify the dictionary through another reference while iterating over it. This will not result in
/// unsoundness or crashes, but will cause the iterator to behave in an unspecified way.
pub struct DictValues<'a, V> {
    iter: DictionaryIter<'a>,
    _v: PhantomData<V>,
}

impl<'a, V> DictValues<'a, V> {
    pub(super) fn new(dictionary: &'a AnyDictionary) -> Self {
        Self {
            iter: DictionaryIter::new(dictionary),
            _v: PhantomData,
        }
    }
}

impl<'a> DictValues<'a, Variant> {
    /// Re-types this iterator to yield `V` instead of `Variant`.
    ///
    /// Only available on untyped `VarDictionary` and `AnyDictionary` value iterators (i.e. `DictIter<'_, Variant, Variant>`), to prevent misleading
    /// API on already-typed iterators where the types are known at compile time.
    ///
    /// The conversion is performed by [`FromGodot`]; panics on type mismatch.
    ///
    /// Preserves the current iteration position.
    pub fn typed<V: FromGodot>(self) -> DictValues<'a, V> {
        DictValues {
            iter: self.iter,
            _v: PhantomData,
        }
    }
}

impl<V: FromGodot> Iterator for DictValues<'_, V> {
    type Item = V;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter
            .next_key_value()
            .map(|(_, value)| V::from_variant(&value))
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
// Expression macros
// Intra-doc-links use HTML to stay in same module (not switch to prelude when looking at godot::builtin).

/// **Dict**: constructs `Dictionary` literals for all possible key and value types.
///
/// # Type inference
/// There are three related macros, all of which create [`Dictionary<K, V>`] expressions, but they differ in how the types `K` and `V` are inferred:
///
/// - `dict!` uses [`AsArg`][crate::meta::AsArg] to set entries. This works when the dictionary's key type `K` and value type `V` are already
///   determined from context -- a type annotation, function parameter, etc. This supports all key/value types including `Gd<T>` and `Variant`.
/// - [`idict!`](macro.idict.html) uses [`AsDirectElement`][crate::meta::AsDirectElement] for (opinionated) type inference from literals.
///   This macro needs no type annotations, however is limited to common types, like `i32`, `&str` (inferred as `GString`), etc.
/// - [`vdict!`](macro.vdict.html) uses `AsArg<Variant>`, meaning it's like `dict!` but inferred as [`VarDictionary`].
///
/// # Examples
/// ```no_run
/// # use godot::prelude::*;
/// // dict! requires type context (e.g. annotation, return type, etc.).
/// // The same expression can be used to initialize different dictionary types:
///
/// let d: Dictionary<GString, i64>     = dict! { "key1" => 10, "key2" => 20 };
/// let d: Dictionary<StringName, u16>  = dict! { "key1" => 10, "key2" => 20 };
/// let d: Dictionary<Variant, Variant> = dict! { "key1" => 10, "key2" => 20 };
///
/// // More strict inference with idict! and vdict! macros:
///
/// let d = idict! { "key1" => 10, "key2" => 20 }; // Dictionary<GString, i32>.
///
/// let d = vdict! { "key1" => 10, "key2" => 20 }; // VarDictionary.
/// ```
///
/// # See also
/// For arrays, a similar macro [`array!`](macro.array.html) exists.
#[macro_export]
macro_rules! dict {
    ($($key:expr => $value:expr),* $(,)?) => {
        {
            let mut d = $crate::builtin::Dictionary::new();
            $(
                d.set($key, $value);
            )*
            d
        }
    };
}

/// **I**nferred **dict**: constructs `Dictionary` literals without ambiguity.
///
/// See [`dict!`](macro.dict.html) for docs and examples.
#[macro_export]
macro_rules! idict {
    ($($key:expr => $value:expr),* $(,)?) => {
        {
            let mut d = $crate::builtin::Dictionary::new();
            $(
                d.__macro_set_direct($key, $value);
            )*
            d
        }
    };
}

/// **V**ariant **dict**: constructs [`VarDictionary`] literals.
///
/// See [`dict!`](macro.dict.html) for docs and examples.
#[macro_export]
macro_rules! vdict {
    // New primary syntax with `=>`. Uses `AsArg` semantics, consistent with `dict!`.
    ($($key:expr => $value:expr_2021),* $(,)?) => {
        {
            let mut dict = $crate::builtin::VarDictionary::new();
            $(
                dict.set($key, $value);
            )*
            dict
        }
    };

    // Old syntax with `:`, deprecated.
    ($($key:tt: $value:expr),* $(,)?) => {
        {
            const _: () = $crate::__deprecated::vdict_colon_syntax();
            let mut d = $crate::builtin::VarDictionary::new();
            $(
                // `cargo check` complains that `(1 + 2): true` has unused parens, even though it's not
                // possible to omit the parens.
                #[allow(unused_parens)]
                d.set($key, $value);
            )*
            d
        }
    };
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Polyfill for API <4.4 typed-ness

// Manually define the function pointer type (not available in pre-4.4 generated code).
type DictionarySetTyped = unsafe extern "C" fn(
    sys::GDExtensionTypePtr,
    sys::GDExtensionVariantType,
    sys::GDExtensionConstStringNamePtr,
    sys::GDExtensionConstVariantPtr,
    sys::GDExtensionVariantType,
    sys::GDExtensionConstStringNamePtr,
    sys::GDExtensionConstVariantPtr,
);
