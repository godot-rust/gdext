/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::fmt;
use std::marker::PhantomData;

use crate::builtin::*;
use crate::meta::error::{ConvertError, FromGodotError, FromVariantError};
use crate::meta::{
    ArrayElement, ArrayTypeInfo, FromGodot, GodotConvert, GodotFfiVariant, GodotType, ToGodot,
};
use crate::obj::EngineEnum;
use crate::registry::property::{
    builtin_type_string, Export, PropertyHintInfo, TypeStringHint, Var,
};
use godot_ffi as sys;
use sys::{ffi_methods, interface_fn, GodotFfi};

/// Godot's `Array` type.
///
/// Unlike GDScript, all indices and sizes are unsigned, so negative indices are not supported.
///
/// # Typed arrays
///
/// Godot's `Array` can be either typed or untyped.
///
/// An untyped array can contain any kind of [`Variant`], even different types in the same array.
/// We represent this in Rust as `VariantArray`, which is just a type alias for `Array<Variant>`.
///
/// Godot also supports typed arrays, which are also just `Variant` arrays under the hood, but with
/// runtime checks, so that no values of the wrong type are inserted into the array. We represent this as
/// `Array<T>`, where the type `T` must implement `ArrayElement`. Some types like `Array<T>` cannot
/// be stored inside arrays, as Godot prevents nesting.
///
/// # Reference semantics
///
/// Like in GDScript, `Array` acts as a reference type: multiple `Array` instances may
/// refer to the same underlying array, and changes to one are visible in the other.
///
/// To create a copy that shares data with the original array, use [`Clone::clone()`].
/// If you want to create a copy of the data, use [`duplicate_shallow()`][Self::duplicate_shallow]
/// or [`duplicate_deep()`][Self::duplicate_deep].
///
/// # Typed array example
///
/// ```no_run
/// # use godot::prelude::*;
/// // Create typed Array<i64> and add values.
/// let mut array = Array::new();
/// array.push(10);
/// array.push(20);
/// array.push(30);
///
/// // Or create the same array in a single expression.
/// let array = array![10, 20, 30];
///
/// // Access elements.
/// let value: i64 = array.at(0); // 10
/// let maybe: Option<i64> = array.get(3); // None
///
/// // Iterate over i64 elements.
/// for value in array.iter_shared() {
///    println!("{value}");
/// }
///
/// // Clone array (shares the reference), and overwrite elements through clone.
/// let mut cloned = array.clone();
/// cloned.set(0, 50); // [50, 20, 30]
/// cloned.remove(1);  // [50, 30]
/// cloned.pop();      // [50]
///
/// // Changes will be reflected in the original array.
/// assert_eq!(array.len(), 1);
/// assert_eq!(array.front(), Some(50));
/// ```
///
/// # Untyped array example
///
/// ```no_run
/// # use godot::prelude::*;
/// // VariantArray allows dynamic element types.
/// let mut array = VariantArray::new();
/// array.push(10.to_variant());
/// array.push("Hello".to_variant());
///
/// // Or equivalent, use the `varray!` macro which converts each element.
/// let array = varray![10, "Hello"];
///
/// // Access elements.
/// let value: Variant = array.at(0);
/// let value: i64 = array.at(0).to(); // Variant::to() extracts i64.
/// let maybe: Result<i64, _> = array.at(1).try_to(); // "Hello" is not i64 -> Err.
/// let maybe: Option<Variant> = array.get(3);
///
/// // ...and so on.
/// ```
///
/// # Thread safety
///
/// Usage is safe if the `Array` is used on a single thread only. Concurrent reads on
/// different threads are also safe, but any writes must be externally synchronized. The Rust
/// compiler will enforce this as long as you use only Rust threads, but it cannot protect against
/// concurrent modification on other threads (e.g. created through GDScript).
///
/// # Godot docs
///
/// [`Array[T]` (stable)](https://docs.godotengine.org/en/stable/classes/class_array.html)
pub struct Array<T: ArrayElement> {
    // Safety Invariant: The type of all values in `opaque` matches the type `T`.
    opaque: sys::types::OpaqueArray,
    _phantom: PhantomData<T>,
}

/// Guard that can only call immutable methods on the array.
struct ImmutableInnerArray<'a> {
    inner: inner::InnerArray<'a>,
}

