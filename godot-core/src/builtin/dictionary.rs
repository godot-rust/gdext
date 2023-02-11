/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use godot_ffi as sys;

use crate::builtin::{inner, FromVariant, ToVariant, Variant};
use crate::obj::Share;
use std::fmt;
use std::marker::PhantomData;
use std::ptr::addr_of_mut;
use sys::types::OpaqueDictionary;
use sys::{ffi_methods, interface_fn, GodotFfi};

use super::Array;

/// Godot's `Dictionary` type.
///
/// The keys and values of the array are all `Variant`, so they can all be of different types.
/// Variants are designed to be generally cheap to clone.
///
/// # Thread safety
///
/// The same principles apply as for [`Array`]. Consult its documentation for details.
#[repr(C)]
pub struct Dictionary {
    opaque: OpaqueDictionary,
}

impl Dictionary {
    fn from_opaque(opaque: OpaqueDictionary) -> Self {
        Self { opaque }
    }

    /// Constructs an empty `Dictionary`.
    pub fn new() -> Self {
        Self::default()
    }

    /// Removes all key-value pairs from the dictionary. Equivalent to `clear` in Godot.
    pub fn clear(&mut self) {
        self.as_inner().clear()
    }

    /// Returns a deep copy of the dictionary. All nested arrays and dictionaries are duplicated and
    /// will not be shared with the original dictionary. Note that any `Object`-derived elements will
    /// still be shallow copied.
    ///
    /// To create a shallow copy, use [`duplicate_shallow()`] instead. To create a new reference to
    /// the same array data, use [`share()`].
    ///
    /// Equivalent to `dictionary.duplicate(true)` in Godot.
    pub fn duplicate_deep(&self) -> Self {
        self.as_inner().duplicate(true)
    }

    /// Returns a shallow copy of the dictionary. All dictionary keys and values are copied, but
    /// any reference types (such as `Array`, `Dictionary` and `Object`) will still refer to the
    /// same value.
    ///
    /// To create a deep copy, use [`duplicate_deep()`] instead. To create a new reference to the
    /// same dictionary data, use [`share()`].
    ///
    /// Equivalent to `dictionary.duplicate(false)` in Godot.
    pub fn duplicate_shallow(&self) -> Self {
        self.as_inner().duplicate(false)
    }

    /// Removes a key from the map, and returns the value associated with
    /// the key if the key was in the dictionary.
    pub fn remove<K: ToVariant>(&mut self, key: K) -> Option<Variant> {
        let key = key.to_variant();
        let old_value = self.get(key.clone());
        self.as_inner().erase(key);
        old_value
    }

    /// Returns the first key whose associated value is `value`, if one exists.
    ///
    /// Unlike in Godot, this will return `None` if the key does not exist
    /// and `Some(nil)` the key is `null`.
    pub fn find_key_by_value<V: ToVariant>(&self, value: V) -> Option<Variant> {
        let key = self.as_inner().find_key(value.to_variant());

        if !key.is_nil() || self.contains_key(key.clone()) {
            Some(key)
        } else {
            None
        }
    }

    /// Returns the value at the key in the dictionary, if there is
    /// one.
    ///
    /// Unlike `get` in Godot, this will return `None` if there is
    /// no value with the given key.
    pub fn get<K: ToVariant>(&self, key: K) -> Option<Variant> {
        let key = key.to_variant();
        if !self.contains_key(key.clone()) {
            return None;
        }

        Some(self.get_or_nil(key))
    }

    /// Returns the value at the key in the dictionary, or nil otherwise. This
    /// method does not let you differentiate `NIL` values stored as values from
    /// absent keys. If you need that, use `get()`.
    ///
    /// This is equivalent to `get` in Godot.
    pub fn get_or_nil<K: ToVariant>(&self, key: K) -> Variant {
        self.as_inner().get(key.to_variant(), Variant::nil())
    }

    /// Returns `true` if the dictionary contains the given key.
    ///
    /// This is equivalent to `has` in Godot.
    pub fn contains_key<K: ToVariant>(&self, key: K) -> bool {
        let key = key.to_variant();
        self.as_inner().has(key)
    }

