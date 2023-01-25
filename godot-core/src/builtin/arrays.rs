/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use godot_ffi as sys;

use crate::builtin::{inner, FromVariant, ToVariant, Variant, VariantConversionError};
use std::fmt;
use std::marker::PhantomData;
use sys::types::*;
use sys::{ffi_methods, interface_fn, GodotFfi};

/// Godot's `Array` type.
///
/// This is a variant array, meaning it contains `Variant`s which may be of different types even
/// within the same array.
///
/// Unlike GDScript, all indices and sizes are unsigned, so negative indices are not supported.
///
/// # Safety
///
/// Usage is safe if the `Array` is used on a single thread only. Concurrent reads on different
/// threads are also safe, but any writes must be externally synchronized. The Rust compiler will
/// enforce this as long as you use only Rust threads, but it cannot protect against concurrent
/// modification on other threads (e.g. created through GDScript).
#[repr(C)]
pub struct Array {
    opaque: sys::types::OpaqueArray,
}

impl_builtin_stub!(PackedByteArray, OpaquePackedByteArray);
impl_builtin_stub!(PackedColorArray, OpaquePackedColorArray);
impl_builtin_stub!(PackedFloat32Array, OpaquePackedFloat32Array);
impl_builtin_stub!(PackedFloat64Array, OpaquePackedFloat64Array);
impl_builtin_stub!(PackedInt32Array, OpaquePackedInt32Array);
impl_builtin_stub!(PackedInt64Array, OpaquePackedInt64Array);
impl_builtin_stub!(PackedStringArray, OpaquePackedStringArray);
impl_builtin_stub!(PackedVector2Array, OpaquePackedVector2Array);
impl_builtin_stub!(PackedVector3Array, OpaquePackedVector3Array);

impl_builtin_froms!(Array;
    PackedByteArray => array_from_packed_byte_array,
    PackedColorArray => array_from_packed_color_array,
    PackedFloat32Array => array_from_packed_float32_array,
    PackedFloat64Array => array_from_packed_float64_array,
    PackedInt32Array => array_from_packed_int32_array,
    PackedInt64Array => array_from_packed_int64_array,
    PackedStringArray => array_from_packed_string_array,
    PackedVector2Array => array_from_packed_vector2_array,
    PackedVector3Array => array_from_packed_vector3_array,
);

impl_builtin_froms!(PackedByteArray; Array => packed_byte_array_from_array);
impl_builtin_froms!(PackedColorArray; Array => packed_color_array_from_array);
impl_builtin_froms!(PackedFloat32Array; Array => packed_float32_array_from_array);
impl_builtin_froms!(PackedFloat64Array; Array => packed_float64_array_from_array);
impl_builtin_froms!(PackedInt32Array; Array => packed_int32_array_from_array);
impl_builtin_froms!(PackedInt64Array; Array => packed_int64_array_from_array);
impl_builtin_froms!(PackedStringArray; Array => packed_string_array_from_array);
impl_builtin_froms!(PackedVector2Array; Array => packed_vector2_array_from_array);
impl_builtin_froms!(PackedVector3Array; Array => packed_vector3_array_from_array);

impl Array {
    fn from_opaque(opaque: sys::types::OpaqueArray) -> Self {
        Self { opaque }
    }
}

// This impl relies on `InnerArray` which is not (yet) available in unit tests
#[cfg(not(any(gdext_test, doctest)))]
impl Array {
    /// Constructs an empty `Array`.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the number of elements in the array. Equivalent of `size()` in Godot.
    pub fn len(&self) -> usize {
        to_usize(self.as_inner().size())
    }

    /// Returns `true` if the array is empty.
    pub fn is_empty(&self) -> bool {
        self.as_inner().is_empty()
    }

    /// Returns a 32-bit integer hash value representing the array and its contents.
    ///
    /// Note: Arrays with equal content will always produce identical hash values. However, the
    /// reverse is not true. Returning identical hash values does not imply the arrays are equal,
    /// because different arrays can have identical hash values due to hash collisions.
    pub fn hash(&self) -> u32 {
        // The GDExtension interface only deals in `i64`, but the engine's own `hash()` function
        // actually returns `uint32_t`.
        self.as_inner().hash().try_into().unwrap()
    }