impl<'a> std::ops::Deref for ImmutableInnerArray<'a> {
    type Target = inner::InnerArray<'a>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

/// A Godot `Array` without an assigned type.
pub type VariantArray = Array<Variant>;

// TODO check if these return a typed array
impl_builtin_froms!(VariantArray;
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

#[cfg(since_api = "4.3")]
impl_builtin_froms!(VariantArray;
    PackedVector4Array => array_from_packed_vector4_array,
);

impl<T: ArrayElement> Array<T> {
    fn from_opaque(opaque: sys::types::OpaqueArray) -> Self {
        // Note: type is not yet checked at this point, because array has not yet been initialized!
        Self {
            opaque,
            _phantom: PhantomData,
        }
    }

    /// Constructs an empty `Array`.
    pub fn new() -> Self {
        Self::default()
    }

    /// ⚠️ Returns the value at the specified index.
    ///
    /// This replaces the `Index` trait, which cannot be implemented for `Array` as references are not guaranteed to remain valid.
    ///
    /// # Panics
    ///
    /// If `index` is out of bounds. If you want to handle out-of-bounds access, use [`get()`](Self::get) instead.
    pub fn at(&self, index: usize) -> T {
        // Panics on out-of-bounds.
        let ptr = self.ptr(index);

        // SAFETY: `ptr` is a live pointer to a variant since `ptr.is_null()` just verified that the index is not out of bounds.
        let variant = unsafe { Variant::borrow_var_sys(ptr) };
        T::from_variant(variant)
    }

    /// Returns the value at the specified index, or `None` if the index is out-of-bounds.
    ///
    /// If you know the index is correct, use [`at()`](Self::at) instead.
    pub fn get(&self, index: usize) -> Option<T> {
        let ptr = self.ptr_or_null(index);
        if ptr.is_null() {
            None
        } else {
            // SAFETY: `ptr` is a live pointer to a variant since `ptr.is_null()` just verified that the index is not out of bounds.
            let variant = unsafe { Variant::borrow_var_sys(ptr) };
            Some(T::from_variant(variant))
        }
    }

    /// Returns `true` if the array contains the given value. Equivalent of `has` in GDScript.
    pub fn contains(&self, value: &T) -> bool {
        self.as_inner().has(value.to_variant())
    }

    /// Returns the number of times a value is in the array.
    pub fn count(&self, value: &T) -> usize {
        to_usize(self.as_inner().count(value.to_variant()))
    }

    /// Returns the number of elements in the array. Equivalent of `size()` in Godot.
    ///
    /// Retrieving the size incurs an FFI call. If you know the size hasn't changed, you may consider storing
    /// it in a variable. For loops, prefer iterators.
    #[doc(alias = "size")]
    pub fn len(&self) -> usize {
        to_usize(self.as_inner().size())
    }

    /// Returns `true` if the array is empty.
    ///
    /// Checking for emptiness incurs an FFI call. If you know the size hasn't changed, you may consider storing
    /// it in a variable. For loops, prefer iterators.
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

    /// Returns the first element in the array, or `None` if the array is empty.
    #[doc(alias = "first")]
    pub fn front(&self) -> Option<T> {
        (!self.is_empty()).then(|| {
            let variant = self.as_inner().front();
            T::from_variant(&variant)
        })
    }

    /// Returns the last element in the array, or `None` if the array is empty.
    #[doc(alias = "last")]
    pub fn back(&self) -> Option<T> {
        (!self.is_empty()).then(|| {
            let variant = self.as_inner().back();
            T::from_variant(&variant)
        })
    }

    /// Clears the array, removing all elements.
    pub fn clear(&mut self) {
        // SAFETY: No new values are written to the array, we only remove values from the array.
        unsafe { self.as_inner_mut() }.clear();
    }

    /// Sets the value at the specified index.
    ///
    /// # Panics
    ///
    /// If `index` is out of bounds.
    pub fn set(&mut self, index: usize, value: T) {
        let ptr_mut = self.ptr_mut(index);

        // SAFETY: `ptr_mut` just checked that the index is not out of bounds.
        unsafe {
            value.to_variant().move_into_var_ptr(ptr_mut);
        }
    }

    /// Appends an element to the end of the array.
    ///
    /// _Godot equivalents: `append` and `push_back`_
    #[doc(alias = "append")]
    #[doc(alias = "push_back")]
    pub fn push(&mut self, value: T) {
        // SAFETY: The array has type `T` and we're writing a value of type `T` to it.
        unsafe { self.as_inner_mut() }.push_back(value.to_variant());
    }

    /// Adds an element at the beginning of the array, in O(n).
    ///
    /// On large arrays, this method is much slower than [`push()`][Self::push], as it will move all the array's elements.
    /// The larger the array, the slower `push_front()` will be.
    pub fn push_front(&mut self, value: T) {
        // SAFETY: The array has type `T` and we're writing a value of type `T` to it.
        unsafe { self.as_inner_mut() }.push_front(value.to_variant());
    }

    /// Removes and returns the last element of the array. Returns `None` if the array is empty.
    ///
    /// _Godot equivalent: `pop_back`_
    #[doc(alias = "pop_back")]
    pub fn pop(&mut self) -> Option<T> {
        (!self.is_empty()).then(|| {
            // SAFETY: We do not write any values to the array, we just remove one.
            let variant = unsafe { self.as_inner_mut() }.pop_back();
            T::from_variant(&variant)
        })
    }

    /// Removes and returns the first element of the array, in O(n). Returns `None` if the array is empty.
    ///
    /// Note: On large arrays, this method is much slower than `pop()` as it will move all the
    /// array's elements. The larger the array, the slower `pop_front()` will be.
    pub fn pop_front(&mut self) -> Option<T> {
        (!self.is_empty()).then(|| {
            // SAFETY: We do not write any values to the array, we just remove one.
            let variant = unsafe { self.as_inner_mut() }.pop_front();
            T::from_variant(&variant)
        })
    }

    /// ⚠️ Inserts a new element before the index. The index must be valid or the end of the array (`index == len()`).
    ///
    /// On large arrays, this method is much slower than [`push()`][Self::push], as it will move all the array's elements after the inserted element.
    /// The larger the array, the slower `insert()` will be.
    ///
    /// # Panics
    /// If `index > len()`.
    pub fn insert(&mut self, index: usize, value: T) {
        let len = self.len();
        assert!(
            index <= len,
            "Array insertion index {index} is out of bounds: length is {len}",
        );

        // SAFETY: The array has type `T` and we're writing a value of type `T` to it.
        unsafe { self.as_inner_mut() }.insert(to_i64(index), value.to_variant());
    }

    /// ⚠️ Removes and returns the element at the specified index. Equivalent of `pop_at` in GDScript.
    ///
    /// On large arrays, this method is much slower than [`pop()`][Self::pop] as it will move all the array's
    /// elements after the removed element. The larger the array, the slower `remove()` will be.
    ///
    /// # Panics
    ///
    /// If `index` is out of bounds.
    #[doc(alias = "pop_at")]
    pub fn remove(&mut self, index: usize) -> T {
        self.check_bounds(index);

        // SAFETY: We do not write any values to the array, we just remove one.
        let variant = unsafe { self.as_inner_mut() }.pop_at(to_i64(index));
        T::from_variant(&variant)
    }

    /// Removes the first occurrence of a value from the array.
    ///
    /// If the value does not exist in the array, nothing happens. To remove an element by index, use [`remove()`][Self::remove] instead.
    ///
    /// On large arrays, this method is much slower than [`pop()`][Self::pop], as it will move all the array's
    /// elements after the removed element.
    pub fn erase(&mut self, value: &T) {
        // SAFETY: We don't write anything to the array.
        unsafe { self.as_inner_mut() }.erase(value.to_variant());
    }

    /// Assigns the given value to all elements in the array. This can be used together with
    /// `resize` to create an array with a given size and initialized elements.
    pub fn fill(&mut self, value: &T) {
        // SAFETY: The array has type `T` and we're writing values of type `T` to it.
        unsafe { self.as_inner_mut() }.fill(value.to_variant());
    }

    /// Resizes the array to contain a different number of elements.
    ///
    /// If the new size is smaller than the current size, then it removes elements from the end. If the new size is bigger than the current one
    /// then the new elements are set to `value`.
    ///
    /// If you know that the new size is smaller, then consider using [`shrink`](Array::shrink) instead.
    pub fn resize(&mut self, new_size: usize, value: &T) {
        let original_size = self.len();

        // SAFETY: While we do insert `Variant::nil()` if the new size is larger, we then fill it with `value` ensuring that all values in the
        // array are of type `T` still.
        unsafe { self.as_inner_mut() }.resize(to_i64(new_size));

        // If new_size < original_size then this is an empty iterator and does nothing.
        for i in original_size..new_size {
            self.set(i, value.to_godot());
        }
    }

    /// Shrinks the array down to `new_size`.
    ///
    /// This will only change the size of the array if `new_size` is smaller than the current size. Returns `true` if the array was shrunk.
    ///
    /// If you want to increase the size of the array, use [`resize`](Array::resize) instead.
    #[doc(alias = "resize")]
    pub fn shrink(&mut self, new_size: usize) -> bool {
        if new_size >= self.len() {
            return false;
        }

        // SAFETY: Since `new_size` is less than the current size, we'll only be removing elements from the array.
        unsafe { self.as_inner_mut() }.resize(to_i64(new_size));

        true
    }

    /// Appends another array at the end of this array. Equivalent of `append_array` in GDScript.
    pub fn extend_array(&mut self, other: Array<T>) {
        // SAFETY: `append_array` will only read values from `other`, and all types can be converted to `Variant`.
        let other: VariantArray = unsafe { other.assume_type::<Variant>() };

        // SAFETY: `append_array` will only write values gotten from `other` into `self`, and all values in `other` are guaranteed
        // to be of type `T`.
        let mut inner_self = unsafe { self.as_inner_mut() };
        inner_self.append_array(other);
    }

    /// Returns a shallow copy of the array. All array elements are copied, but any reference types
    /// (such as `Array`, `Dictionary` and `Object`) will still refer to the same value.
    ///
    /// To create a deep copy, use [`duplicate_deep()`][Self::duplicate_deep] instead.
    /// To create a new reference to the same array data, use [`clone()`][Clone::clone].
    pub fn duplicate_shallow(&self) -> Self {
        // SAFETY: We never write to the duplicated array, and all values read are read as `Variant`.
        let duplicate: VariantArray = unsafe { self.as_inner().duplicate(false) };

        // SAFETY: duplicate() returns a typed array with the same type as Self, and all values are taken from `self` so have the right type.
        unsafe { duplicate.assume_type() }
    }

    /// Returns a deep copy of the array. All nested arrays and dictionaries are duplicated and
    /// will not be shared with the original array. Note that any `Object`-derived elements will
    /// still be shallow copied.
    ///
    /// To create a shallow copy, use [`duplicate_shallow()`][Self::duplicate_shallow] instead.
    /// To create a new reference to the same array data, use [`clone()`][Clone::clone].
    pub fn duplicate_deep(&self) -> Self {
        // SAFETY: We never write to the duplicated array, and all values read are read as `Variant`.
        let duplicate: VariantArray = unsafe { self.as_inner().duplicate(true) };

        // SAFETY: duplicate() returns a typed array with the same type as Self, and all values are taken from `self` so have the right type.
        unsafe { duplicate.assume_type() }
    }

    /// Returns a sub-range `begin..end`, as a new array.
    ///
    /// The values of `begin` (inclusive) and `end` (exclusive) will be clamped to the array size.
    ///
    /// If specified, `step` is the relative index between source elements. It can be negative,
    /// in which case `begin` must be higher than `end`. For example,
    /// `Array::from(&[0, 1, 2, 3, 4, 5]).slice(5, 1, -2)` returns `[5, 3]`.
    ///
    /// Array elements are copied to the slice, but any reference types (such as `Array`,
    /// `Dictionary` and `Object`) will still refer to the same value. To create a deep copy, use
    /// [`subarray_deep()`][Self::subarray_deep] instead.
    #[doc(alias = "slice")]
    pub fn subarray_shallow(&self, begin: usize, end: usize, step: Option<isize>) -> Self {
        self.subarray_impl(begin, end, step, false)
    }

    /// Returns a sub-range `begin..end`, as a new `Array`.
    ///
    /// The values of `begin` (inclusive) and `end` (exclusive) will be clamped to the array size.
    ///
    /// If specified, `step` is the relative index between source elements. It can be negative,
    /// in which case `begin` must be higher than `end`. For example,
    /// `Array::from(&[0, 1, 2, 3, 4, 5]).slice(5, 1, -2)` returns `[5, 3]`.
    ///
    /// All nested arrays and dictionaries are duplicated and will not be shared with the original
    /// array. Note that any `Object`-derived elements will still be shallow copied. To create a
    /// shallow copy, use [`subarray_shallow()`][Self::subarray_shallow] instead.
    #[doc(alias = "slice")]
    pub fn subarray_deep(&self, begin: usize, end: usize, step: Option<isize>) -> Self {
        self.subarray_impl(begin, end, step, true)
    }

    fn subarray_impl(&self, begin: usize, end: usize, step: Option<isize>, deep: bool) -> Self {
        assert_ne!(step, Some(0), "subarray: step cannot be zero");

        let len = self.len();
        let begin = begin.min(len);
        let end = end.min(len);
        let step = step.unwrap_or(1);

        // SAFETY: The type of the array is `T` and we convert the returned array to an `Array<T>` immediately.
        let subarray: VariantArray = unsafe {
            self.as_inner()
                .slice(to_i64(begin), to_i64(end), step.try_into().unwrap(), deep)
        };

        // SAFETY: slice() returns a typed array with the same type as Self
        unsafe { subarray.assume_type() }
    }

    /// Returns an iterator over the elements of the `Array`. Note that this takes the array
    /// by reference but returns its elements by value, since they are internally converted from
    /// `Variant`.
    ///
    /// Notice that it's possible to modify the `Array` through another reference while
    /// iterating over it. This will not result in unsoundness or crashes, but will cause the
    /// iterator to behave in an unspecified way.
    pub fn iter_shared(&self) -> Iter<'_, T> {
        Iter {
            array: self,
            next_idx: 0,
        }
    }

    /// Returns the minimum value contained in the array if all elements are of comparable types.
    ///
    /// If the elements can't be compared or the array is empty, `None` is returned.
    pub fn min(&self) -> Option<T> {
        let min = self.as_inner().min();
        (!min.is_nil()).then(|| T::from_variant(&min))
    }

    /// Returns the maximum value contained in the array if all elements are of comparable types.
    ///
    /// If the elements can't be compared or the array is empty, `None` is returned.
    pub fn max(&self) -> Option<T> {
        let max = self.as_inner().max();
        (!max.is_nil()).then(|| T::from_variant(&max))
    }

    /// Returns a random element from the array, or `None` if it is empty.
    pub fn pick_random(&self) -> Option<T> {
        (!self.is_empty()).then(|| {
            let variant = self.as_inner().pick_random();
            T::from_variant(&variant)
        })
    }

    /// Searches the array for the first occurrence of a value and returns its index, or `None` if
    /// not found. Starts searching at index `from`; pass `None` to search the entire array.
    pub fn find(&self, value: &T, from: Option<usize>) -> Option<usize> {
        let from = to_i64(from.unwrap_or(0));
        let index = self.as_inner().find(value.to_variant(), from);
        if index >= 0 {
            Some(index.try_into().unwrap())
        } else {
            None
        }
    }

    /// Searches the array backwards for the last occurrence of a value and returns its index, or
    /// `None` if not found. Starts searching at index `from`; pass `None` to search the entire
    /// array.
    pub fn rfind(&self, value: &T, from: Option<usize>) -> Option<usize> {
        let from = from.map(to_i64).unwrap_or(-1);
        let index = self.as_inner().rfind(value.to_variant(), from);
        // It's not documented, but `rfind` returns -1 if not found.
        if index >= 0 {
            Some(to_usize(index))
        } else {
            None
        }
    }

    /// Finds the index of an existing value in a sorted array using binary search.
    /// Equivalent of `bsearch` in GDScript.
    ///
    /// If the value is not present in the array, returns the insertion index that
    /// would maintain sorting order.
    ///
    /// Calling `bsearch` on an unsorted array results in unspecified behavior.
    pub fn bsearch(&self, value: &T) -> usize {
        to_usize(self.as_inner().bsearch(value.to_variant(), true))
    }

    /// Finds the index of an existing value in a sorted array using binary search.
    /// Equivalent of `bsearch_custom` in GDScript.
    ///
    /// Takes a `Callable` and uses the return value of it to perform binary search.
    ///
    /// If the value is not present in the array, returns the insertion index that
    /// would maintain sorting order.
    ///
    /// Calling `bsearch_custom` on an unsorted array results in unspecified behavior.
    ///
    /// Consider using `sort_custom()` to ensure the sorting order is compatible with
    /// your callable's ordering
    pub fn bsearch_custom(&self, value: &T, func: Callable) -> usize {
        to_usize(
            self.as_inner()
                .bsearch_custom(value.to_variant(), func, true),
        )
    }

    /// Reverses the order of the elements in the array.
    pub fn reverse(&mut self) {
        // SAFETY: We do not write any values that don't already exist in the array, so all values have the correct type.
        unsafe { self.as_inner_mut() }.reverse();
    }

    /// Sorts the array.
    ///
    /// Note: The sorting algorithm used is not [stable](https://en.wikipedia.org/wiki/Sorting_algorithm#Stability).
    /// This means that values considered equal may have their order changed when using `sort_unstable`.
    #[doc(alias = "sort")]
    pub fn sort_unstable(&mut self) {
        // SAFETY: We do not write any values that don't already exist in the array, so all values have the correct type.
        unsafe { self.as_inner_mut() }.sort();
    }

    /// Sorts the array.
    ///
    /// Uses the provided `Callable` to determine ordering.
    ///
    /// Note: The sorting algorithm used is not [stable](https://en.wikipedia.org/wiki/Sorting_algorithm#Stability).
    /// This means that values considered equal may have their order changed when using `sort_unstable_custom`.
    #[doc(alias = "sort_custom")]
    pub fn sort_unstable_custom(&mut self, func: Callable) {
        // SAFETY: We do not write any values that don't already exist in the array, so all values have the correct type.
        unsafe { self.as_inner_mut() }.sort_custom(func);
    }

    /// Shuffles the array such that the items will have a random order. This method uses the
    /// global random number generator common to methods such as `randi`. Call `randomize` to
    /// ensure that a new seed will be used each time if you want non-reproducible shuffling.
    pub fn shuffle(&mut self) {
        // SAFETY: We do not write any values that don't already exist in the array, so all values have the correct type.
        unsafe { self.as_inner_mut() }.shuffle();
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
            "Array index {index} is out of bounds: length is {len}",
        );
    }