    /// Returns `true` if the dictionary contains all the given keys.
    ///
    /// This is equivalent to `has_all` in Godot.
    pub fn contains_all_keys(&self, keys: Array) -> bool {
        self.as_inner().has_all(keys)
    }

    /// Returns a 32-bit integer hash value representing the dictionary and its contents.
    pub fn hash(&self) -> u32 {
        self.as_inner().hash().try_into().unwrap()
    }

    /// Creates a new `Array` containing all the keys currently in the dictionary.
    pub fn keys_array(&self) -> Array {
        self.as_inner().keys()
    }

    /// Creates a new `Array` containing all the values currently in the dictionary.
    pub fn values_array(&self) -> Array {
        self.as_inner().values()
    }

    /// Returns true if the dictionary is empty.
    pub fn is_empty(&self) -> bool {
        self.as_inner().is_empty()
    }

    /// Copies all keys and values from other into self.
    ///
    /// If overwrite is true, it will overwrite pre-existing keys. Otherwise
    /// it will not.
    ///
    /// This is equivalent to `merge` in Godot.
    pub fn extend_dictionary(&mut self, other: Self, overwrite: bool) {
        self.as_inner().merge(other, overwrite)
    }

    /// Returns the number of entries in the dictionary.
    ///
    /// This is equivalent to `size` in Godot.
    pub fn len(&self) -> usize {
        self.as_inner().size().try_into().unwrap()
    }

    /// Get the pointer corresponding to the given key in the dictionary,
    /// if there exists no value at the given key then a new one is created
    /// and initialized to a nil variant.
    fn get_ptr_mut<K: ToVariant>(&mut self, key: K) -> *mut Variant {
        let key = key.to_variant();
        unsafe {
            let ptr =
                (interface_fn!(dictionary_operator_index))(self.sys_mut(), key.var_sys_const());
            assert!(!ptr.is_null());
            ptr as *mut Variant
        }
    }

    /// Insert a value at the given key, returning the value
    /// that previously was at that key if there was one.
    pub fn insert<K: ToVariant, V: ToVariant>(&mut self, key: K, value: V) -> Option<Variant> {
        let key = key.to_variant();
        let old_value = self.get(key.clone());
        self.set(key, value);
        old_value
    }

    /// Set a key to a given value.
    pub fn set<K: ToVariant, V: ToVariant>(&mut self, key: K, value: V) {
        let key = key.to_variant();
        unsafe {
            *self.get_ptr_mut(key) = value.to_variant();
        }
    }

    /// Returns an iterator over the key-value pairs of the `Dictionary`. The pairs are each of type `(Variant, Variant)`.
    /// Each pair references the original `Dictionary`, but instead of a `&`-reference to key-value pairs as
    /// you might expect, the iterator returns a (cheap, shallow) copy of each key-value pair.
    ///
    /// Note that it's possible to modify the `Dictionary` through another reference while iterating
    /// over it. This will not result in unsoundness or crashes, but will cause the iterator to
    /// behave in an unspecified way.
    pub fn iter_shared(&self) -> Iter<'_> {
        Iter::new(self)
    }

    /// Returns an iterator over the keys `Dictionary`. The keys are each of type `Variant`. Each key references
    /// the original `Dictionary`, but instead of a `&`-reference to keys pairs as you might expect, the
    /// iterator returns a (cheap, shallow) copy of each key pair.
    ///
    /// Note that it's possible to modify the `Dictionary` through another reference while iterating
    /// over it. This will not result in unsoundness or crashes, but will cause the iterator to
    /// behave in an unspecified way.
    pub fn keys_shared(&self) -> Keys<'_> {
        Keys::new(self)
    }

    #[doc(hidden)]
    pub fn as_inner(&self) -> inner::InnerDictionary {
        inner::InnerDictionary::from_outer(self)
    }
}

