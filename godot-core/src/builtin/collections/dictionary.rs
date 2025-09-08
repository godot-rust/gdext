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
use sys::{ffi_methods, interface_fn, GodotFfi};

use crate::builtin::{inner, Variant, VariantArray};
use crate::meta::{ElementType, ExtVariantType, FromGodot, ToGodot};

/// Godot's `Dictionary` type.
///
/// Ordered associative hash-table, mapping keys to values.
///
/// The keys and values of the dictionary are all `Variant`s, so they can be of different types.
/// Variants are designed to be generally cheap to clone. Typed dictionaries are planned in a future godot-rust version.
///
/// Check out the [book](https://godot-rust.github.io/book/godot-api/builtins.html#arrays-and-dictionaries) for a tutorial on dictionaries.
///
/// # Dictionary example
///
/// ```no_run
/// # use godot::prelude::*;
/// // Create empty dictionary and add key-values pairs.
/// let mut dict = Dictionary::new();
/// dict.set("str", "Hello");
/// dict.set("num", 23);
///
/// // Keys don't need to be strings.
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
/// // Iterate over key-value pairs as (Variant, Variant).
/// for (key, value) in dict.iter_shared() {
///     println!("{key} => {value}");
/// }
///
/// // Use typed::<K, V>() to get typed iterators.
/// for (key, value) in dict.iter_shared().typed::<GString, Variant>() {
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
/// # Thread safety
///
/// The same principles apply as for [`VariantArray`]. Consult its documentation for details.
///
/// # Godot docs
///
/// [`Dictionary` (stable)](https://docs.godotengine.org/en/stable/classes/class_dictionary.html)
pub struct Dictionary {
    opaque: OpaqueDictionary,

    /// Lazily computed and cached element type information for the key type.
    cached_key_type: OnceCell<ElementType>,

    /// Lazily computed and cached element type information for the value type.
    cached_value_type: OnceCell<ElementType>,
}

impl Dictionary {
    fn from_opaque(opaque: OpaqueDictionary) -> Self {
        Self {
            opaque,
            cached_key_type: OnceCell::new(),
            cached_value_type: OnceCell::new(),
        }
    }

    /// Constructs an empty `Dictionary`.
    pub fn new() -> Self {
        Self::default()
    }

    /// ⚠️ Returns the value for the given key, or panics.
    ///
    /// If you want to check for presence, use [`get()`][Self::get] or [`get_or_nil()`][Self::get_or_nil].
    ///
    /// # Panics
    ///
    /// If there is no value for the given key. Note that this is distinct from a `NIL` value, which is returned as `Variant::nil()`.
    pub fn at<K: ToGodot>(&self, key: K) -> Variant {
        // Code duplication with get(), to avoid third clone (since K: ToGodot takes ownership).

        let key = key.to_variant();
        if self.contains_key(key.clone()) {
            self.get_or_nil(key)
        } else {
            panic!("key {key:?} missing in dictionary: {self:?}")
        }
    }

    /// Returns the value for the given key, or `None`.
    ///
    /// Note that `NIL` values are returned as `Some(Variant::nil())`, while absent values are returned as `None`.
    /// If you want to treat both as `NIL`, use [`get_or_nil()`][Self::get_or_nil].
    ///
    /// When you are certain that a key is present, use [`at()`][`Self::at`] instead.
    ///
    /// This can be combined with Rust's `Option` methods, e.g. `dict.get(key).unwrap_or(default)`.
    pub fn get<K: ToGodot>(&self, key: K) -> Option<Variant> {
        // If implementation is changed, make sure to update at().

        let key = key.to_variant();
        if self.contains_key(key.clone()) {
            Some(self.get_or_nil(key))
        } else {
            None
        }
    }

    /// Returns the value at the key in the dictionary, or `NIL` otherwise.
    ///
    /// This method does not let you differentiate `NIL` values stored as values from absent keys.
    /// If you need that, use [`get()`][`Self::get`] instead.
    ///
    /// When you are certain that a key is present, use [`at()`][`Self::at`] instead.
    ///
    /// _Godot equivalent: `dict.get(key, null)`_
    pub fn get_or_nil<K: ToGodot>(&self, key: K) -> Variant {
        self.as_inner().get(&key.to_variant(), &Variant::nil())
    }