    /// Returns a pointer to the element at the given index.
    ///
    /// # Panics
    ///
    /// If `index` is out of bounds.
    fn ptr(&self, index: usize) -> sys::GDExtensionConstVariantPtr {
        let ptr = self.ptr_or_null(index);
        assert!(
            !ptr.is_null(),
            "Array index {index} out of bounds (len {len})",
            len = self.len(),
        );
        ptr
    }

    /// Returns a pointer to the element at the given index, or null if out of bounds.
    fn ptr_or_null(&self, index: usize) -> sys::GDExtensionConstVariantPtr {
        // SAFETY: array_operator_index_const returns null for invalid indexes.
        let variant_ptr = unsafe {
            let index = to_i64(index);
            interface_fn!(array_operator_index_const)(self.sys(), index)
        };

        // Signature is wrong in GDExtension, semantically this is a const ptr
        sys::SysPtr::as_const(variant_ptr)
    }

    /// Returns a mutable pointer to the element at the given index.
    ///
    /// # Panics
    ///
    /// If `index` is out of bounds.
    fn ptr_mut(&mut self, index: usize) -> sys::GDExtensionVariantPtr {
        let ptr = self.ptr_mut_or_null(index);
        assert!(
            !ptr.is_null(),
            "Array index {index} out of bounds (len {len})",
            len = self.len(),
        );
        ptr
    }

