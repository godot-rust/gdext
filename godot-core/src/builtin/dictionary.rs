use godot_ffi as sys;

use crate::builtin::{inner, FromVariant, ToVariant, Variant, VariantConversionError};
use std::collections::{HashMap, HashSet};
use std::fmt;
use sys::{ffi_methods, interface_fn, GodotFfi};
use sys::types::*;

use super::Array;

/// Godot's `Dictionary` type.
///
/// The keys and values of the array are all `Variant`, so they can all be of different types.
/// 
/// Keys are assumed to be cheap to clone, this will usually be the case especially for 
/// value types and reference counted types.
///
/// # Safety
///
/// TODO: I suspect it is similar to the rules for array.
#[repr(C)]
pub struct Dictionary {
    opaque: OpaqueDictionary,
}

impl Dictionary {
    fn from_opaque(opaque: OpaqueDictionary) -> Self {
        Self { opaque }
    }

    /// Constructs an empty `Dictionary`
    pub fn new() -> Self {
        Self::default()
    }

    /// Removes all key-value pairs from the dictionary. Equivalent to `clear` in godot.
    pub fn clear(&mut self) {
        self.as_inner().clear()
    }

    /// Returns a deep copy of the dictionary. All nested array/dictionary keys and 
    /// values are duplicated as well.
    /// 
    /// Equivalent to `dictionary.duplicate(true)` in godot.
    pub fn duplicate_deep(&self) -> Self {
        self.as_inner().duplicate(true)
    }

    /// Returns a shallow copy of the dictionary. Nested array/dictionary keys and 
    /// values are not duplicated.
    /// 
    /// Equivalent to `dictionary.duplicate(false)` in godot.
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

    /// Returns the first key whose associated value is `value`, if one exists
    /// 
    /// Unlike in godot, this will return `None` if the key does not exist
    /// and `Some(nil)` the key is `null`
    pub fn find_key(&self, value: Variant) -> Option<Variant> {
        let key = self.as_inner().find_key(value);

        if !key.is_nil() || self.contains_key(key.clone()) {
            Some(key)
        } else {
            None
        }
    }

    /// Returns the value at the key in the dictionary, if there is
    /// one
    /// 
    /// Unlike `get` in godot, this will return `None` if there is
    /// no value with the given key. 
    pub fn get<K: ToVariant>(&self, key: K) -> Option<Variant> {
        let key = key.to_variant();
        if !self.contains_key(key.clone()) {
            return None;
        }

        Some(self.get_or_nil(key))
    }

    /// Returns the value at the key in the dictionary, or nil otherwise
    /// 
    /// This is equivalent to `get` in godot.
    pub fn get_or_nil<K: ToVariant>(&self, key: K) -> Variant {
        self.as_inner().get(key.to_variant(), Variant::nil())
    }

    /// Returns `true` if the dictionary contains the given key
    /// 
    /// This is equivalent to `has` in godot
    pub fn contains_key<K: ToVariant>(&self, key: K) -> bool {
        let key = key.to_variant();
        self.as_inner().has(key)
    }

    /// Returns `true` if the dictionary contains all the given keys
    /// 
    /// This is equivalent to `has_all` in godot
    pub fn contains_all_keys(&self, keys: Array) -> bool {
        self.as_inner().has_all(keys)
    }

    /// Returns a 32-bit integer hash value representing the dictionary and its contents.
    pub fn hash(&self) -> u32 {
        self.as_inner().hash().try_into().unwrap()
    }

    /// Creates a new `Array` containing all the keys currently in the dictionary
    pub fn keys(&self) -> Array {
        self.as_inner().keys()
    }

    /// Creates a new `Array` containing all the values currently in the dictionary
    pub fn values(&self) -> Array {
        self.as_inner().values()
    }

    /// Returns true if the dictionary is empty
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

    /// Returns the number of entries in the dictionary
    /// 
    /// This is equivalent to `size` in godot.
    pub fn len(&self) -> usize {
        self.as_inner().size().try_into().unwrap()
    }

    /// Get the pointer corresponding to the given key in the dictionary,
    /// this pointer is null if there is no value at the given key.
    pub fn get_ptr<K: ToVariant>(&self, key: K) -> *const Variant {
        let key = key.to_variant();
        unsafe {
            (interface_fn!(dictionary_operator_index_const))(self.sys_const(), key.var_sys_const()) as *const Variant
        }
    }