/// Creates a Dictionary from the given iterator I over a (&K, &V) key-value pair.
/// Each key and value are converted to a Variant.
impl<'a, 'b, K, V, I> From<I> for Dictionary
where
    I: IntoIterator<Item = (&'a K, &'b V)>,
    K: ToVariant + 'a,
    V: ToVariant + 'b,
{
    fn from(iterable: I) -> Self {
        iterable
            .into_iter()
            .map(|(key, value)| (key.to_variant(), value.to_variant()))
            .collect()
    }
}

/// Inserts all key-values from the iterator into the dictionary,
/// replacing values with existing keys with new values returned
/// from the iterator.
impl<K: ToVariant, V: ToVariant> Extend<(K, V)> for Dictionary {
    fn extend<I: IntoIterator<Item = (K, V)>>(&mut self, iter: I) {
        for (k, v) in iter.into_iter() {
            self.set(k.to_variant(), v.to_variant())
        }
    }
}

impl<K: ToVariant, V: ToVariant> FromIterator<(K, V)> for Dictionary {
    fn from_iter<I: IntoIterator<Item = (K, V)>>(iter: I) -> Self {
        let mut dict = Dictionary::new();
        dict.extend(iter);
        dict
    }
}

impl_builtin_traits! {
    for Dictionary {
        Default => dictionary_construct_default;
        Drop => dictionary_destroy;
        PartialEq => dictionary_operator_equal;
    }
}

impl GodotFfi for Dictionary {
    ffi_methods! {
        type sys::GDExtensionTypePtr = *mut Opaque;
        fn from_sys;
        fn sys;
        fn write_sys;
    }

    unsafe fn from_sys_init(init_fn: impl FnOnce(sys::GDExtensionTypePtr)) -> Self {
        // Can't use uninitialized pointer -- Dictionary CoW implementation in C++ expects that on
        // assignment, the target CoW pointer is either initialized or nullptr

        let mut result = Self::default();
        init_fn(result.sys_mut());
        result
    }
}

impl fmt::Debug for Dictionary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.to_variant().stringify())
    }
}

/// Creates a new reference to the data in this dictionary. Changes to the original dictionary will be
/// reflected in the copy and vice versa.
///
/// To create a (mostly) independent copy instead, see [`Dictionary::duplicate_shallow()`] and
/// [`Dictionary::duplicate_deep()`].
impl Share for Dictionary {
    fn share(&self) -> Self {
        unsafe {
            Self::from_sys_init(|self_ptr| {
                let ctor = sys::builtin_fn!(dictionary_construct_copy);
                let args = [self.sys_const()];
                ctor(self_ptr, args.as_ptr());
            })
        }
    }
}

struct DictionaryIter<'a> {
    last_key: Option<Variant>,
    dictionary: &'a Dictionary,
    is_first: bool,
}

impl<'a> DictionaryIter<'a> {
    fn new(dictionary: &'a Dictionary) -> Self {
        Self {
            last_key: None,
            dictionary,
            is_first: true,
        }
    }

    fn call_init(dictionary: &Dictionary) -> Option<Variant> {
        // SAFETY:
        // `dictionary` is a valid `Dictionary` since we have a reference to it,
        //    so this will call the implementation for dictionaries.
        // `variant` is an initialized and valid `Variant`.
        let variant: Variant = Variant::nil();
        unsafe { Self::call_iter_fn(interface_fn!(variant_iter_init), dictionary, variant) }
    }

    fn call_next(dictionary: &Dictionary, last_key: Variant) -> Option<Variant> {
        // SAFETY:
        // `dictionary` is a valid `Dictionary` since we have a reference to it,
        //    so this will call the implementation for dictionaries.
        // `last_key` is an initialized and valid `Variant`, since we own a copy of it.
        unsafe { Self::call_iter_fn(interface_fn!(variant_iter_next), dictionary, last_key) }
    }