    /// Returns a pointer to the element at the given index, or null if out of bounds.
    fn ptr_mut_or_null(&mut self, index: usize) -> sys::GDExtensionVariantPtr {
        // SAFETY: array_operator_index returns null for invalid indexes.
        unsafe {
            let index = to_i64(index);
            interface_fn!(array_operator_index)(self.sys_mut(), index)
        }
    }

    /// # Safety
    ///
    /// This has the same safety issues as doing `self.assume_type::<Variant>()` and so the relevant safety invariants from
    /// [`assume_type`](Self::assume_type) must be upheld.
    ///
    /// In particular this means that all reads are fine, since all values can be converted to `Variant`. However, writes are only OK
    /// if they match the type `T`.
    #[doc(hidden)]
    pub unsafe fn as_inner_mut(&self) -> inner::InnerArray {
        // The memory layout of `Array<T>` does not depend on `T`.
        inner::InnerArray::from_outer_typed(self)
    }

    fn as_inner(&self) -> ImmutableInnerArray {
        ImmutableInnerArray {
            // SAFETY: We can only read from the array.
            inner: unsafe { self.as_inner_mut() },
        }
    }

    /// Changes the generic type on this array, without changing its contents. Needed for API
    /// functions that return a variant array even though we know its type, and for API functions
    /// that take a variant array even though we want to pass a typed one.
    ///
    /// # Safety
    ///
    /// - Any values written to the array must match the runtime type of the array.
    /// - Any values read from the array must be convertible to the type `U`.
    ///
    /// If the safety invariant of `Array` is intact, which it must be for any publicly accessible arrays, then `U` must match
    /// the runtime type of the array. This then implies that both of the conditions above hold. This means that you only need
    /// to keep the above conditions in mind if you are intentionally violating the safety invariant of `Array`.
    ///
    /// Note also that any `GodotType` can be written to a `Variant` array.
    ///
    /// In the current implementation, both cases will produce a panic rather than undefined
    /// behavior, but this should not be relied upon.
    unsafe fn assume_type<U: ArrayElement>(self) -> Array<U> {
        // SAFETY: The memory layout of `Array<T>` does not depend on `T`.
        unsafe { std::mem::transmute(self) }
    }

