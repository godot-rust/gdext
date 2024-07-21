/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::fmt;

use crate::builtin::*;
use crate::meta::error::{ArrayMismatch, ConvertError, ErrorKind};
use crate::meta::{
    ArrayElement, ArrayTypeInfo, FromGodot, GodotConvert, GodotFfiVariant, GodotType, ToGodot,
};
use crate::registry::property::{Export, PropertyHintInfo, TypeStringHint, Var};
use godot_ffi as sys;
use sys::{ffi_methods, GodotFfi};

#[derive(PartialEq, PartialOrd)]
pub struct OutArray {
    inner: VariantArray,
}

impl OutArray {
    pub(crate) fn consume_typed_array<T: ArrayElement>(array: Array<T>) -> Self {
        // SAFETY:
        // - No values are written to the array, as the public API lacks all corresponding methods.
        // - All values read by the value are convertible to U=Variant, by definition.
        let array = unsafe { array.assume_type::<Variant>() };

        Self { inner: array }
    }

    fn from_opaque(opaque: sys::types::OpaqueArray) -> Self {
        Self {
            inner: VariantArray::from_opaque(opaque),
        }
    }

    pub(crate) fn new_untyped() -> Self {
        // SAFETY: we explicitly create an untyped array, so we don't need to set a type.
        let inner = unsafe { VariantArray::default_unchecked() };

        Self { inner }
    }

    /// ⚠️ Returns the value at the specified index.
    ///
    /// This replaces the `Index` trait, which cannot be implemented for `Array` as references are not guaranteed to remain valid.
    ///
    /// # Panics
    ///
    /// If `index` is out of bounds. If you want to handle out-of-bounds access, use [`get()`](Self::get) instead.
    pub fn at(&self, index: usize) -> Variant {
        self.inner.at(index)
    }

    /// Returns the value at the specified index, or `None` if the index is out-of-bounds.
    ///
    /// If you know the index is correct, use [`at()`](Self::at) instead.
    pub fn get(&self, index: usize) -> Option<Variant> {
        self.inner.get(index)
    }

    /// Returns `true` if the array contains the given value. Equivalent of `has` in GDScript.
    pub fn contains(&self, value: &Variant) -> bool {
        self.inner.contains(value)
    }

    /// Returns the number of times a value is in the array.
    pub fn count(&self, value: &Variant) -> usize {
        self.inner.count(value)
    }

    /// Returns the number of elements in the array. Equivalent of `size()` in Godot.
    ///
    /// Retrieving the size incurs an FFI call. If you know the size hasn't changed, you may consider storing
    /// it in a variable. For loops, prefer iterators.
    #[doc(alias = "size")]
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Returns `true` if the array is empty.
    ///
    /// Checking for emptiness incurs an FFI call. If you know the size hasn't changed, you may consider storing
    /// it in a variable. For loops, prefer iterators.
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Returns a 32-bit integer hash value representing the array and its contents.
    ///
    /// Note: Arrays with equal content will always produce identical hash values. However, the
    /// reverse is not true. Returning identical hash values does not imply the arrays are equal,
    /// because different arrays can have identical hash values due to hash collisions.
    pub fn hash(&self) -> u32 {
        self.inner.hash()
    }

    /// Returns the first element in the array, or `None` if the array is empty.
    #[doc(alias = "first")]
    pub fn front(&self) -> Option<Variant> {
        self.inner.front()
    }

    /// Returns the last element in the array, or `None` if the array is empty.
    #[doc(alias = "last")]
    pub fn back(&self) -> Option<Variant> {
        self.inner.back()
    }

    /// Clears the array, removing all elements.
    pub fn clear(&mut self) {
        self.inner.clear()
    }

    /// Removes and returns the last element of the array. Returns `None` if the array is empty.
    ///
    /// _Godot equivalent: `pop_back`_
    #[doc(alias = "pop_back")]
    pub fn pop(&mut self) -> Option<Variant> {
        self.inner.pop()
    }