    /// Converts this array to a strongly typed Rust vector. If the conversion from `Variant` fails
    /// for any element, an error is returned.
    pub fn try_to_vec<T: FromVariant>(&self) -> Result<Vec<T>, VariantConversionError> {
        let len = self.len();
        let mut vec = Vec::with_capacity(len);
        let ptr = self.ptr(0);
        for offset in 0..to_isize(len) {
            // SAFETY: Arrays are stored contiguously in memory, so we can use pointer arithmetic
            // instead of going through `array_operator_index_const` for every index.
            let element = unsafe { T::try_from_variant(&*ptr.offset(offset))? };
            vec.push(element);
        }
        Ok(vec)
    }

    /// Implements iteration over an `Array` by reference, but returns (cheap) copies of the
    /// `Variant`s in the array.
    pub fn iter(&self) -> ArrayIterator<'_> {
        ArrayIterator {
            start: self.ptr(0),
            len: self.len(),
            next_idx: 0,
            _phantom: PhantomData,
        }
    }

    /// Clears the array, removing all elements.
    pub fn clear(&mut self) {
        self.as_inner().clear();
    }

    /// Resizes the array to contain a different number of elements. If the new size is smaller,
    /// elements are removed from the end. If the new size is larger, new elements are set to
    /// [`Variant::nil()`].
    pub fn resize(&mut self, size: usize) {
        self.as_inner().resize(to_i64(size));
    }

    // TODO: find out why this segfaults (even on an empty array, regardless of deep = true/false)
    // /// Returns a deep copy of the array. All nested arrays and dictionaries are duplicated and
    // /// will not be shared with the original array. Note that any `Object`-derived elements will
    // /// still be shallow copied.
    // ///
    // /// To create a shallow copy, use `clone()` instead.
    // pub fn duplicate_deep(&self) -> Self {
    //     self.as_inner().duplicate(true)
    // }

    /// Returns the value at the specified index as a `Variant`. To convert to a specific type, use
    /// the available conversion methods on `Variant`, such as [`Variant::try_to`] or
    /// [`Variant::to`].
    ///
    /// # Panics
    ///
    /// If `index` is out of bounds.
    pub fn get(&self, index: usize) -> Variant {
        let ptr = self.ptr(index);
        // SAFETY: `ptr` just verified that the index is not out of bounds.
        unsafe { (*ptr).clone() }
    }

    /// Returns the first element in the array, or `None` if the array is empty. Equivalent of
    /// `front()` in GDScript.
    pub fn first(&self) -> Option<Variant> {
        (!self.is_empty()).then(|| self.as_inner().front())
    }

    /// Returns the last element in the array, or `None` if the array is empty. Equivalent of
    /// `back()` in GDScript.
    pub fn last(&self) -> Option<Variant> {
        (!self.is_empty()).then(|| self.as_inner().back())
    }

    /// Finds the index of an existing value in a sorted array using binary search. Equivalent of
    /// `bsearch` in GDScript.
    ///
    /// If the value is not present in the array, returns the insertion index that would maintain
    /// sorting order.
    ///
    /// Calling `binary_search` on an unsorted array results in unspecified behavior.
    pub fn binary_search(&self, value: Variant) -> usize {
        to_usize(self.as_inner().bsearch(value, true))
    }

    /// Returns the number of times a value is in the array.
    pub fn count(&self, value: Variant) -> usize {
        to_usize(self.as_inner().count(value))
    }

    /// Returns `true` if the array contains the given value. Equivalent of `has` in GDScript.
    pub fn contains(&self, value: Variant) -> bool {
        self.as_inner().has(value)
    }

    /// Searches the array for the first occurrence of a value and returns its index, or `None` if
    /// not found. Starts searching at index `from`; pass `None` to search the entire array.
    pub fn find(&self, value: Variant, from: Option<usize>) -> Option<usize> {
        let from = to_i64(from.unwrap_or(0));
        let index = self.as_inner().find(value, from);
        if index >= 0 {
            Some(index.try_into().unwrap())
        } else {
            None
        }
    }

    /// Searches the array backwards for the last occurrence of a value and returns its index, or
    /// `None` if not found. Starts searching at index `from`; pass `None` to search the entire
    /// array.
    pub fn rfind(&self, value: Variant, from: Option<usize>) -> Option<usize> {
        let from = from.map(to_i64).unwrap_or(-1);
        let index = self.as_inner().rfind(value, from);
        // It's not documented, but `rfind` returns -1 if not found.
        if index >= 0 {
            Some(to_usize(index))
        } else {
            None
        }
    }

    /// Returns the minimum value contained in the array if all elements are of comparable types.
    /// If the elements can't be compared or the array is empty, `None` is returned.
    pub fn min(&self) -> Option<Variant> {
        let min = self.as_inner().min();
        (!min.is_nil()).then_some(min)
    }

    /// Returns the maximum value contained in the array if all elements are of comparable types.
    /// If the elements can't be compared or the array is empty, `None` is returned.
    pub fn max(&self) -> Option<Variant> {
        let max = self.as_inner().max();
        (!max.is_nil()).then_some(max)
    }

    /// Returns a random element from the array, or `None` if it is empty.
    pub fn pick_random(&self) -> Option<Variant> {
        (!self.is_empty()).then(|| self.as_inner().pick_random())
    }

    /// Sets the value at the specified index as a `Variant`. To convert a specific type (which
    /// implements `ToVariant`) to a variant, call [`ToVariant::to_variant`] on it.
    ///
    /// # Panics
    ///
    /// If `index` is out of bounds.
    pub fn set(&mut self, index: usize, value: Variant) {
        let ptr_mut = self.ptr_mut(index);
        // SAFETY: `ptr_mut` just checked that the index is not out of bounds.
        unsafe {
            *ptr_mut = value;
        }
    }

    /// Appends an element to the end of the array. Equivalent of `append` and `push_back` in
    /// GDScript.
    pub fn push(&mut self, value: Variant) {
        self.as_inner().push_back(value);
    }

    /// Adds an element at the beginning of the array. See also `push`.
    ///
    /// Note: On large arrays, this method is much slower than `push` as it will move all the
    /// array's elements. The larger the array, the slower `push_front` will be.
    pub fn push_front(&mut self, value: Variant) {
        self.as_inner().push_front(value);
    }

    /// Removes and returns the last element of the array. Returns `None` if the array is empty.
    /// Equivalent of `pop_back` in GDScript.
    pub fn pop(&mut self) -> Option<Variant> {
        (!self.is_empty()).then(|| self.as_inner().pop_back())
    }

    /// Removes and returns the first element of the array. Returns `None` if the array is empty.
    ///
    /// Note: On large arrays, this method is much slower than `pop` as it will move all the
    /// array's elements. The larger the array, the slower `pop_front` will be.
    pub fn pop_front(&mut self) -> Option<Variant> {
        (!self.is_empty()).then(|| self.as_inner().pop_front())
    }

    /// Inserts a new element at a given index in the array. The index must be valid, or at the end
    /// of the array (`index == len()`).
    ///
    /// Note: On large arrays, this method is much slower than `push` as it will move all the
    /// array's elements after the inserted element. The larger the array, the slower `insert` will
    /// be.
    pub fn insert(&mut self, index: usize, value: Variant) {
        let len = self.len();
        assert!(
            index <= len,
            "Array insertion index {} is out of bounds: length is {}",
            index,
            len
        );
        self.as_inner().insert(to_i64(index), value);
    }

    /// Removes and returns the element at the specified index. Equivalent of `pop_at` in GDScript.
    ///
    /// On large arrays, this method is much slower than `pop_back` as it will move all the array's
    /// elements after the removed element. The larger the array, the slower `remove` will be.
    ///
    /// # Panics
    ///
    /// If `index` is out of bounds.
    pub fn remove(&mut self, index: usize) -> Variant {
        self.check_bounds(index);
        self.as_inner().pop_at(to_i64(index))
    }

    /// Removes the first occurrence of a value from the array. If the value does not exist in the
    /// array, nothing happens. To remove an element by index, use `remove` instead.
    ///
    /// On large arrays, this method is much slower than `pop_back` as it will move all the array's
    /// elements after the removed element. The larger the array, the slower `remove` will be.
    pub fn erase(&mut self, value: Variant) {
        self.as_inner().erase(value);
    }

    /// Assigns the given value to all elements in the array. This can be used together with
    /// `resize` to create an array with a given size and initialized elements.
    pub fn fill(&mut self, value: Variant) {
        self.as_inner().fill(value);
    }

    /// Appends another array at the end of this array. Equivalent of `append_array` in GDScript.
    pub fn extend_array(&mut self, other: Array) {
        self.as_inner().append_array(other);
    }

    /// Reverses the order of the elements in the array.
    pub fn reverse(&mut self) {
        self.as_inner().reverse();
    }

    /// Sorts the array.
    ///
    /// Note: The sorting algorithm used is not
    /// [stable](https://en.wikipedia.org/wiki/Sorting_algorithm#Stability). This means that values
    /// considered equal may have their order changed when using `sort_unstable`.
    pub fn sort_unstable(&mut self) {
        self.as_inner().sort();
    }

    /// Shuffles the array such that the items will have a random order. This method uses the
    /// global random number generator common to methods such as `randi`. Call `randomize` to
    /// ensure that a new seed will be used each time if you want non-reproducible shuffling.
    pub fn shuffle(&mut self) {
        self.as_inner().shuffle();
    }

    /// Asserts that the given index refers to an existing element.
    ///
    /// # Panics
    ///
    /// If `index` is out of bounds.
    fn check_bounds(&self, index: usize) {
        let len = self.len();
        assert!(
            index < len,
            "Array index {} is out of bounds: length is {}",
            index,
            len
        );
    }

    fn ptr(&self, index: usize) -> *const Variant {
        if self.is_empty() {
            std::ptr::null()
        } else {
            self.check_bounds(index);
            // SAFETY: We just checked that the index is not out of bounds.
            let ptr = unsafe {
                let item_ptr: sys::GDExtensionVariantPtr =
                    (interface_fn!(array_operator_index_const))(self.sys(), to_i64(index));
                item_ptr as *const Variant
            };
            assert!(!ptr.is_null());
            ptr
        }
    }

    fn ptr_mut(&self, index: usize) -> *mut Variant {
        if self.is_empty() {
            std::ptr::null_mut()
        } else {
            self.check_bounds(index);
            // SAFETY: We just checked that the index is not out of bounds.
            let ptr = unsafe {
                let item_ptr: sys::GDExtensionVariantPtr =
                    (interface_fn!(array_operator_index))(self.sys(), to_i64(index));
                item_ptr as *mut Variant
            };
            assert!(!ptr.is_null());
            ptr
        }
    }

    #[doc(hidden)]
    pub fn as_inner(&self) -> inner::InnerArray {
        inner::InnerArray::from_outer(self)
    }
}