    /// Returns the runtime type info of this array.
    fn type_info(&self) -> ArrayTypeInfo {
        let variant_type = VariantType::from_sys(
            self.as_inner().get_typed_builtin() as sys::GDExtensionVariantType
        );
        let class_name = self.as_inner().get_typed_class_name();

        ArrayTypeInfo {
            variant_type,
            class_name,
        }
    }

    /// Checks that the inner array has the correct type set on it for storing elements of type `T`.
    fn with_checked_type(self) -> Result<Self, ConvertError> {
        let self_ty = self.type_info();
        let target_ty = ArrayTypeInfo::of::<T>();

        if self_ty == target_ty {
            Ok(self)
        } else {
            Err(FromGodotError::BadArrayType {
                expected: target_ty,
                actual: self_ty,
            }
            .into_error(self))
        }
    }

    /// Sets the type of the inner array.
    ///
    /// # Safety
    ///
    /// Must only be called once, directly after creation.
    unsafe fn init_inner_type(&mut self) {
        debug_assert!(self.is_empty());
        debug_assert!(!self.type_info().is_typed());

        let type_info = ArrayTypeInfo::of::<T>();
        if type_info.is_typed() {
            let script = Variant::nil();

            // SAFETY: The array is a newly created empty untyped array.
            unsafe {
                interface_fn!(array_set_typed)(
                    self.sys_mut(),
                    type_info.variant_type.sys(),
                    type_info.class_name.string_sys(),
                    script.var_sys(),
                );
            }
        }
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Traits

// Godot has some inconsistent behavior around NaN values. In GDScript, `NAN == NAN` is `false`,
// but `[NAN] == [NAN]` is `true`. If they decide to make all NaNs equal, we can implement `Eq` and
// `Ord`; if they decide to make all NaNs unequal, we can remove this comment.
//
// impl<T> Eq for Array<T> {}
//
// impl<T> Ord for Array<T> {
//     ...
// }

// SAFETY:
// - `move_return_ptr`
//   Nothing special needs to be done beyond a `std::mem::swap` when returning an Array.
//   So we can just use `ffi_methods`.
//
// - `from_arg_ptr`
//   Arrays are properly initialized through a `from_sys` call, but the ref-count should be incremented
//   as that is the callee's responsibility. Which we do by calling `std::mem::forget(array.clone())`.
unsafe impl<T: ArrayElement> GodotFfi for Array<T> {
    fn variant_type() -> VariantType {
        VariantType::ARRAY
    }

    ffi_methods! { type sys::GDExtensionTypePtr = *mut Opaque; .. }
}

// Only implement for untyped arrays; typed arrays cannot be nested in Godot.
impl ArrayElement for VariantArray {}

impl<T: ArrayElement> GodotConvert for Array<T> {
    type Via = Self;
}

impl<T: ArrayElement> ToGodot for Array<T> {
    fn to_godot(&self) -> Self::Via {
        self.clone()
    }

    fn into_godot(self) -> Self::Via {
        self
    }

    fn to_variant(&self) -> Variant {
        self.ffi_to_variant()
    }
}

impl<T: ArrayElement> FromGodot for Array<T> {
    fn try_from_godot(via: Self::Via) -> Result<Self, ConvertError> {
        Ok(via)
    }
}

impl<T: ArrayElement> fmt::Debug for Array<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // Going through `Variant` because there doesn't seem to be a direct way.
        write!(f, "{:?}", self.to_variant().stringify())
    }
}