    /// Gets a value and ensures the key is set, inserting default if key is absent.
    ///
    /// If the `key` exists in the dictionary, this behaves like [`get()`][Self::get], and the existing value is returned.
    /// Otherwise, the `default` value is inserted and returned.
    ///
    /// # Compatibility
    /// This function is natively available from Godot 4.3 onwards, we provide a polyfill for older versions.
    ///
    /// _Godot equivalent: `get_or_add`_
    #[doc(alias = "get_or_add")]
    pub fn get_or_insert<K: ToGodot, V: ToGodot>(&mut self, key: K, default: V) -> Variant {
        self.debug_ensure_mutable();

        let key_variant = key.to_variant();
        let default_variant = default.to_variant();

        // Godot 4.3+: delegate to native get_or_add().
        #[cfg(since_api = "4.3")]
        {
            self.as_inner().get_or_add(&key_variant, &default_variant)
        }

        // Polyfill for Godot versions before 4.3.
        #[cfg(before_api = "4.3")]
        {
            if let Some(existing_value) = self.get(key_variant.clone()) {
                existing_value
            } else {
                self.set(key_variant, default_variant.clone());
                default_variant
            }
        }
    }

    /// Returns `true` if the dictionary contains the given key.
    ///
    /// _Godot equivalent: `has`_
    #[doc(alias = "has")]
    pub fn contains_key<K: ToGodot>(&self, key: K) -> bool {
        let key = key.to_variant();
        self.as_inner().has(&key)
    }

