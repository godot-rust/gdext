use godot_ffi as sys;

use crate::builtin::{inner, FromVariant, ToVariant, Variant, VariantConversionError};
use crate::obj::Share;
use std::collections::{HashMap, HashSet};
use std::fmt;
use sys::types::*;
use sys::{ffi_methods, interface_fn, GodotFfi};

use super::Array;

/// Godot's `Dictionary` type.
///
/// The keys and values of the array are all `Variant`, so they can all be of different types.
///
/// Keys are assumed to be cheap to clone, this will usually be the case especially for
/// value types and reference counted types.
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

    /// Removes all key-value pairs from the dictionary. Equivalent to `clear` in godot.
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
    /// Equivalent to `dictionary.duplicate(true)` in godot.
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
    /// Equivalent to `dictionary.duplicate(false)` in godot.
    pub fn duplicate_shallow(&self) -> Self {
        self.as_inner().duplicate(false)
    }

    /// Removes a key from the map, and returns the value associated with
    /// the key if the key was in the dictionary.
    pub fn remove(&mut self, key: impl ToVariant) -> Option<Variant> {
        let key = key.to_variant();
        let old_value = self.get(key.clone());
        self.as_inner().erase(key);
        old_value
    }

    /// Returns the first key whose associated value is `value`, if one exists.
    ///
    /// Unlike in godot, this will return `None` if the key does not exist
    /// and `Some(nil)` the key is `null`.
    pub fn find_key_by_value(&self, value: impl ToVariant) -> Option<Variant> {
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
    /// Unlike `get` in godot, this will return `None` if there is
    /// no value with the given key.
    pub fn get(&self, key: impl ToVariant) -> Option<Variant> {
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
    /// This is equivalent to `get` in godot.
    pub fn get_or_nil(&self, key: impl ToVariant) -> Variant {
        self.as_inner().get(key.to_variant(), Variant::nil())
    }

    /// Returns `true` if the dictionary contains the given key.
    ///
    /// This is equivalent to `has` in godot.
    pub fn contains_key(&self, key: impl ToVariant) -> bool {
        let key = key.to_variant();
        self.as_inner().has(key)
    }

    /// Returns `true` if the dictionary contains all the given keys.
    ///
    /// This is equivalent to `has_all` in godot.
    pub fn contains_all_keys(&self, keys: Array) -> bool {
        self.as_inner().has_all(keys)
    }

    /// Returns a 32-bit integer hash value representing the dictionary and its contents.
    pub fn hash(&self) -> u32 {
        self.as_inner().hash().try_into().unwrap()
    }

    /// Creates a new `Array` containing all the keys currently in the dictionary.
    pub fn keys(&self) -> Array {
        self.as_inner().keys()
    }

    /// Creates a new `Array` containing all the values currently in the dictionary.
    pub fn values(&self) -> Array {
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
    /// This is equivalent to `merge` in godot.
    pub fn extend_dictionary(&mut self, other: Self, overwrite: bool) {
        self.as_inner().merge(other, overwrite)
    }

    /// Returns the number of entries in the dictionary.
    ///
    /// This is equivalent to `size` in godot.
    pub fn len(&self) -> usize {
        self.as_inner().size().try_into().unwrap()
    }

    /// Get the pointer corresponding to the given key in the dictionary,
    /// this pointer is null if there is no value at the given key.
    #[allow(dead_code)] // TODO: remove function if it turns out i'll never actually get any use out of it
    fn get_ptr(&self, key: impl ToVariant) -> *const Variant {
        let key = key.to_variant();
        unsafe {
            (interface_fn!(dictionary_operator_index_const))(self.sys_const(), key.var_sys_const())
                as *const Variant
        }
    }

    /// Get the pointer corresponding to the given key in the dictionary,
    /// if there exists no value at the given key then a new one is created
    /// and initialized to a nil variant.
    fn get_ptr_mut(&mut self, key: impl ToVariant) -> *mut Variant {
        let key = key.to_variant();
        unsafe {
            let ptr =
                (interface_fn!(dictionary_operator_index))(self.sys_mut(), key.var_sys_const())
                    as *mut Variant;
            // dictionary_operator_index initializes the value so it wont be null
            assert!(!ptr.is_null());
            // i think it might be safe to turn this into a &mut Variant?
            ptr
        }
    }

    /// Insert a value at the given key, returning the value
    /// that previously was at that key if there was one.
    pub fn insert(&mut self, key: impl ToVariant, value: impl ToVariant) -> Option<Variant> {
        let key = key.to_variant();
        let old_value = self.get(key.clone());
        self.set(key, value);
        old_value
    }

    /// Set a key to a given value
    pub fn set(&mut self, key: impl ToVariant, value: impl ToVariant) {
        let key = key.to_variant();
        unsafe {
            *self.get_ptr_mut(key) = value.to_variant();
        }
    }

    #[doc(hidden)]
    pub fn as_inner(&self) -> inner::InnerDictionary {
        inner::InnerDictionary::from_outer(self)
    }
}

/// Creates a `Dictionary` from the given `T`. Each key and value are
/// converted to a `Variant`.
impl<'a, 'b, K, V, T> From<T> for Dictionary
where
    T: IntoIterator<Item = (&'a K, &'b V)>,
    K: ToVariant + 'a,
    V: ToVariant + 'b,
{
    fn from(iterable: T) -> Self {
        iterable.into_iter().map(|(key,value)| (key.to_variant(), value.to_variant())).collect()
    }
}

/// Convert this dictionary to a strongly typed rust `HashMap`. If the conversion
/// fails for any key or value, an error is returned.
///
/// Will be replaced by a proper iteration implementation.
impl<K: FromVariant + Eq + std::hash::Hash, V: FromVariant> TryFrom<&Dictionary> for HashMap<K, V> {
    type Error = VariantConversionError;

    fn try_from(dictionary: &Dictionary) -> Result<Self, Self::Error> {
        // TODO: try to panic or something if modified while iterating
        // Though probably better to fix when implementing iteration proper
        dictionary
            .keys()
            .iter_shared()
            .zip(dictionary.values().iter_shared())
            .map(|(key, value)| Ok((K::try_from_variant(&key)?, V::try_from_variant(&value)?)))
            .collect()
    }
}

/// Convert the keys of this dictionary to a strongly typed rust `HashSet`. If the
/// conversion fails for any key, an error is returned.
impl<K: FromVariant + Eq + std::hash::Hash> TryFrom<&Dictionary> for HashSet<K> {
    type Error = VariantConversionError;

    fn try_from(dictionary: &Dictionary) -> Result<Self, Self::Error> {
        // TODO: try to panic or something if modified while iterating
        // Though probably better to fix when implementing iteration proper
        dictionary
            .keys()
            .iter_shared()
            .map(|key| K::try_from_variant(&key))
            .collect()
    }
}

/// Inserts all key-values from the iterator into the dictionary,
/// replacing values with existing keys with new values returned
/// from the iterator.
impl<K: ToVariant, V: ToVariant> Extend<(K, V)> for Dictionary {
    fn extend<T: IntoIterator<Item = (K, V)>>(&mut self, iter: T) {
        for (k, v) in iter.into_iter() {
            self.set(k.to_variant(), v.to_variant())
        }
    }
}

impl<K: ToVariant, V: ToVariant> FromIterator<(K, V)> for Dictionary {
    fn from_iter<T: IntoIterator<Item = (K, V)>>(iter: T) -> Self {
        let mut dict = Dictionary::new();
        for (k, v) in iter.into_iter() {
            dict.set(k.to_variant(), v.to_variant())
        }
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
    ffi_methods! { type sys::GDExtensionTypePtr = *mut Opaque; .. }
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

/// Creates a new dictionary with the given keys and values, the syntax mirrors
/// godot's dictionary creation syntax.
///
/// Any value can be used as a key, but to use an expression you need to surround it
/// in `()` or `{}`
///
/// Example
/// ```rust, no_run
/// # #[macro_use] extern crate godot_core;
/// # fn main() {
/// let key = "my_key";
/// let d = dict! {
///     "key1": 10,
///     "another": Variant::nil(),
///     key: true,
///     (1 + 2): "final",
/// }
/// # }
/// ```
#[macro_export]
macro_rules! dict {
    ($($key:tt: $value:expr),* $(,)?) => {
        {
            let mut d = $crate::builtin::Dictionary::new();
            $(
                // otherwise `(1 + 2): true` would complain even though you can't write
                // 1 + 2: true
                #[allow(unused_parens)] 
                d.set($key, $value);
            )*
            d
        }
    };
}