impl<T: ArrayElement + fmt::Display> fmt::Display for Array<T> {
    /// Formats `Array` to match Godot's string representation.
    ///
    /// # Example
    /// ```no_run
    /// # use godot::prelude::*;
    /// let a = array![1,2,3,4];
    /// assert_eq!(format!("{a}"), "[1, 2, 3, 4]");
    /// ```
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "[")?;
        for (count, v) in self.iter_shared().enumerate() {
            if count != 0 {
                write!(f, ", ")?;
            }
            write!(f, "{v}")?;
        }
        write!(f, "]")
    }
}

/// Creates a new reference to the data in this array. Changes to the original array will be
/// reflected in the copy and vice versa.
///
/// To create a (mostly) independent copy instead, see [`Array::duplicate_shallow()`] and
/// [`Array::duplicate_deep()`].
impl<T: ArrayElement> Clone for Array<T> {
    fn clone(&self) -> Self {
        // SAFETY: `self` is a valid array, since we have a reference that keeps it alive.
        let array = unsafe {
            Self::new_with_uninit(|self_ptr| {
                let ctor = sys::builtin_fn!(array_construct_copy);
                let args = [self.sys()];
                ctor(self_ptr, args.as_ptr());
            })
        };

        array
            .with_checked_type()
            .expect("copied array should have same type as original array")
    }
}

impl<T: ArrayElement + TypeStringHint> TypeStringHint for Array<T> {
    fn type_string() -> String {
        format!("{}:{}", VariantType::ARRAY.ord(), T::type_string())
    }
}

impl TypeStringHint for VariantArray {
    fn type_string() -> String {
        builtin_type_string::<VariantArray>()
    }
}

impl<T: ArrayElement> Var for Array<T> {
    fn get_property(&self) -> Self::Via {
        self.to_godot()
    }

    fn set_property(&mut self, value: Self::Via) {
        *self = FromGodot::from_godot(value)
    }