/// Creates an `Array` from the given Rust array. Each element is converted to a `Variant`.
#[cfg(not(any(gdext_test, doctest)))]
impl<T: ToVariant, const N: usize> From<&[T; N]> for Array {
    fn from(arr: &[T; N]) -> Self {
        Self::from(&arr[..])
    }
}

/// Creates an `Array` from the given slice. Each element is converted to a `Variant`.
#[cfg(not(any(gdext_test, doctest)))]
impl<T: ToVariant> From<&[T]> for Array {
    fn from(slice: &[T]) -> Self {
        let mut array = Self::new();
        let len = slice.len();
        if len == 0 {
            return array;
        }
        array.resize(len);
        let ptr = array.ptr_mut(0);
        for (i, element) in slice.iter().enumerate() {
            // SAFETY: The array contains exactly `len` elements, stored contiguously in memory.
            unsafe {
                *ptr.offset(to_isize(i)) = element.to_variant();
            }
        }
        array
    }
}

/// Creates an `Array` from an iterator. Each element is converted to a `Variant`.
#[cfg(not(any(gdext_test, doctest)))]
impl<T: ToVariant> FromIterator<T> for Array {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let mut array = Array::new();
        array.extend(iter);
        array
    }
}

/// Extends an `Array` with the contents of an iterator. Each element is converted to a `Variant`.
#[cfg(not(any(gdext_test, doctest)))]
impl<T: ToVariant> Extend<T> for Array {
    fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
        // Unfortunately the GDExtension API does not offer the equivalent of `Vec::reserve`.
        // Otherwise we could use it to pre-allocate based on `iter.size_hint()`.
        //
        // A faster implementation using `resize()` and direct pointer writes might still be
        // possible.
        for item in iter.into_iter() {
            self.push(item.to_variant());
        }
    }
}