    /// Get the pointer corresponding to the given key in the dictionary,
    /// if there exists no value at the given key then a new one is created
    /// and initialized to a nil variant.
    pub fn get_ptr_mut<K: ToVariant>(&mut self, key: K) -> *mut Variant {
        let key = key.to_variant();
        unsafe {
            let ptr =
                (interface_fn!(dictionary_operator_index))(self.sys_mut(), key.var_sys_const()) as *mut Variant;
            // dictionary_operator_index initializes the value so it wont be null
            assert!(!ptr.is_null());
            // i think it might be safe to turn this into a &mut Variant?
            ptr
        }
    }

    /// Insert a value at the given key, returning the value
    /// that previously was at that key if there was one.
    pub fn insert<K: ToVariant>(&mut self, key: K, value: Variant) -> Option<Variant> {
        let key = key.to_variant();
        let old_value = self.remove(key.clone());
        self.set(key, value);
        old_value
    }

    /// Set a key to a given value
    pub fn set<K: ToVariant>(&mut self, key: K, value: Variant) {
        let key = key.to_variant();
        unsafe {
            *self.get_ptr_mut(key) = value;
        }
    }

    /// Convert this dictionary to a strongly typed rust `HashMap`. If the conversion
    /// fails for any key or value, an error is returned. 
    pub fn try_to_hashmap<K: FromVariant + Eq + std::hash::Hash, V: FromVariant>(
        &self,
    ) -> Result<HashMap<K, V>, VariantConversionError> {
        let mut map = HashMap::new();
        for key in self.keys().into_iter() {
            map.insert(key.try_to()?, self.get(key).unwrap().try_to()?);
        }
        Ok(map)
    }

    /// Convert the keys of this dictionary to a strongly typed rust `HashSet`. If the 
    /// conversion fails for any key, an error is returned. 
    pub fn try_to_hashset<K: FromVariant + Eq + std::hash::Hash>(
        &self,
    ) -> Result<HashSet<K>, VariantConversionError> {
        Ok(self.keys().try_to_vec::<K>()?.into_iter().collect())
    }

    #[doc(hidden)]
    pub fn as_inner(&self) -> inner::InnerDictionary {
        inner::InnerDictionary::from_outer(self)
    }
}

/// Creates a `Dictionary` from the given `HashMap`. Each key and value are 
/// converted to a `Variant`.
impl<K: ToVariant, V: ToVariant> From<HashMap<K, V>> for Dictionary {
    fn from(map: HashMap<K, V>) -> Self {
        let mut dict = Dictionary::new();
        for (k, v) in map.into_iter() {
            dict.insert(k.to_variant(), v.to_variant());
        }
        dict
    }
}

/// Creates a `Dictionary` from the given `HashSet`. Each key is converted
/// to a `Variant`, and the values are all `true`.
impl<K: ToVariant> From<HashSet<K>> for Dictionary {
    fn from(set: HashSet<K>) -> Self {
        let mut dict = Dictionary::new();
        for k in set.into_iter() {
            dict.insert(k.to_variant(), true.to_variant());
        }
        dict
    }
}

/// Inserts all key-values from the iterator into the dictionary,
/// replacing values with existing keys with new values returned
/// from the iterator.
impl<K: ToVariant, V: ToVariant> Extend<(K, V)> for Dictionary {
    fn extend<T: IntoIterator<Item = (K, V)>>(&mut self, iter: T) {
        for (k, v) in iter.into_iter() {
            self.insert(k.to_variant(), v.to_variant());
        }
    }
}

impl<K: ToVariant, V: ToVariant> FromIterator<(K, V)> for Dictionary {
    fn from_iter<T: IntoIterator<Item = (K, V)>>(iter: T) -> Self {
        let mut dict = Dictionary::new();
        for (k, v) in iter.into_iter() {
            dict.insert(k.to_variant(), v.to_variant());
        }
        dict
    }
}

impl_builtin_traits! {
    for Dictionary {
        Default => dictionary_construct_default;
        Clone => dictionary_construct_copy;
        Drop => dictionary_destroy;
        Eq => dictionary_operator_equal;
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

/// Creates a new dictionary with the given keys and values, the syntax mirrors
/// godot's dictionary creation syntax.
/// 
/// Currently only literal keys are supported. But any expression whose result 
/// can be converted with `Variant::from()` may be used.
/// 
/// Example
/// ```rust, no_run
/// # #[macro_use] extern crate godot_core;
/// # fn main() {
/// let d = dict! {
///     "key1": 10,
///     "another": Variant::nil(),
///     "true": true,
///     "final": "final",
/// }
/// # }
/// ``` 
#[macro_export]
macro_rules! dict {
    () => {
        ::godot::builtin::Dictionary::new()
    };
    ($($key:literal: $value:expr),+ $(,)?) => {
        {
            let mut d = ::godot::builtin::Dictionary::new();
            $(d.set($key, ::godot::builtin::Variant::from($value));)*
            d
        }
    };
}