    /// Returns `true` if the dictionary contains all the given keys.
    ///
    /// _Godot equivalent: `has_all`_
    #[doc(alias = "has_all")]
    pub fn contains_all_keys(&self, keys: &VariantArray) -> bool {
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
    /// Unlike Godot, this will return `None` if the key does not exist and `Some(Variant::nil())` the key is `NIL`.
    ///
    /// This operation is rarely needed and very inefficient. If you find yourself needing it a lot, consider
    /// using a `HashMap` or `Dictionary` with the inverse mapping (`V` -> `K`).
    ///
    /// _Godot equivalent: `find_key`_
    #[doc(alias = "find_key")]
    pub fn find_key_by_value<V: ToGodot>(&self, value: V) -> Option<Variant> {
        let key = self.as_inner().find_key(&value.to_variant());

        if !key.is_nil() || self.contains_key(key.clone()) {
            Some(key)
        } else {
            None
        }
    }

    /// Removes all key-value pairs from the dictionary.
    pub fn clear(&mut self) {
        self.debug_ensure_mutable();

        self.as_inner().clear()
    }

    /// Set a key to a given value.
    ///
    /// If you are interested in the previous value, use [`insert()`][Self::insert] instead.
    ///
    /// _Godot equivalent: `dict[key] = value`_
    pub fn set<K: ToGodot, V: ToGodot>(&mut self, key: K, value: V) {
        self.debug_ensure_mutable();

        let key = key.to_variant();

        // SAFETY: `self.get_ptr_mut(key)` always returns a valid pointer to a value in the dictionary; either pre-existing or newly inserted.
        unsafe {
            value.to_variant().move_into_var_ptr(self.get_ptr_mut(key));
        }
    }

    /// Insert a value at the given key, returning the previous value for that key (if available).
    ///
    /// If you don't need the previous value, use [`set()`][Self::set] instead.
    #[must_use]
    pub fn insert<K: ToGodot, V: ToGodot>(&mut self, key: K, value: V) -> Option<Variant> {
        self.debug_ensure_mutable();

        let key = key.to_variant();
        let old_value = self.get(key.clone());
        self.set(key, value);
        old_value
    }

    /// Removes a key from the map, and returns the value associated with
    /// the key if the key was in the dictionary.
    ///
    /// _Godot equivalent: `erase`_
    #[doc(alias = "erase")]
    pub fn remove<K: ToGodot>(&mut self, key: K) -> Option<Variant> {
        self.debug_ensure_mutable();

        let key = key.to_variant();
        let old_value = self.get(key.clone());
        self.as_inner().erase(&key);
        old_value
    }

    /// Returns a 32-bit integer hash value representing the dictionary and its contents.
    #[must_use]
    pub fn hash(&self) -> u32 {
        self.as_inner().hash().try_into().unwrap()
    }

    /// Creates a new `Array` containing all the keys currently in the dictionary.
    ///
    /// _Godot equivalent: `keys`_
    #[doc(alias = "keys")]
    pub fn keys_array(&self) -> VariantArray {
        self.as_inner().keys()
    }

    /// Creates a new `Array` containing all the values currently in the dictionary.
    ///
    /// _Godot equivalent: `values`_
    #[doc(alias = "values")]
    pub fn values_array(&self) -> VariantArray {
        self.as_inner().values()
    }

    /// Copies all keys and values from `other` into `self`.
    ///
    /// If `overwrite` is true, it will overwrite pre-existing keys.
    ///
    /// _Godot equivalent: `merge`_
    #[doc(alias = "merge")]
    pub fn extend_dictionary(&mut self, other: &Self, overwrite: bool) {
        self.debug_ensure_mutable();

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
        self.as_inner().duplicate(true).with_cache(self)
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
        self.as_inner().duplicate(false).with_cache(self)
    }

    /// Returns an iterator over the key-value pairs of the `Dictionary`.
    ///
    /// The pairs are each of type `(Variant, Variant)`. Each pair references the original `Dictionary`, but instead of a `&`-reference
    /// to key-value pairs as you might expect, the iterator returns a (cheap, shallow) copy of each key-value pair.
    ///
    /// Note that it's possible to modify the `Dictionary` through another reference while iterating over it. This will not result in
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
    /// # Semantics and alternatives
    /// You can use this in Rust, but the behavior of mutating methods is only validated in a best-effort manner (more than in GDScript though):
    /// some methods like `set()` panic in Debug mode, when used on a read-only dictionary. There is no guarantee that any attempts to change
    /// result in feedback; some may silently do nothing.
    ///
    /// In Rust, you can use shared references (`&Dictionary`) to prevent mutation. Note however that `Clone` can be used to create another
    /// reference, through which mutation can still occur. For deep-immutable dictionaries, you'll need to keep your `Dictionary` encapsulated
    /// or directly use Rust data structures.
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

    /// Best-effort mutability check.
    ///
    /// # Panics
    /// In Debug mode, if the array is marked as read-only.
    fn debug_ensure_mutable(&self) {
        debug_assert!(
            !self.is_read_only(),
            "mutating operation on read-only dictionary"
        );
    }

    /// Returns the runtime element type information for keys in this dictionary.
    ///
    /// Provides information about Godot typed dictionaries, even though godot-rust currently doesn't implement generics for those.
    ///
    /// The result is generally cached, so feel free to call this method repeatedly.
    ///
    /// # Panics (Debug)
    /// In the astronomically rare case where another extension in Godot modifies a dictionary's key type (which godot-rust already cached as `Untyped`)
    /// via C function `dictionary_set_typed`, thus leading to incorrect cache values. Such bad practice of not typing dictionaries immediately on
    /// construction is not supported, and will not be checked in Release mode.
    #[cfg(since_api = "4.4")]
    pub fn key_element_type(&self) -> ElementType {
        ElementType::get_or_compute_cached(
            &self.cached_key_type,
            || self.as_inner().get_typed_key_builtin(),
            || self.as_inner().get_typed_key_class_name(),
            || self.as_inner().get_typed_key_script(),
        )
    }

    /// Returns the runtime element type information for values in this dictionary.
    ///
    /// Provides information about Godot typed dictionaries, even though godot-rust currently doesn't implement generics for those.
    ///
    /// The result is generally cached, so feel free to call this method repeatedly.
    ///
    /// # Panics (Debug)
    /// In the astronomically rare case where another extension in Godot modifies a dictionary's value type (which godot-rust already cached as `Untyped`)
    /// via C function `dictionary_set_typed`, thus leading to incorrect cache values. Such bad practice of not typing dictionaries immediately on
    /// construction is not supported, and will not be checked in Release mode.
    #[cfg(since_api = "4.4")]
    pub fn value_element_type(&self) -> ElementType {
        ElementType::get_or_compute_cached(
            &self.cached_value_type,
            || self.as_inner().get_typed_value_builtin(),
            || self.as_inner().get_typed_value_class_name(),
            || self.as_inner().get_typed_value_script(),
        )
    }

    #[doc(hidden)]
    pub fn as_inner(&self) -> inner::InnerDictionary<'_> {
        inner::InnerDictionary::from_outer(self)
    }

