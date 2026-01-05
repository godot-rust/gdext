/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::fmt;

use godot_ffi as sys;
use sys::{ffi_methods, GodotFfi};

use crate::builtin::iter::ArrayFunctionalOps;
use crate::builtin::*;
use crate::meta;
use crate::meta::error::ConvertError;
use crate::meta::{
    ArrayElement, ElementType, FromGodot, GodotConvert, GodotFfiVariant, GodotType, ToGodot,
};
use crate::registry::property::SimpleVar;

/// Covariant `Array` that can be either typed or untyped.
///
/// Unlike [`Array<T>`], which carries compile-time type information, `AnyArray` is a type-erased version of arrays.
/// It can point to any `Array<T>`, for both typed and untyped arrays. See [`Array`: Element type](struct.Array.html#element-type) section.
///
/// # Covariance
/// In GDScript, the subtyping relationship is modeled incorrectly for arrays:
/// ```gdscript
/// var typed: Array[int] = [1, 2, 3]
/// var untyped: Array = typed   # Implicit "upcast" to Array[Variant].
///
/// untyped.append("hello")      # Not detected by GDScript parser (no-op at runtime).
/// ```
///
/// godot-rust on the other hand introduces a new type `AnyArray`, which can store _any_ array, typed or untyped.
/// `AnyArray` thus provides operations that are valid regardless of the type, e.g. `len()`, `clear()` or `shuffle()`.
/// Methods, which can be more concrete on `Array<T>` by using `T` (e.g. `pick_random() -> Option<T>`), exist on both types.
///
/// `AnyArray` does not provide any operations where data flows _in_ to the array, such as `push()` or `insert()`.
///
/// ## Conversions
/// See the [corresponding section in `Array`](struct.Array.html#conversions-between-arrays).
#[derive(PartialEq, PartialOrd)]
#[repr(transparent)] // Guarantees same layout as VarArray, enabling Deref from Array<T>.
pub struct AnyArray {
    array: VarArray,
}

impl AnyArray {
    pub(super) fn from_typed_or_untyped<T: ArrayElement>(array: Array<T>) -> Self {
        // SAFETY: Array<Variant> is not accessed as such, but immediately wrapped in AnyArray.
        let inner = unsafe { array.assume_type::<Variant>() };

        Self { array: inner }
    }

    /// Creates an empty untyped `AnyArray`.
    pub(crate) fn new_untyped() -> Self {
        Self {
            array: VarArray::default(),
        }
    }

    fn from_opaque(opaque: sys::types::OpaqueArray) -> Self {
        Self {
            array: VarArray::from_opaque(opaque),
        }
    }

    /// ⚠️ Returns the value at the specified index.
    ///
    /// This replaces the `Index` trait, which cannot be implemented for `Array`, as it stores variants and not references.
    ///
    /// # Panics
    /// If `index` is out of bounds. To handle out-of-bounds access fallibly, use [`get()`](Self::get) instead.
    pub fn at(&self, index: usize) -> Variant {
        self.array.at(index)
    }

    /// Returns the value at the specified index, or `None` if the index is out-of-bounds.
    ///
    /// If you know the index is correct, use [`at()`](Self::at) instead.
    pub fn get(&self, index: usize) -> Option<Variant> {
        self.array.get(index)
    }

    /// Returns `true` if the array contains the given value. Equivalent of `has` in GDScript.
    pub fn contains(&self, value: &Variant) -> bool {
        self.array.contains(value)
    }

    /// Returns the number of times a value is in the array.
    pub fn count(&self, value: &Variant) -> usize {
        self.array.count(value)
    }

    /// Returns the number of elements in the array. Equivalent of `size()` in Godot.
    ///
    /// Retrieving the size incurs an FFI call. If you know the size hasn't changed, you may consider storing
    /// it in a variable. For loops, prefer iterators.
    #[doc(alias = "size")]
    pub fn len(&self) -> usize {
        to_usize(self.array.as_inner().size())
    }

    /// Returns `true` if the array is empty.
    ///
    /// Checking for emptiness incurs an FFI call. If you know the size hasn't changed, you may consider storing
    /// it in a variable. For loops, prefer iterators.
    pub fn is_empty(&self) -> bool {
        self.array.as_inner().is_empty()
    }