    /// Removes and returns the first element of the array, in O(n). Returns `None` if the array is empty.
    ///
    /// Note: On large arrays, this method is much slower than `pop()` as it will move all the
    /// array's elements. The larger the array, the slower `pop_front()` will be.
    pub fn pop_front(&mut self) -> Option<Variant> {
        self.inner.pop_front()
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
    pub fn remove(&mut self, index: usize) -> Variant {
        self.inner.remove(index)
    }

    /// Removes the first occurrence of a value from the array.
    ///
    /// If the value does not exist in the array, nothing happens. To remove an element by index, use [`remove()`][Self::remove] instead.
    ///
    /// On large arrays, this method is much slower than [`pop()`][Self::pop], as it will move all the array's
    /// elements after the removed element.
    // TODO test if method works.
    pub fn erase(&mut self, value: &Variant) {
        self.inner.erase(value)
    }

    /// Shrinks the array down to `new_size`.
    ///
    /// This will only change the size of the array if `new_size` is smaller than the current size. Returns `true` if the array was shrunk.
    ///
    /// If you want to increase the size of the array, use [`resize`](Array::resize) instead.
    #[doc(alias = "resize")]
    pub fn shrink(&mut self, new_size: usize) -> bool {
        self.inner.shrink(new_size)
    }

    /// Returns a shallow copy of the array. All array elements are copied, but any reference types
    /// (such as `Array`, `Dictionary` and `Object`) will still refer to the same value.
    ///
    /// To create a deep copy, use [`duplicate_deep()`][Self::duplicate_deep] instead.
    /// To create a new reference to the same array data, use [`clone()`][Clone::clone].
    pub fn duplicate_shallow(&self) -> Self {
        Self::consume_typed_array(self.inner.duplicate_shallow())
    }

    /// Returns a deep copy of the array. All nested arrays and dictionaries are duplicated and
    /// will not be shared with the original array. Note that any `Object`-derived elements will
    /// still be shallow copied.
    ///
    /// To create a shallow copy, use [`duplicate_shallow()`][Self::duplicate_shallow] instead.
    /// To create a new reference to the same array data, use [`clone()`][Clone::clone].
    pub fn duplicate_deep(&self) -> Self {
        Self::consume_typed_array(self.inner.duplicate_deep())
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
        let sliced = self.inner.subarray_shallow(begin, end, step);
        Self::consume_typed_array(sliced)
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
        let sliced = self.inner.subarray_deep(begin, end, step);
        Self::consume_typed_array(sliced)
    }

    /// Returns an iterator over the elements of the `Array`. Note that this takes the array
    /// by reference but returns its elements by value, since they are internally converted from
    /// `Variant`.
    ///
    /// Notice that it's possible to modify the `Array` through another reference while
    /// iterating over it. This will not result in unsoundness or crashes, but will cause the
    /// iterator to behave in an unspecified way.
    pub fn iter_shared(&self) -> Iter<'_> {
        Iter {
            inner: self.inner.iter_shared(),
        }
    }

    /// Returns the minimum value contained in the array if all elements are of comparable types.
    ///
    /// If the elements can't be compared or the array is empty, `None` is returned.
    pub fn min(&self) -> Option<Variant> {
        self.inner.min()
    }

    /// Returns the maximum value contained in the array if all elements are of comparable types.
    ///
    /// If the elements can't be compared or the array is empty, `None` is returned.
    pub fn max(&self) -> Option<Variant> {
        self.inner.max()
    }

    /// Returns a random element from the array, or `None` if it is empty.
    pub fn pick_random(&self) -> Option<Variant> {
        self.inner.pick_random()
    }

    /// Searches the array for the first occurrence of a value and returns its index, or `None` if
    /// not found. Starts searching at index `from`; pass `None` to search the entire array.
    // TODO test if method works.
    pub fn find(&self, value: &Variant, from: Option<usize>) -> Option<usize> {
        self.inner.find(value, from)
    }

    /// Searches the array backwards for the last occurrence of a value and returns its index, or
    /// `None` if not found. Starts searching at index `from`; pass `None` to search the entire array.
    // TODO test if method works.
    pub fn rfind(&self, value: &Variant, from: Option<usize>) -> Option<usize> {
        self.inner.rfind(value, from)
    }