    #[cfg(since_api = "4.2")]
    fn property_hint() -> PropertyHintInfo {
        if T::Ffi::variant_type() == VariantType::NIL {
            return PropertyHintInfo::with_hint_none("");
        }

        PropertyHintInfo {
            hint: crate::global::PropertyHint::ARRAY_TYPE,
            hint_string: T::godot_type_name().into(),
        }
    }
}

impl<T: ArrayElement + TypeStringHint> Export for Array<T> {
    fn default_export_info() -> PropertyHintInfo {
        PropertyHintInfo {
            hint: crate::global::PropertyHint::TYPE_STRING,
            hint_string: T::type_string().into(),
        }
    }
}

impl Export for Array<Variant> {
    fn default_export_info() -> PropertyHintInfo {
        PropertyHintInfo::with_hint_none("Array")
    }
}

impl<T: ArrayElement> Default for Array<T> {
    #[inline]
    fn default() -> Self {
        let mut array = unsafe {
            Self::new_with_uninit(|self_ptr| {
                let ctor = sys::builtin_fn!(array_construct_default);
                ctor(self_ptr, std::ptr::null_mut())
            })
        };

        // SAFETY: We just created this array, and haven't called `init_inner_type` before.
        unsafe { array.init_inner_type() };
        array
    }
}

// T must be GodotType (or subtrait ArrayElement), because drop() requires sys_mut(), which is on the GodotFfi trait.
// Its sister method GodotFfi::from_sys_init() requires Default, which is only implemented for T: GodotType.
// This could be addressed by splitting up GodotFfi if desired.
impl<T: ArrayElement> Drop for Array<T> {
    #[inline]
    fn drop(&mut self) {
        unsafe {
            let array_destroy = sys::builtin_fn!(array_destroy);
            array_destroy(self.sys_mut());
        }
    }
}

impl<T: ArrayElement> GodotType for Array<T> {
    type Ffi = Self;

    fn to_ffi(&self) -> Self::Ffi {
        // `to_ffi` is sometimes intentionally called with an array in an invalid state.
        self.clone()
    }

    fn into_ffi(self) -> Self::Ffi {
        self
    }

    fn try_from_ffi(ffi: Self::Ffi) -> Result<Self, ConvertError> {
        Ok(ffi)
    }

    fn godot_type_name() -> String {
        "Array".to_string()
    }
}

impl<T: ArrayElement> GodotFfiVariant for Array<T> {
    fn ffi_to_variant(&self) -> Variant {
        unsafe {
            Variant::new_with_var_uninit(|variant_ptr| {
                let array_to_variant = sys::builtin_fn!(array_to_variant);
                array_to_variant(variant_ptr, sys::SysPtr::force_mut(self.sys()));
            })
        }
    }

    fn ffi_from_variant(variant: &Variant) -> Result<Self, ConvertError> {
        if variant.get_type() != Self::variant_type() {
            return Err(FromVariantError::BadType {
                expected: Self::variant_type(),
                actual: variant.get_type(),
            }
            .into_error(variant.clone()));
        }

        let array = unsafe {
            Self::new_with_uninit(|self_ptr| {
                let array_from_variant = sys::builtin_fn!(array_from_variant);
                array_from_variant(self_ptr, sys::SysPtr::force_mut(variant.var_sys()));
            })
        };

        array.with_checked_type()
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Conversion traits

/// Creates a `Array` from the given Rust array.
impl<T: ArrayElement + ToGodot, const N: usize> From<&[T; N]> for Array<T> {
    fn from(arr: &[T; N]) -> Self {
        Self::from(&arr[..])
    }
}

/// Creates a `Array` from the given slice.
impl<T: ArrayElement + ToGodot> From<&[T]> for Array<T> {
    fn from(slice: &[T]) -> Self {
        let mut array = Self::new();
        let len = slice.len();
        if len == 0 {
            return array;
        }

        // SAFETY: We fill the array with `Variant::nil()`, however since we're resizing to the size of the slice we'll end up rewriting all
        // the nulls with values of type `T`.
        unsafe { array.as_inner_mut() }.resize(to_i64(len));

        // SAFETY: `array` has `len` elements since we just resized it, and they are all valid `Variant`s. Additionally, since
        // the array was created in this function, and we do not access the array while this slice exists, the slice has unique
        // access to the elements.
        let elements = unsafe { Variant::borrow_slice_mut(array.ptr_mut(0), len) };
        for (element, array_slot) in slice.iter().zip(elements.iter_mut()) {
            *array_slot = element.to_variant();
        }

        array
    }
}

/// Creates a `Array` from an iterator.
impl<T: ArrayElement + ToGodot> FromIterator<T> for Array<T> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let mut array = Self::new();
        array.extend(iter);
        array
    }
}

/// Extends a `Array` with the contents of an iterator.
impl<T: ArrayElement + ToGodot> Extend<T> for Array<T> {
    fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
        // Unfortunately the GDExtension API does not offer the equivalent of `Vec::reserve`.
        // Otherwise, we could use it to pre-allocate based on `iter.size_hint()`.
        //
        // A faster implementation using `resize()` and direct pointer writes might still be
        // possible.
        for item in iter.into_iter() {
            self.push(item);
        }
    }
}