    /// Returns a 32-bit integer hash value representing the array and its contents.
    ///
    /// Arrays with equal content will always produce identical hash values. However, the reverse is not true:
    /// Different arrays can have identical hash values due to hash collisions.
    pub fn hash_u32(&self) -> u32 {
        self.array
            .as_inner()
            .hash()
            .try_into()
            .expect("Godot hashes are uint32_t")
    }

    /// Returns the first element in the array, or `None` if the array is empty.
    #[doc(alias = "first")]
    pub fn front(&self) -> Option<Variant> {
        self.array.front()
    }

    /// Returns the last element in the array, or `None` if the array is empty.
    #[doc(alias = "last")]
    pub fn back(&self) -> Option<Variant> {
        self.array.back()
    }

    /// Clears the array, removing all elements.
    pub fn clear(&mut self) {
        self.balanced_ensure_mutable();

        // SAFETY: No new values are written to the array, we only remove values from the array.
        unsafe { self.as_inner_mut() }.clear();
    }

    /// Removes and returns the last element of the array. Returns `None` if the array is empty.
    ///
    /// _Godot equivalent: `pop_back`_
    #[doc(alias = "pop_back")]
    pub fn pop(&mut self) -> Option<Variant> {
        self.array.pop()
    }

    /// Removes and returns the first element of the array, in O(n). Returns `None` if the array is empty.
    ///
    /// Note: On large arrays, this method is much slower than `pop()` as it will move all the
    /// array's elements. The larger the array, the slower `pop_front()` will be.
    pub fn pop_front(&mut self) -> Option<Variant> {
        self.array.pop_front()
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
        self.array.remove(index)
    }

    /// Removes the first occurrence of a value from the array.
    ///
    /// If the value does not exist in the array, nothing happens. To remove an element by index, use [`remove()`][Self::remove] instead.
    ///
    /// On large arrays, this method is much slower than [`pop()`][Self::pop], as it will move all the array's
    /// elements after the removed element.
    pub fn erase(&mut self, value: &Variant) {
        self.array.erase(value)
    }

    /// Shrinks the array down to `new_size`.
    ///
    /// This will only change the size of the array if `new_size` is smaller than the current size. Returns `true` if the array was shrunk.
    ///
    /// If you want to increase the size of the array, use [`resize`](Array::resize) instead.
    #[doc(alias = "resize")]
    pub fn shrink(&mut self, new_size: usize) -> bool {
        self.balanced_ensure_mutable();

        if new_size >= self.len() {
            return false;
        }

        // SAFETY: Since `new_size` is less than the current size, we'll only be removing elements from the array.
        unsafe { self.as_inner_mut() }.resize(to_i64(new_size));

        true
    }

    /// Returns a shallow copy, sharing reference types (`Array`, `Dictionary`, `Object`...) with the original array.
    ///
    /// This operation retains the dynamic [element type][Self::element_type]: copying `Array<T>` will yield another `Array<T>`.
    ///
    /// To create a deep copy, use [`duplicate_deep()`][Self::duplicate_deep] instead.
    /// To create a new reference to the same array data, use [`clone()`][Clone::clone].
    pub fn duplicate_shallow(&self) -> AnyArray {
        self.array.duplicate_shallow().upcast_any_array()
    }

    /// Returns a deep copy, duplicating nested `Array`/`Dictionary` elements but keeping `Object` elements shared.
    ///
    /// This operation retains the dynamic [element type][Self::element_type]: copying `Array<T>` will yield another `Array<T>`.
    ///
    /// To create a shallow copy, use [`duplicate_shallow()`][Self::duplicate_shallow] instead.
    /// To create a new reference to the same array data, use [`clone()`][Clone::clone].
    pub fn duplicate_deep(&self) -> Self {
        self.array.duplicate_deep().upcast_any_array()
    }