    /// Get the pointer corresponding to the given key in the dictionary.
    ///
    /// If there exists no value at the given key, a `NIL` variant will be inserted for that key.
    fn get_ptr_mut<K: ToGodot>(&mut self, key: K) -> sys::GDExtensionVariantPtr {
        let key = key.to_variant();

        // Never a null pointer, since entry either existed already or was inserted above.
        // SAFETY: accessing an unknown key _mutably_ creates that entry in the dictionary, with value `NIL`.
        unsafe { interface_fn!(dictionary_operator_index)(self.sys_mut(), key.var_sys()) }
    }

    /// Execute a function that creates a new Dictionary, transferring cached element types if available.
    ///
    /// This is a convenience helper for methods that create new Dictionary instances and want to preserve
    /// cached type information to avoid redundant FFI calls.
    fn with_cache(self, source: &Self) -> Self {
        // Transfer both key and value type caches independently
        ElementType::transfer_cache(&source.cached_key_type, &self.cached_key_type);
        ElementType::transfer_cache(&source.cached_value_type, &self.cached_value_type);
        self
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Traits

// SAFETY:
// - `move_return_ptr`
//   Nothing special needs to be done beyond a `std::mem::swap` when returning a Dictionary.
//   So we can just use `ffi_methods`.
//
// - `from_arg_ptr`
//   Dictionaries are properly initialized through a `from_sys` call, but the ref-count should be
//   incremented as that is the callee's responsibility. Which we do by calling
//   `std::mem::forget(dictionary.clone())`.
unsafe impl GodotFfi for Dictionary {
    const VARIANT_TYPE: ExtVariantType = ExtVariantType::Concrete(sys::VariantType::DICTIONARY);

    ffi_methods! { type sys::GDExtensionTypePtr = *mut Opaque; .. }
}

crate::meta::impl_godot_as_self!(Dictionary: ByRef);

impl_builtin_traits! {
    for Dictionary {
        Default => dictionary_construct_default;
        Drop => dictionary_destroy;
        PartialEq => dictionary_operator_equal;
        // No < operator for dictionaries.
        // Hash could be added, but without Eq it's not that useful.
    }
}

impl fmt::Debug for Dictionary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.to_variant().stringify())
    }
}