/// See [`Array::iter()`].
#[cfg(not(any(gdext_test, doctest)))]
impl<'a> IntoIterator for &'a Array {
    type Item = Variant;
    type IntoIter = ArrayIterator<'a>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

pub struct ArrayIterator<'a> {
    start: *const Variant,
    len: usize,
    next_idx: usize,
    _phantom: PhantomData<&'a Array>,
}

impl<'a> Iterator for ArrayIterator<'a> {
    type Item = Variant;

    fn next(&mut self) -> Option<Self::Item> {
        if self.next_idx < self.len {
            let offset = to_isize(self.next_idx);
            self.next_idx += 1;
            unsafe { Some((*self.start.offset(offset)).clone()) }
        } else {
            None
        }
    }
}

impl fmt::Debug for Array {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // Going through `Variant` because there doesn't seem to be a direct way.
        write!(f, "{:?}", self.to_variant().stringify())
    }
}

impl_builtin_traits! {
    for Array {
        Default => array_construct_default;
        Clone => array_construct_copy;
        Drop => array_destroy;
    }
}

impl GodotFfi for Array {
    ffi_methods! { type sys::GDExtensionTypePtr = *mut Opaque; .. }
}

fn to_i64(i: usize) -> i64 {
    i.try_into().unwrap()
}