    /// Returns a sub-range as a new array.
    ///
    /// Array elements are copied to the slice, but any reference types (such as `Array`,
    /// `Dictionary` and `Object`) will still refer to the same value. To create a deep copy, use
    /// [`subarray_deep()`][Self::subarray_deep] instead.
    ///
    /// _Godot equivalent: `slice`_
    #[doc(alias = "slice")]
    pub fn subarray_shallow(&self, range: impl meta::SignedRange, step: Option<i32>) -> Self {
        let sliced = self.array.subarray_shallow(range, step);
        sliced.upcast_any_array()
    }

    /// Returns a sub-range as a new `Array`.
    ///
    /// All nested arrays and dictionaries are duplicated and will not be shared with the original
    /// array. Note that any `Object`-derived elements will still be shallow copied. To create a
    /// shallow copy, use [`subarray_shallow()`][Self::subarray_shallow] instead.
    ///
    /// _Godot equivalent: `slice` with `deep: true`_
    #[doc(alias = "slice")]
    pub fn subarray_deep(&self, range: impl meta::SignedRange, step: Option<i32>) -> Self {
        let sliced = self.array.subarray_deep(range, step);
        sliced.upcast_any_array()
    }

    /// Returns an non-exclusive iterator over the elements of the `Array`.
    ///
    /// Takes the array by reference but returns its elements by value, since they are internally converted from `Variant`.
    ///
    /// Notice that it's possible to modify the `Array` through another reference while iterating over it. This will not result
    /// in unsoundness or crashes, but will cause the iterator to behave in an unspecified way.
    pub fn iter_shared(&self) -> Iter<'_> {
        Iter {
            inner: self.array.iter_shared(),
        }
    }

    /// Returns the minimum value contained in the array if all elements are of comparable types.
    ///
    /// If the elements can't be compared or the array is empty, `None` is returned.
    pub fn min(&self) -> Option<Variant> {
        self.array.min()
    }

    /// Returns the maximum value contained in the array if all elements are of comparable types.
    ///
    /// If the elements can't be compared or the array is empty, `None` is returned.
    pub fn max(&self) -> Option<Variant> {
        self.array.max()
    }

    /// Returns a random element from the array, or `None` if it is empty.
    pub fn pick_random(&self) -> Option<Variant> {
        self.array.pick_random()
    }

    /// Searches the array for the first occurrence of a value and returns its index, or `None` if
    /// not found. Starts searching at index `from`; pass `None` to search the entire array.
    // TODO test if method works.
    pub fn find(&self, value: &Variant, from: Option<usize>) -> Option<usize> {
        self.array.find(value, from)
    }

    /// Searches the array backwards for the last occurrence of a value and returns its index, or
    /// `None` if not found. Starts searching at index `from`; pass `None` to search the entire array.
    // TODO test if method works.
    pub fn rfind(&self, value: &Variant, from: Option<usize>) -> Option<usize> {
        self.array.rfind(value, from)
    }

    /// Finds the index of an existing value in a sorted array using binary search.
    /// Equivalent of `bsearch` in GDScript.
    ///
    /// If the value is not present in the array, returns the insertion index that
    /// would maintain sorting order.
    ///
    /// Calling `bsearch` on an unsorted array results in unspecified behavior.
    pub fn bsearch(&self, value: &Variant) -> usize {
        self.array.bsearch(value)
    }

    /// Reverses the order of the elements in the array.
    pub fn reverse(&mut self) {
        self.balanced_ensure_mutable();

        // SAFETY: We do not write any values that don't already exist in the array, so all values have the correct type.
        unsafe { self.as_inner_mut() }.reverse();
    }

    /// Sorts the array.
    ///
    /// The sorting algorithm used is not [stable](https://en.wikipedia.org/wiki/Sorting_algorithm#Stability).
    /// This means that values considered equal may have their order changed when using `sort_unstable()`. For most variant types,
    /// this distinction should not matter though.
    ///
    /// See also: [`Array::sort_unstable_by()`][Array::sort_unstable_by], [`sort_unstable_custom()`][Self::sort_unstable_custom].
    ///
    /// _Godot equivalent: `Array.sort()`_
    #[doc(alias = "sort")]
    pub fn sort_unstable(&mut self) {
        self.balanced_ensure_mutable();

        // SAFETY: We do not write any values that don't already exist in the array, so all values have the correct type.
        unsafe { self.as_inner_mut() }.sort();
    }

    /// Sorts the array, using type-unsafe `Callable` comparator.
    ///
    /// The callable expects two parameters `(lhs, rhs)` and should return a bool `lhs < rhs`.
    ///
    /// The sorting algorithm used is not [stable](https://en.wikipedia.org/wiki/Sorting_algorithm#Stability).
    /// This means that values considered equal may have their order changed when using `sort_unstable_custom()`. For most variant types,
    /// this distinction should not matter though.
    ///
    /// Type-safe alternatives: [`Array::sort_unstable_by()`][Array::sort_unstable_by], [`sort_unstable()`][Self::sort_unstable].
    ///
    /// _Godot equivalent: `Array.sort_custom()`_
    #[doc(alias = "sort_custom")]
    pub fn sort_unstable_custom(&mut self, func: &Callable) {
        self.balanced_ensure_mutable();

        // SAFETY: We do not write any values that don't already exist in the array, so all values have the correct type.
        unsafe { self.as_inner_mut() }.sort_custom(func);
    }

    /// Shuffles the array such that the items will have a random order.
    ///
    /// This method uses the global random number generator common to methods such as `randi`. Call `randomize` to
    /// ensure that a new seed will be used each time, if you want non-reproducible shuffling.
    pub fn shuffle(&mut self) {
        self.balanced_ensure_mutable();

        // SAFETY: We do not write any values that don't already exist in the array, so all values have the correct type.
        unsafe { self.as_inner_mut() }.shuffle();
    }

    /// Accesses Godot's functional-programming APIs (filter, map, reduce, etc.).
    pub fn functional_ops(&self) -> ArrayFunctionalOps<'_, Variant> {
        self.array.functional_ops()
    }

    /// Returns the runtime element type information for this array.
    ///
    /// The result is generally cached, so feel free to call this method repeatedly.
    pub fn element_type(&self) -> ElementType {
        ElementType::get_or_compute_cached(
            &self.array.cached_element_type,
            || self.array.as_inner().get_typed_builtin(),
            || self.array.as_inner().get_typed_class_name(),
            || self.array.as_inner().get_typed_script(),
        )
    }

    /// Returns `true` if the array is read-only. See [`make_read_only`][crate::builtin::Array::make_read_only].
    pub fn is_read_only(&self) -> bool {
        self.array.as_inner().is_read_only()
    }

    /// Best-effort mutability check.
    ///
    /// # Panics
    /// In debug builds, panics if the array is read-only.
    fn balanced_ensure_mutable(&self) {
        sys::balanced_assert!(
            !self.is_read_only(),
            "mutating operation on read-only array"
        );
    }

    /// # Safety
    /// Must not be used for any "input" operations, moving elements into the array -- this would break covariance.
    #[doc(hidden)]
    pub unsafe fn as_inner_mut(&self) -> inner::InnerArray<'_> {
        inner::InnerArray::from_outer_typed(&self.array)
    }

    /// Converts to `Array<T>` if the runtime type matches.
    ///
    /// If `T=Variant`, this will attempt to "downcast" to an untyped array, identical to [`try_cast_var_array()`][Self::try_cast_var_array].
    ///
    /// Returns `Err(self)` if the array's dynamic type differs from `T`. Check [`element_type()`][Self::element_type]
    /// before calling to determine what type the array actually holds.
    ///
    /// Consumes `self`, to avoid incrementing reference-count. Use `clone()` if you need to keep the original. Using `self` also has the nice
    /// side effect that this method cannot be called on concrete `Array<T>` types, as `Deref` only operates on references, not values.
    // Naming: not `try_into_typed` because T can be Variant.
    pub fn try_cast_array<T: ArrayElement>(self) -> Result<Array<T>, Self> {
        let from_type = self.array.element_type();
        let to_type = ElementType::of::<T>();

        if from_type == to_type {
            // SAFETY: just checked type.
            let array = unsafe { self.array.assume_type::<T>() };
            Ok(array)
        } else {
            // If we add ConvertError here:
            // let mismatch = ArrayMismatch { expected: to_type, actual: from_type };
            // Err(ConvertError::with_kind(ErrorKind::FromAnyArray(mismatch)))

            Err(self)
        }
    }

    /// Converts to an untyped `VarArray` if the array is untyped.
    ///
    /// This is a shorthand for [`try_cast_array::<Variant>()`][Self::try_cast_array].
    ///
    /// Consumes `self`, to avoid incrementing reference-count. Use `clone()` if you need to keep the original. Using `self` also has the nice
    /// side effect that this method cannot be called on concrete `Array<T>` types, as `Deref` only operates on references, not values.
    pub fn try_cast_var_array(self) -> Result<VarArray, Self> {
        self.try_cast_array::<Variant>()
    }

    // If we add direct-conversion methods that panic, we can use meta::element_godot_type_name::<T>() to mention type in case of mismatch.
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Traits