    /// Finds the index of an existing value in a sorted array using binary search.
    /// Equivalent of `bsearch` in GDScript.
    ///
    /// If the value is not present in the array, returns the insertion index that
    /// would maintain sorting order.
    ///
    /// Calling `bsearch` on an unsorted array results in unspecified behavior.
    pub fn bsearch(&self, value: &Variant) -> usize {
        self.inner.bsearch(value)
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
    pub fn bsearch_custom(&self, value: &Variant, func: Callable) -> usize {
        self.inner.bsearch_custom(value, func)
    }

    /// Reverses the order of the elements in the array.
    pub fn reverse(&mut self) {
        self.inner.reverse()
    }

    /// Sorts the array.
    ///
    /// Note: The sorting algorithm used is not [stable](https://en.wikipedia.org/wiki/Sorting_algorithm#Stability).
    /// This means that values considered equal may have their order changed when using `sort_unstable`.
    #[doc(alias = "sort")]
    pub fn sort_unstable(&mut self) {
        self.inner.sort_unstable()
    }

    /// Sorts the array.
    ///
    /// Uses the provided `Callable` to determine ordering.
    ///
    /// Note: The sorting algorithm used is not [stable](https://en.wikipedia.org/wiki/Sorting_algorithm#Stability).
    /// This means that values considered equal may have their order changed when using `sort_unstable_custom`.
    #[doc(alias = "sort_custom")]
    pub fn sort_unstable_custom(&mut self, func: Callable) {
        self.inner.sort_unstable_custom(func)
    }

    /// Shuffles the array such that the items will have a random order. This method uses the
    /// global random number generator common to methods such as `randi`. Call `randomize` to
    /// ensure that a new seed will be used each time if you want non-reproducible shuffling.
    pub fn shuffle(&mut self) {
        self.inner.shuffle()
    }

    /// Returns the dynamic element type.
    ///
    /// If the array is untyped (`Array<Variant>`), then `VariantType::NIL` will be returned.
    ///
    /// See also [`get_typed_class_name()`][Self::get_typed_class_name].
    pub fn get_typed_builtin(&self) -> VariantType {
        self.inner.type_info().variant_type
    }

    /// Returns the dynamic class of the elements.
    ///
    /// If the array is untyped (`Array<Variant>`) or the builtin type is not `VariantType::OBJECT`, then `None` will be returned.
    ///
    /// See also [`get_typed_builtin()`][Self::get_typed_builtin].
    pub fn get_typed_class_name(&self) -> Option<StringName> {
        self.inner.type_info().class_name
    }

    /// Attempts to convert to `Array<T>`.
    ///
    /// If the dynamic type is not `T`, then `Err` is returned. You can use [`get_typed_builtin()`][Self::get_typed_builtin] and
    /// [`get_typed_class_name()`][Self::get_typed_class_name] to check the dynamic type.
    pub fn try_into_typed_array<T: ArrayElement>(self) -> Result<Array<T>, ConvertError> {
        let from_type = self.inner.type_info();
        let to_type = ArrayTypeInfo::of::<T>();

        if from_type == to_type {
            // SAFETY: just checked type.
            let array = unsafe { self.inner.assume_type::<T>() };
            Ok(array)
        } else {
            Err(ConvertError::with_kind(ErrorKind::FromOutArray(
                ArrayMismatch {
                    expected: to_type,
                    actual: from_type,
                },
            )))
        }
    }

    /// Attempts to convert to `VariantArray` (== `Array<Variant>`).
    ///
    /// If the dynamic type is not `T`, then `Err` is returned. You can use [`get_typed_builtin()`][Self::get_typed_builtin] and
    /// [`get_typed_class_name()`][Self::get_typed_class_name] to check the dynamic type.
    ///
    /// This is a shorthand for [`try_into_typed_array::<Variant>()`][Self::try_into_typed_array].
    pub fn try_into_variant_array(self) -> Result<VariantArray, ConvertError> {
        self.try_into_typed_array::<Variant>()
    }

    /// ⚠️  Converts to `Array<T>`, panicking on error.
    ///
    /// # Panics
    /// If the dynamic type is not `T`.
    // See what usage patterns emerge before making public.
    #[allow(dead_code)] // not yet used.
    pub(crate) fn into_typed_array<T: ArrayElement>(self) -> Array<T> {
        self.try_into_typed_array().unwrap_or_else(|err| {
            panic!(
                "Failed to convert OutArray to Array<{}>: {err}",
                T::class_name()
            )
        })
    }

    /// ⚠️ Converts to `Array<Variant>`, panicking on error.
    ///
    /// # Panics
    /// If the dynamic type is not `Variant`.
    // See what usage patterns emerge before making public.
    pub(crate) fn into_variant_array(self) -> VariantArray {
        self.try_into_variant_array()
            .unwrap_or_else(|err| panic!("Failed to convert OutArray to VariantArray: {err}"))
    }

    // Visibility: shared with Array<T>.
    /// # Safety
    /// See [`Array::assume_type()`].
    pub(super) unsafe fn assume_type<T: ArrayElement>(self) -> Array<T> {
        debug_assert_eq!(self.inner.type_info(), ArrayTypeInfo::of::<T>());
        self.inner.assume_type()
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Traits

// SAFETY: See VariantArray.
//
// We cannot provide GodotConvert with Via=VariantArray, because ToGodot::to_godot() would otherwise enable a safe conversion from OutArray to
// VariantArray, which is not sound.
unsafe impl GodotFfi for OutArray {
    fn variant_type() -> VariantType {
        VariantType::ARRAY
    }

    // No Default trait, thus manually defining this and ffi_methods!.
    unsafe fn new_with_init(init_fn: impl FnOnce(sys::GDExtensionTypePtr)) -> Self {
        let mut result = Self::new_untyped();
        init_fn(result.sys_mut());
        result
    }

    // Manually forwarding these, since no Opaque.
    fn sys(&self) -> sys::GDExtensionConstTypePtr {
        self.inner.sys()
    }

    fn sys_mut(&mut self) -> sys::GDExtensionTypePtr {
        self.inner.sys_mut()
    }

    unsafe fn move_return_ptr(self, dst: sys::GDExtensionTypePtr, call_type: sys::PtrcallType) {
        self.inner.move_return_ptr(dst, call_type)
    }

    ffi_methods! { type sys::GDExtensionTypePtr = *mut Opaque;
        fn new_from_sys;
        fn new_with_uninit;
        fn from_arg_ptr;
    }
}

impl Clone for OutArray {
    fn clone(&self) -> Self {
        // SAFETY: we don't want to check that static type (Variant) matches dynamic type (anything), because all types are valid in OutArray.
        let inner = unsafe { VariantArray::clone_unchecked(&self.inner) };

        Self { inner }
    }
}

// Only implement for untyped arrays; typed arrays cannot be nested in Godot.
impl ArrayElement for OutArray {}

impl GodotConvert for OutArray {
    type Via = Self;
}

impl ToGodot for OutArray {
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

impl FromGodot for OutArray {
    fn try_from_godot(via: Self::Via) -> Result<Self, ConvertError> {
        Ok(via)
    }
}

impl fmt::Debug for OutArray {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.inner.fmt(f)
    }
}

impl fmt::Display for OutArray {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.inner.fmt(f)
    }
}

impl TypeStringHint for OutArray {
    fn type_string() -> String {
        VariantArray::type_string()
    }
}

impl Var for OutArray {
    fn get_property(&self) -> Self::Via {
        self.to_godot()
    }

    fn set_property(&mut self, value: Self::Via) {
        *self = FromGodot::from_godot(value)
    }

    #[cfg(since_api = "4.2")]
    fn property_hint() -> PropertyHintInfo {
        VariantArray::property_hint()
    }
}

impl Export for OutArray {
    fn default_export_info() -> PropertyHintInfo {
        VariantArray::default_export_info()
    }
}

impl GodotType for OutArray {
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
        VariantArray::godot_type_name()
    }
}

impl GodotFfiVariant for OutArray {
    fn ffi_to_variant(&self) -> Variant {
        VariantArray::ffi_to_variant(&self.inner)
    }

    fn ffi_from_variant(variant: &Variant) -> Result<Self, ConvertError> {
        // SAFETY: All element types are valid for OutArray.
        let result = unsafe { VariantArray::unchecked_from_variant(variant) };
        result.map(|inner| Self { inner })
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

/// An iterator over typed elements of an [`Array`].
pub struct Iter<'a> {
    inner: super::array::Iter<'a, Variant>,
}

impl<'a> Iterator for Iter<'a> {
    type Item = Variant;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}