fn to_usize(i: i64) -> usize {
    i.try_into().unwrap()
}

fn to_isize(i: usize) -> isize {
    i.try_into().unwrap()
}

#[repr(C)]
pub struct TypedArray<T> {
    opaque: OpaqueArray,
    _phantom: PhantomData<T>,
}
impl<T> TypedArray<T> {
    fn from_opaque(opaque: OpaqueArray) -> Self {
        Self {
            opaque,
            _phantom: PhantomData,
        }
    }
}

impl<T> Clone for TypedArray<T> {
    fn clone(&self) -> Self {
        unsafe {
            Self::from_sys_init(|self_ptr| {
                let ctor = ::godot_ffi::builtin_fn!(array_construct_copy);
                let args = [self.sys_const()];
                ctor(self_ptr, args.as_ptr());
            })
        }
    }
}

// TODO enable this:
// impl_builtin_traits! {
//     for TypedArray<T> {
//         Clone => array_construct_copy;
//         Drop => array_destroy;
//     }
// }

impl<T> GodotFfi for TypedArray<T> {
    ffi_methods! { type sys::GDExtensionTypePtr = *mut Opaque; .. }
}

impl<T> Drop for TypedArray<T> {
    fn drop(&mut self) {
        unsafe {
            let destructor = sys::builtin_fn!(array_destroy @1);
            destructor(self.sys_mut());
        }
    }
}

impl<T: FromVariant> TypedArray<T> {
    pub fn get(&self, index: i64) -> Option<T> {
        unsafe {
            let ptr = (interface_fn!(array_operator_index))(self.sys(), index);
            let v = Variant::from_var_sys(ptr);
            T::try_from_variant(&v).ok()
        }
    }
}