// SAFETY: See VarArray.
//
// We cannot provide GodotConvert with Via=VarArray, because ToGodot::to_godot() would otherwise enable a safe conversion from AnyArray to
// VarArray, which is not sound.
unsafe impl GodotFfi for AnyArray {
    const VARIANT_TYPE: sys::ExtVariantType = sys::ExtVariantType::Concrete(VariantType::ARRAY);

    // No Default trait, thus manually defining this and ffi_methods!.
    unsafe fn new_with_init(init_fn: impl FnOnce(sys::GDExtensionTypePtr)) -> Self {
        let mut result = Self::new_untyped();
        init_fn(result.sys_mut());
        result
    }

    // Manually forwarding these, since no Opaque.
    fn sys(&self) -> sys::GDExtensionConstTypePtr {
        self.array.sys()
    }

    fn sys_mut(&mut self) -> sys::GDExtensionTypePtr {
        self.array.sys_mut()
    }

    unsafe fn move_return_ptr(self, dst: sys::GDExtensionTypePtr, call_type: sys::PtrcallType) {
        self.array.move_return_ptr(dst, call_type)
    }

    ffi_methods! { type sys::GDExtensionTypePtr = *mut Opaque;
        fn new_from_sys;
        fn new_with_uninit;
        fn from_arg_ptr;
    }
}