    /// # SAFETY:
    /// `iter_fn` must point to a valid function that interprets the parameters according to their type specification.
    unsafe fn call_iter_fn(
        iter_fn: unsafe extern "C" fn(
            sys::GDExtensionConstVariantPtr,
            sys::GDExtensionVariantPtr,
            *mut sys::GDExtensionBool,
        ) -> sys::GDExtensionBool,
        dictionary: &Dictionary,
        next_var: Variant,
    ) -> Option<Variant> {
        let dictionary = dictionary.to_variant();
        let mut valid: u8 = 0;

        let has_next = iter_fn(
            dictionary.var_sys(),
            next_var.var_sys(),
            addr_of_mut!(valid),
        );
        let valid = u8_to_bool(valid);
        let has_next = u8_to_bool(has_next);

        if has_next {
            assert!(valid);
            Some(next_var)
        } else {
            None
        }
    }

    fn next_key(&mut self) -> Option<Variant> {
        let new_key = if self.is_first {
            self.is_first = false;
            Self::call_init(self.dictionary)
        } else {
            Self::call_next(self.dictionary, self.last_key.take()?)
        };
        self.last_key = new_key.clone();
        new_key
    }

    fn next_key_value(&mut self) -> Option<(Variant, Variant)> {
        let key = self.next_key()?;
        if !self.dictionary.contains_key(key.clone()) {
            return None;
        }

        let value = self.dictionary.as_inner().get(key.clone(), Variant::nil());
        Some((key, value))
    }
}

/// An iterator over key-value pairs from a `Dictionary`.
///
/// See [Dictionary::iter_shared()] for more information about iteration over dictionaries.
pub struct Iter<'a> {
    iter: DictionaryIter<'a>,
}

impl<'a> Iter<'a> {
    fn new(dictionary: &'a Dictionary) -> Self {
        Self {
            iter: DictionaryIter::new(dictionary),
        }
    }

    /// Creates an iterator that will convert each `(Variant, Variant)` key-value pair into
    /// a `(K,V)` key-value pair, panicking upon failure to convert.
    pub fn typed<K: FromVariant, V: FromVariant>(self) -> TypedIter<'a, K, V> {
        TypedIter::from_untyped(self)
    }
}

impl<'a> Iterator for Iter<'a> {
    type Item = (Variant, Variant);

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next_key_value()
    }
}

/// An iterator over keys from a `Dictionary`.
///
/// See [Dictionary::keys_shared()] for more information about iteration over dictionaries.
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
    pub fn typed<K: FromVariant>(self) -> TypedKeys<'a, K> {
        TypedKeys::from_untyped(self)
    }
}
impl<'a> Iterator for Keys<'a> {
    type Item = Variant;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next_key()
    }
}

/// An iterator over key-value pairs from a `Dictionary` that will attempt to convert each
/// key-value pair into a `(K,V)`.
///
/// See [Dictionary::iter_shared()] for more information about iteration over dictionaries.
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

impl<'a, K: FromVariant, V: FromVariant> Iterator for TypedIter<'a, K, V> {
    type Item = (K, V);

    fn next(&mut self) -> Option<Self::Item> {
        self.iter
            .next_key_value()
            .map(|(key, value)| (K::from_variant(&key), V::from_variant(&value)))
    }
}

/// An iterator over keys from a `Dictionary` that will attempt to convert each key into a `K`.
///
/// See [Dictionary::iter_shared()] for more information about iteration over dictionaries.
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

impl<'a, K: FromVariant> Iterator for TypedKeys<'a, K> {
    type Item = K;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next_key().map(|k| K::from_variant(&k))
    }
}

fn u8_to_bool(u: u8) -> bool {
    match u {
        0 => false,
        1 => true,
        _ => panic!("Invalid boolean value {u}"),
    }
}

/// Creates a new dictionary with the given keys and values, the syntax mirrors
/// Godot's dictionary creation syntax.
///
/// Any value can be used as a key, but to use an expression you need to surround it
/// in `()` or `{}`.
///
/// Example
/// ```no_run
/// use godot::builtin::{dict, Variant};
///
/// let key = "my_key";
/// let d = dict! {
///     "key1": 10,
///     "another": Variant::nil(),
///     key: true,
///     (1 + 2): "final",
/// };
/// ```
#[macro_export]
macro_rules! dict {
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