/// Converts this array to a strongly typed Rust vector.
impl<T: ArrayElement + FromGodot> From<&Array<T>> for Vec<T> {
    fn from(array: &Array<T>) -> Vec<T> {
        let len = array.len();
        let mut vec = Vec::with_capacity(len);

        // SAFETY: Unless `experimental-threads` is enabled, then we cannot have concurrent access to this array.
        // And since we don't concurrently access the array in this function, we can create a slice to its contents.
        let elements = unsafe { Variant::borrow_slice(array.ptr(0), len) };

        vec.extend(elements.iter().map(T::from_variant));

        vec
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

/// An iterator over typed elements of an [`Array`].
pub struct Iter<'a, T: ArrayElement> {
    array: &'a Array<T>,
    next_idx: usize,
}

impl<'a, T: ArrayElement + FromGodot> Iterator for Iter<'a, T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.next_idx < self.array.len() {
            let idx = self.next_idx;
            self.next_idx += 1;

            let element_ptr = self.array.ptr_or_null(idx);

            // SAFETY: We just checked that the index is not out of bounds, so the pointer won't be null.
            // We immediately convert this to the right element, so barring `experimental-threads` the pointer won't be invalidated in time.
            let variant = unsafe { Variant::borrow_var_sys(element_ptr) };
            let element = T::from_variant(variant);
            Some(element)
        } else {
            None
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.array.len() - self.next_idx;
        (remaining, Some(remaining))
    }
}

// TODO There's a macro for this, but it doesn't support generics yet; add support and use it
impl<T: ArrayElement> PartialEq for Array<T> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        unsafe {
            let mut result = false;
            sys::builtin_call! {
                array_operator_equal(self.sys(), other.sys(), result.sys_mut())
            }
            result
        }
    }
}

impl<T: ArrayElement> PartialOrd for Array<T> {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        let op_less = |lhs, rhs| unsafe {
            let mut result = false;
            sys::builtin_call! {
                array_operator_less(lhs, rhs, result.sys_mut())
            }
            result
        };

        if op_less(self.sys(), other.sys()) {
            Some(std::cmp::Ordering::Less)
        } else if op_less(other.sys(), self.sys()) {
            Some(std::cmp::Ordering::Greater)
        } else if self.eq(other) {
            Some(std::cmp::Ordering::Equal)
        } else {
            None
        }
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

/// Constructs [`Array`] literals, similar to Rust's standard `vec!` macro.
///
/// The type of the array is inferred from the arguments.
///
/// # Example
/// ```no_run
/// # use godot::prelude::*;
/// let arr = array![3, 1, 4];  // Array<i32>
/// ```
///
/// # See also
/// To create an `Array` of variants, see the [`varray!`] macro.
///
/// For dictionaries, a similar macro [`dict!`] exists.
#[macro_export]
macro_rules! array {
    ($($elements:expr),* $(,)?) => {
        {
            let mut array = $crate::builtin::Array::default();
            $(
                array.push($elements);
            )*
            array
        }
    };
}

/// Constructs [`VariantArray`] literals, similar to Rust's standard `vec!` macro.
///
/// The type of the array elements is always [`Variant`].
///
/// # Example
/// ```no_run
/// # use godot::prelude::*;
/// let arr: VariantArray = varray![42_i64, "hello", true];
/// ```
///
/// # See also
/// To create a typed `Array` with a single element type, see the [`array!`] macro.
///
/// For dictionaries, a similar macro [`dict!`] exists.
#[macro_export]
macro_rules! varray {
    // Note: use to_variant() and not Variant::from(), as that works with both references and values
    ($($elements:expr),* $(,)?) => {
        {
            use $crate::meta::ToGodot as _;
            let mut array = $crate::builtin::VariantArray::default();
            $(
                array.push($elements.to_variant());
            )*
            array
        }
    };
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

#[cfg(feature = "serde")]
mod serialize {
    use super::*;
    use serde::de::{SeqAccess, Visitor};
    use serde::ser::SerializeSeq;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::marker::PhantomData;

    impl<T> Serialize for Array<T>
    where
        T: ArrayElement + Serialize,
    {
        #[inline]
        fn serialize<S>(
            &self,
            serializer: S,
        ) -> Result<<S as Serializer>::Ok, <S as Serializer>::Error>
        where
            S: Serializer,
        {
            let mut sequence = serializer.serialize_seq(Some(self.len()))?;
            for e in self.iter_shared() {
                sequence.serialize_element(&e)?
            }
            sequence.end()
        }
    }

    impl<'de, T> Deserialize<'de> for Array<T>
    where
        T: ArrayElement + Deserialize<'de>,
    {
        #[inline]
        fn deserialize<D>(deserializer: D) -> Result<Self, <D as Deserializer<'de>>::Error>
        where
            D: Deserializer<'de>,
        {
            struct ArrayVisitor<T>(PhantomData<T>);
            impl<'de, T> Visitor<'de> for ArrayVisitor<T>
            where
                T: ArrayElement + Deserialize<'de>,
            {
                type Value = Array<T>;

                fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                    formatter.write_str(std::any::type_name::<Self::Value>())
                }

                fn visit_seq<A>(
                    self,
                    mut seq: A,
                ) -> Result<Self::Value, <A as SeqAccess<'de>>::Error>
                where
                    A: SeqAccess<'de>,
                {
                    let mut vec = seq.size_hint().map_or_else(Vec::new, Vec::with_capacity);
                    while let Some(val) = seq.next_element::<T>()? {
                        vec.push(val);
                    }
                    Ok(Self::Value::from(vec.as_slice()))
                }
            }

            deserializer.deserialize_seq(ArrayVisitor::<T>(PhantomData))
        }
    }
}