impl Clone for AnyArray {
    fn clone(&self) -> Self {
        // SAFETY: we don't want to check that static type (Variant) matches dynamic type (anything), because all types are valid in AnyArray.
        let inner = unsafe { VarArray::clone_unchecked(&self.array) };

        Self { array: inner }
    }
}

// Only implement for untyped arrays; typed arrays cannot be nested in Godot.
impl meta::sealed::Sealed for AnyArray {}

impl ArrayElement for AnyArray {}

impl GodotConvert for AnyArray {
    type Via = Self;
}

impl ToGodot for AnyArray {
    type Pass = meta::ByValue;

    fn to_godot(&self) -> meta::ToArg<'_, Self::Via, Self::Pass> {
        self.clone()
    }

    fn to_variant(&self) -> Variant {
        self.ffi_to_variant()
    }
}

impl FromGodot for AnyArray {
    fn try_from_godot(via: Self::Via) -> Result<Self, ConvertError> {
        Ok(via)
    }
}

impl SimpleVar for AnyArray {}

impl fmt::Debug for AnyArray {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.array.fmt(f)
    }
}

impl fmt::Display for AnyArray {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.array.fmt(f)
    }
}

impl GodotType for AnyArray {
    type Ffi = Self;

    type ToFfi<'f>
        = meta::RefArg<'f, AnyArray>
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

    fn godot_type_name() -> String {
        VarArray::godot_type_name()
    }
}

impl GodotFfiVariant for AnyArray {
    fn ffi_to_variant(&self) -> Variant {
        VarArray::ffi_to_variant(&self.array)
    }

    fn ffi_from_variant(variant: &Variant) -> Result<Self, ConvertError> {
        // SAFETY: All element types are valid for AnyArray.
        let result = unsafe { VarArray::unchecked_from_variant(variant) };
        result.map(|inner| Self { array: inner })
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

/// An iterator over elements of an [`AnyArray`].
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