impl fmt::Display for Dictionary {
    /// Formats `Dictionary` to match Godot's string representation.
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

/// Creates a new reference to the data in this dictionary. Changes to the original dictionary will be
/// reflected in the copy and vice versa.
///
/// To create a (mostly) independent copy instead, see [`Dictionary::duplicate_shallow()`] and
/// [`Dictionary::duplicate_deep()`].
impl Clone for Dictionary {
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

/// Creates a dictionary from the given iterator `I` over a `(&K, &V)` key-value pair.
///
/// Each key and value are converted to a `Variant`.
impl<'a, 'b, K, V, I> From<I> for Dictionary
where
    I: IntoIterator<Item = (&'a K, &'b V)>,
    K: ToGodot + 'a,
    V: ToGodot + 'b,
{
    fn from(iterable: I) -> Self {
        iterable
            .into_iter()
            .map(|(key, value)| (key.to_variant(), value.to_variant()))
            .collect()
    }
}

/// Insert iterator range into dictionary.
///
/// Inserts all key-value pairs from the iterator into the dictionary. Previous values for keys appearing
/// in `iter` will be overwritten.
impl<K: ToGodot, V: ToGodot> Extend<(K, V)> for Dictionary {
    fn extend<I: IntoIterator<Item = (K, V)>>(&mut self, iter: I) {
        for (k, v) in iter.into_iter() {
            self.set(k.to_variant(), v.to_variant())
        }
    }
}

impl<K: ToGodot, V: ToGodot> FromIterator<(K, V)> for Dictionary {
    fn from_iter<I: IntoIterator<Item = (K, V)>>(iter: I) -> Self {
        let mut dict = Dictionary::new();
        dict.extend(iter);
        dict
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

/// Internal helper for different iterator impls -- not an iterator itself
struct DictionaryIter<'a> {
    last_key: Option<Variant>,
    dictionary: &'a Dictionary,
    is_first: bool,
    next_idx: usize,
}

impl<'a> DictionaryIter<'a> {
    fn new(dictionary: &'a Dictionary) -> Self {
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
        if !self.dictionary.contains_key(key.clone()) {
            return None;
        }

        let value = self.dictionary.as_inner().get(&key, &Variant::nil());
        Some((key, value))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        // Need to check for underflow in case any entry was removed while
        // iterating (i.e. next_index > dicitonary.len())
        let remaining = usize::saturating_sub(self.dictionary.len(), self.next_idx);

        (remaining, Some(remaining))
    }

    fn call_init(dictionary: &Dictionary) -> Option<Variant> {
        let variant: Variant = Variant::nil();
        let iter_fn = |dictionary, next_value: sys::GDExtensionVariantPtr, valid| unsafe {
            interface_fn!(variant_iter_init)(dictionary, sys::SysPtr::as_uninit(next_value), valid)
        };

        Self::ffi_iterate(iter_fn, dictionary, variant)
    }

    fn call_next(dictionary: &Dictionary, last_key: Variant) -> Option<Variant> {
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
        dictionary: &Dictionary,
        mut next_value: Variant,
    ) -> Option<Variant> {
        let dictionary = dictionary.to_variant();
        let mut valid_u8: u8 = 0;

        // SAFETY:
        // `dictionary` is a valid `Dictionary` since we have a reference to it,
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
/// See [`Dictionary::iter_shared()`] for more information about iteration over dictionaries.
pub struct Iter<'a> {
    iter: DictionaryIter<'a>,
}

impl<'a> Iter<'a> {
    fn new(dictionary: &'a Dictionary) -> Self {
        Self {
            iter: DictionaryIter::new(dictionary),
        }
    }

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

/// Iterator over keys in a [`Dictionary`].
///
/// See [`Dictionary::keys_shared()`] for more information about iteration over dictionaries.
pub struct Keys<'a> {
    iter: DictionaryIter<'a>,
}

impl<'a> Keys<'a> {
    fn new(dictionary: &'a Dictionary) -> Self {
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
    pub fn array(self) -> VariantArray {
        // Can only be called
        assert!(self.iter.is_first);
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

/// [`Dictionary`] iterator that converts each key-value pair into a typed `(K, V)`.
///
/// See [`Dictionary::iter_shared()`] for more information about iteration over dictionaries.
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

/// [`Dictionary`] iterator that converts each key into a typed `K`.
///
/// See [`Dictionary::iter_shared()`] for more information about iteration over dictionaries.
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

/// Constructs [`Dictionary`] literals, close to Godot's own syntax.
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
/// For arrays, similar macros [`array!`][macro@crate::builtin::array] and [`varray!`][macro@crate::builtin::varray] exist.
#[macro_export]
macro_rules! vdict {
    ($($key:tt: $value:expr),* $(,)?) => {
        {
            let mut d = $crate::builtin::Dictionary::new();
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

#[macro_export]
#[deprecated = "Migrate to `vdict!`. The name `dict!` will be used in the future for typed dictionaries."]
macro_rules! dict {
    ($($key:tt: $value:expr),* $(,)?) => {
        $crate::vdict!(
            $($key: $value),*
        )
    };
}
