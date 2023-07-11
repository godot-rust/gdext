/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use godot_ffi as sys;

use crate::builtin::meta::VariantMetadata;
use crate::builtin::*;
use crate::obj::Share;
use crate::property::{Export, ExportInfo, Property, TypeStringHint};
use std::fmt;
use std::marker::PhantomData;
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
/// runtime checks that no values of the wrong type are put into the array. We represent this as
/// `Array<T>`, where the type `T` implements `VariantMetadata`, `FromVariant` and `ToVariant`.
///
/// # Reference semantics
///
/// Like in GDScript, `Array` acts as a reference type: multiple `Array` instances may
/// refer to the same underlying array, and changes to one are visible in the other.
///
/// To create a copy that shares data with the original array, use [`Share::share()`]. If you want
/// to create a copy of the data, use [`duplicate_shallow()`] or [`duplicate_deep()`].
///
/// # Thread safety
///
/// Usage is safe if the `Array` is used on a single thread only. Concurrent reads on
/// different threads are also safe, but any writes must be externally synchronized. The Rust
/// compiler will enforce this as long as you use only Rust threads, but it cannot protect against

/// concurrent modification on other threads (e.g. created through GDScript).

// `T` must be restricted to `VariantMetadata` in the type, because `Drop` can only be implemented
// for `T: VariantMetadata` because `drop()` requires `sys_mut()`, which is on the `GodotFfi`
// trait, whose `from_sys_init()` requires `Default`, which is only implemented for `T:
// VariantMetadata`. Whew. This could be fixed by splitting up `GodotFfi` if desired.
#[repr(C)]
pub struct Array<T: VariantMetadata> {
    opaque: sys::types::OpaqueArray,
    _phantom: PhantomData<T>,
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

impl<T: VariantMetadata> Array<T> {
    fn from_opaque(opaque: sys::types::OpaqueArray) -> Self {
        // Note: type is not yet checked at this point, because array has not yet been initialized!
        Self {
            opaque,
            _phantom: PhantomData,
        }
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
            "Array index {index} is out of bounds: length is {len}",
        );
    }

    /// Returns a pointer to the element at the given index.
    ///
    /// # Panics
    ///
    /// If `index` is out of bounds.
    fn ptr(&self, index: usize) -> *const Variant {
        let ptr = self.ptr_or_null(index);
        assert!(
            !ptr.is_null(),
            "Array index {index} out of bounds (len {len})",
            len = self.len(),
        );
        ptr
    }

    /// Returns a pointer to the element at the given index, or null if out of bounds.
    fn ptr_or_null(&self, index: usize) -> *const Variant {
        // SAFETY: array_operator_index_const returns null for invalid indexes.
        let variant_ptr = unsafe {
            let index = to_i64(index);
            interface_fn!(array_operator_index_const)(self.sys(), index)
        };

        Variant::ptr_from_sys(variant_ptr)
    }

    /// Returns a mutable pointer to the element at the given index.
    ///
    /// # Panics
    ///
    /// If `index` is out of bounds.
    fn ptr_mut(&self, index: usize) -> *mut Variant {
        let ptr = self.ptr_mut_or_null(index);
        assert!(
            !ptr.is_null(),
            "Array index {index} out of bounds (len {len})",
            len = self.len(),
        );
        ptr
    }

    /// Returns a pointer to the element at the given index, or null if out of bounds.
    fn ptr_mut_or_null(&self, index: usize) -> *mut Variant {
        // SAFETY: array_operator_index returns null for invalid indexes.
        let variant_ptr = unsafe {
            let index = to_i64(index);
            interface_fn!(array_operator_index)(self.sys(), index)
        };

        Variant::ptr_from_sys_mut(variant_ptr)
    }

    #[doc(hidden)]
    pub fn as_inner(&self) -> inner::InnerArray {
        // SAFETY: The memory layout of `TypedArray<T>` does not depend on `T`.
        inner::InnerArray::from_outer_typed(self)
    }

    /// Changes the generic type on this array, without changing its contents. Needed for API
    /// functions that return a variant array even though we know its type, and for API functions
    /// that take a variant array even though we want to pass a typed one.
    ///
    /// This is marked `unsafe` since it can be used to break the invariant that a `TypedArray<T>`
    /// always holds a Godot array whose runtime type is `T`.
    ///
    /// # Safety
    ///
    /// In and of itself, calling this does not result in undefined behavior. However:
    /// - If `T` is not `Variant`, the returned array should not be written to, because the runtime
    ///   type check may fail.
    /// - If `U` is not `Variant`, the returned array should not be read from, because conversion
    ///   from variants may fail.
    /// In the current implementation, both cases will produce a panic rather than undefined
    /// behavior, but this should not be relied upon.
    unsafe fn assume_type<U: VariantMetadata>(self) -> Array<U> {
        // SAFETY: The memory layout of `TypedArray<T>` does not depend on `T`.
        unsafe { std::mem::transmute(self) }
    }
}

impl<T: VariantMetadata> Array<T> {
    /// Constructs an empty `Array`.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns a shallow copy of the array. All array elements are copied, but any reference types
    /// (such as `Array`, `Dictionary` and `Object`) will still refer to the same value.
    ///
    /// To create a deep copy, use [`duplicate_deep()`] instead. To create a new reference to the
    /// same array data, use [`share()`].
    pub fn duplicate_shallow(&self) -> Self {
        let duplicate: VariantArray = self.as_inner().duplicate(false);
        // SAFETY: duplicate() returns a typed array with the same type as Self
        unsafe { duplicate.assume_type() }
    }

    /// Returns a deep copy of the array. All nested arrays and dictionaries are duplicated and
    /// will not be shared with the original array. Note that any `Object`-derived elements will
    /// still be shallow copied.
    ///
    /// To create a shallow copy, use [`duplicate_shallow()`] instead. To create a new reference to
    /// the same array data, use [`share()`].
    pub fn duplicate_deep(&self) -> Self {
        let duplicate: VariantArray = self.as_inner().duplicate(true);
        // SAFETY: duplicate() returns a typed array with the same type as Self
        unsafe { duplicate.assume_type() }
    }

    /// Returns a sub-range `begin..end`, as a new array.
    ///
    /// The values of `begin` (inclusive) and `end` (exclusive) will be clamped to the array size.
    ///
    /// If specified, `step` is the relative index between source elements. It can be negative,
    /// in which case `begin` must be higher than `end`. For example,
    /// `TypedArray::from(&[0, 1, 2, 3, 4, 5]).slice(5, 1, -2)` returns `[5, 3]`.
    ///
    /// Array elements are copied to the slice, but any reference types (such as `Array`,
    /// `Dictionary` and `Object`) will still refer to the same value. To create a deep copy, use
    /// [`subarray_deep()`] instead.
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
    /// `TypedArray::from(&[0, 1, 2, 3, 4, 5]).slice(5, 1, -2)` returns `[5, 3]`.
    ///
    /// All nested arrays and dictionaries are duplicated and will not be shared with the original
    /// array. Note that any `Object`-derived elements will still be shallow copied. To create a
    /// shallow copy, use [`subarray_shallow()`] instead.
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

        let subarray: VariantArray =
            self.as_inner()
                .slice(to_i64(begin), to_i64(end), step.try_into().unwrap(), deep);

        // SAFETY: slice() returns a typed array with the same type as Self
        unsafe { subarray.assume_type() }
    }

    /// Appends another array at the end of this array. Equivalent of `append_array` in GDScript.
    pub fn extend_array(&mut self, other: Array<T>) {
        // SAFETY: Read-only arrays are covariant: conversion to a variant array is fine as long as
        // we don't insert values into it afterwards, and `append_array()` doesn't do that.
        let other: VariantArray = unsafe { other.assume_type::<Variant>() };
        self.as_inner().append_array(other);
    }

    /// Returns the runtime type info of this array.
    fn type_info(&self) -> TypeInfo {
        let variant_type = VariantType::from_sys(
            self.as_inner().get_typed_builtin() as sys::GDExtensionVariantType
        );
        let class_name = self.as_inner().get_typed_class_name();

        TypeInfo {
            variant_type,
            class_name,
        }
    }

    /// Checks that the inner array has the correct type set on it for storing elements of type `T`.
    fn with_checked_type(self) -> Result<Self, VariantConversionError> {
        if self.type_info() == TypeInfo::new::<T>() {
            Ok(self)
        } else {
            Err(VariantConversionError::BadType)
        }
    }

    /// Sets the type of the inner array. Can only be called once, directly after creation.
    fn init_inner_type(&mut self) {
        debug_assert!(self.is_empty());
        debug_assert!(!self.type_info().is_typed());

        let type_info = TypeInfo::new::<T>();
        if type_info.is_typed() {
            let script = Variant::nil();
            unsafe {
                interface_fn!(array_set_typed)(
                    self.sys(),
                    type_info.variant_type.sys(),
                    type_info.class_name.string_sys(),
                    script.var_sys(),
                );
            }
        }
    }
}

impl<T: VariantMetadata + FromVariant> Array<T> {
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

    /// Returns the value at the specified index.
    ///
    /// # Panics
    ///
    /// If `index` is out of bounds.
    pub fn get(&self, index: usize) -> T {
        // Panics on out-of-bounds
        let ptr = self.ptr(index);

        // SAFETY: `ptr()` just verified that the index is not out of bounds.
        let variant = unsafe { &*ptr };
        T::from_variant(variant)
    }

    /// Returns the first element in the array, or `None` if the array is empty. Equivalent of
    /// `front()` in GDScript.
    pub fn first(&self) -> Option<T> {
        (!self.is_empty()).then(|| {
            let variant = self.as_inner().front();
            T::from_variant(&variant)
        })
    }

    /// Returns the last element in the array, or `None` if the array is empty. Equivalent of
    /// `back()` in GDScript.
    pub fn last(&self) -> Option<T> {
        (!self.is_empty()).then(|| {
            let variant = self.as_inner().back();
            T::from_variant(&variant)
        })
    }

    /// Returns the minimum value contained in the array if all elements are of comparable types.
    /// If the elements can't be compared or the array is empty, `None` is returned.
    pub fn min(&self) -> Option<T> {
        let min = self.as_inner().min();
        (!min.is_nil()).then(|| T::from_variant(&min))
    }

    /// Returns the maximum value contained in the array if all elements are of comparable types.
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

    /// Removes and returns the last element of the array. Returns `None` if the array is empty.
    /// Equivalent of `pop_back` in GDScript.
    pub fn pop(&mut self) -> Option<T> {
        (!self.is_empty()).then(|| {
            let variant = self.as_inner().pop_back();
            T::from_variant(&variant)
        })
    }

    /// Removes and returns the first element of the array. Returns `None` if the array is empty.
    ///
    /// Note: On large arrays, this method is much slower than `pop` as it will move all the
    /// array's elements. The larger the array, the slower `pop_front` will be.
    pub fn pop_front(&mut self) -> Option<T> {
        (!self.is_empty()).then(|| {
            let variant = self.as_inner().pop_front();
            T::from_variant(&variant)
        })
    }

    /// Removes and returns the element at the specified index. Equivalent of `pop_at` in GDScript.
    ///
    /// On large arrays, this method is much slower than `pop_back` as it will move all the array's
    /// elements after the removed element. The larger the array, the slower `remove` will be.
    ///
    /// # Panics
    ///
    /// If `index` is out of bounds.
    pub fn remove(&mut self, index: usize) -> T {
        self.check_bounds(index);
        let variant = self.as_inner().pop_at(to_i64(index));
        T::from_variant(&variant)
    }
}

impl<T: VariantMetadata + ToVariant> Array<T> {
    /// Finds the index of an existing value in a sorted array using binary search. Equivalent of
    /// `bsearch` in GDScript.
    ///
    /// If the value is not present in the array, returns the insertion index that would maintain
    /// sorting order.
    ///
    /// Calling `binary_search` on an unsorted array results in unspecified behavior.
    pub fn binary_search(&self, value: &T) -> usize {
        to_usize(self.as_inner().bsearch(value.to_variant(), true))
    }

    /// Returns the number of times a value is in the array.
    pub fn count(&self, value: &T) -> usize {
        to_usize(self.as_inner().count(value.to_variant()))
    }

    /// Returns `true` if the array contains the given value. Equivalent of `has` in GDScript.
    pub fn contains(&self, value: &T) -> bool {
        self.as_inner().has(value.to_variant())
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

    /// Sets the value at the specified index.
    ///
    /// # Panics
    ///
    /// If `index` is out of bounds.
    pub fn set(&mut self, index: usize, value: T) {
        let ptr_mut = self.ptr_mut(index);
        // SAFETY: `ptr_mut` just checked that the index is not out of bounds.
        unsafe {
            *ptr_mut = value.to_variant();
        }
    }

    /// Appends an element to the end of the array. Equivalent of `append` and `push_back` in
    /// GDScript.
    pub fn push(&mut self, value: T) {
        self.as_inner().push_back(value.to_variant());
    }

    /// Adds an element at the beginning of the array. See also `push`.
    ///
    /// Note: On large arrays, this method is much slower than `push` as it will move all the
    /// array's elements. The larger the array, the slower `push_front` will be.
    pub fn push_front(&mut self, value: T) {
        self.as_inner().push_front(value.to_variant());
    }

    /// Inserts a new element at a given index in the array. The index must be valid, or at the end
    /// of the array (`index == len()`).
    ///
    /// Note: On large arrays, this method is much slower than `push` as it will move all the
    /// array's elements after the inserted element. The larger the array, the slower `insert` will
    /// be.
    pub fn insert(&mut self, index: usize, value: T) {
        let len = self.len();
        assert!(
            index <= len,
            "TypedArray insertion index {index} is out of bounds: length is {len}",
        );
        self.as_inner().insert(to_i64(index), value.to_variant());
    }

    /// Removes the first occurrence of a value from the array. If the value does not exist in the
    /// array, nothing happens. To remove an element by index, use `remove` instead.
    ///
    /// On large arrays, this method is much slower than `pop_back` as it will move all the array's
    /// elements after the removed element. The larger the array, the slower `remove` will be.
    pub fn erase(&mut self, value: &T) {
        self.as_inner().erase(value.to_variant());
    }

    /// Assigns the given value to all elements in the array. This can be used together with
    /// `resize` to create an array with a given size and initialized elements.
    pub fn fill(&mut self, value: &T) {
        self.as_inner().fill(value.to_variant());
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Traits

// Godot has some inconsistent behavior around NaN values. In GDScript, `NAN == NAN` is `false`,
// but `[NAN] == [NAN]` is `true`. If they decide to make all NaNs equal, we can implement `Eq` and
// `Ord`; if they decide to make all NaNs unequal, we can remove this comment.
//
// impl<T> Eq for TypedArray<T> {}
//
// impl<T> Ord for TypedArray<T> {
//     ...
// }

// SAFETY:
// - `move_return_ptr`
//   Nothing special needs to be done beyond a `std::mem::swap` when returning an Array.
//   So we can just use `ffi_methods`.
//
// - `from_arg_ptr`
//   Arrays are properly initialized through a `from_sys` call, but the ref-count should be incremented
//   as that is the callee's responsibility. Which we do by calling `std::mem::forget(array.share())`.
unsafe impl<T: VariantMetadata> GodotFfi for Array<T> {
    ffi_methods! { type sys::GDExtensionTypePtr = *mut Opaque;
        fn from_sys;
        fn sys;
        fn from_sys_init;
        fn move_return_ptr;
    }

    unsafe fn from_sys_init_default(init_fn: impl FnOnce(sys::GDExtensionTypePtr)) -> Self {
        let mut result = Self::default();
        init_fn(result.sys_mut());
        result
    }

    unsafe fn from_arg_ptr(ptr: sys::GDExtensionTypePtr, _call_type: sys::PtrcallType) -> Self {
        let array = Self::from_sys(ptr);
        std::mem::forget(array.share());
        array
    }
}

impl<T: VariantMetadata> fmt::Debug for Array<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // Going through `Variant` because there doesn't seem to be a direct way.
        write!(f, "{:?}", self.to_variant().stringify())
    }
}

/// Creates a new reference to the data in this array. Changes to the original array will be
/// reflected in the copy and vice versa.
///
/// To create a (mostly) independent copy instead, see [`VariantArray::duplicate_shallow()`] and
/// [`VariantArray::duplicate_deep()`].
impl<T: VariantMetadata> Share for Array<T> {
    fn share(&self) -> Self {
        let array = unsafe {
            Self::from_sys_init(|self_ptr| {
                let ctor = ::godot_ffi::builtin_fn!(array_construct_copy);
                let args = [self.sys_const()];
                ctor(self_ptr, args.as_ptr());
            })
        };
        array
            .with_checked_type()
            .expect("copied array should have same type as original array")
    }
}

impl<T: VariantMetadata + TypeStringHint> TypeStringHint for Array<T> {
    fn type_string() -> String {
        format!("{}:{}", sys::VariantType::Array as i32, T::type_string())
    }
}

impl<T: VariantMetadata> Property for Array<T> {
    type Intermediate = Self;

    fn get_property(&self) -> Self::Intermediate {
        self.share()
    }

    fn set_property(&mut self, value: Self::Intermediate) {
        *self = value;
    }
}

impl<T: VariantMetadata + TypeStringHint> Export for Array<T> {
    fn default_export_info() -> ExportInfo {
        ExportInfo {
            hint: crate::engine::global::PropertyHint::PROPERTY_HINT_TYPE_STRING,
            hint_string: T::type_string().into(),
        }
    }
}

impl Export for Array<Variant> {
    fn default_export_info() -> ExportInfo {
        ExportInfo::with_hint_none()
    }
}

impl<T: VariantMetadata> Default for Array<T> {
    #[inline]
    fn default() -> Self {
        let mut array = unsafe {
            Self::from_sys_init(|self_ptr| {
                let ctor = sys::builtin_fn!(array_construct_default);
                ctor(self_ptr, std::ptr::null_mut())
            })
        };
        array.init_inner_type();
        array
    }
}

impl<T: VariantMetadata> Drop for Array<T> {
    #[inline]
    fn drop(&mut self) {
        unsafe {
            let array_destroy = sys::builtin_fn!(array_destroy);
            array_destroy(self.sys_mut());
        }
    }
}

impl<T: VariantMetadata> VariantMetadata for Array<T> {
    fn variant_type() -> VariantType {
        VariantType::Array
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Conversion traits

impl<T: VariantMetadata> ToVariant for Array<T> {
    fn to_variant(&self) -> Variant {
        unsafe {
            Variant::from_var_sys_init(|variant_ptr| {
                let array_to_variant = sys::builtin_fn!(array_to_variant);
                array_to_variant(variant_ptr, self.sys());
            })
        }
    }
}

impl<T: VariantMetadata> FromVariant for Array<T> {
    fn try_from_variant(variant: &Variant) -> Result<Self, VariantConversionError> {
        if variant.get_type() != Self::variant_type() {
            return Err(VariantConversionError::BadType);
        }

        let array = unsafe {
            sys::from_sys_init_or_init_default::<Self>(|self_ptr| {
                let array_from_variant = sys::builtin_fn!(array_from_variant);
                array_from_variant(self_ptr, variant.var_sys());
            })
        };

        array.with_checked_type()
    }
}

/// Creates a `Array` from the given Rust array.
impl<T: VariantMetadata + ToVariant, const N: usize> From<&[T; N]> for Array<T> {
    fn from(arr: &[T; N]) -> Self {
        Self::from(&arr[..])
    }
}

/// Creates a `Array` from the given slice.
impl<T: VariantMetadata + ToVariant> From<&[T]> for Array<T> {
    fn from(slice: &[T]) -> Self {
        let mut array = Self::new();
        let len = slice.len();
        if len == 0 {
            return array;
        }
        array.resize(len);

        let ptr = array.ptr_mut_or_null(0);
        for (i, element) in slice.iter().enumerate() {
            // SAFETY: The array contains exactly `len` elements, stored contiguously in memory.
            // Also, the pointer is non-null, as we checked for emptiness above.
            unsafe {
                *ptr.offset(to_isize(i)) = element.to_variant();
            }
        }
        array
    }
}

/// Creates a `Array` from an iterator.
impl<T: VariantMetadata + ToVariant> FromIterator<T> for Array<T> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let mut array = Self::new();
        array.extend(iter);
        array
    }
}

/// Extends a `Array` with the contents of an iterator.
impl<T: VariantMetadata + ToVariant> Extend<T> for Array<T> {
    fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
        // Unfortunately the GDExtension API does not offer the equivalent of `Vec::reserve`.
        // Otherwise we could use it to pre-allocate based on `iter.size_hint()`.
        //
        // A faster implementation using `resize()` and direct pointer writes might still be
        // possible.
        for item in iter.into_iter() {
            self.push(item);
        }
    }
}

/// Converts this array to a strongly typed Rust vector.
impl<T: VariantMetadata + FromVariant> From<&Array<T>> for Vec<T> {
    fn from(array: &Array<T>) -> Vec<T> {
        let len = array.len();
        let mut vec = Vec::with_capacity(len);
        let ptr = array.ptr(0);
        for offset in 0..to_isize(len) {
            // SAFETY: Arrays are stored contiguously in memory, so we can use pointer arithmetic
            // instead of going through `array_operator_index_const` for every index.
            let variant = unsafe { &*ptr.offset(offset) };
            let element = T::from_variant(variant);
            vec.push(element);
        }
        vec
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

pub struct Iter<'a, T: VariantMetadata> {
    array: &'a Array<T>,
    next_idx: usize,
}

impl<'a, T: VariantMetadata + FromVariant> Iterator for Iter<'a, T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.next_idx < self.array.len() {
            let idx = self.next_idx;
            self.next_idx += 1;

            let element_ptr = self.array.ptr_or_null(idx);

            // SAFETY: We just checked that the index is not out of bounds, so the pointer won't be null.
            let variant = unsafe { &*element_ptr };
            let element = T::from_variant(variant);
            Some(element)
        } else {
            None
        }
    }
}

// TODO There's a macro for this, but it doesn't support generics yet; add support and use it
impl<T: VariantMetadata> PartialEq for Array<T> {
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

impl<T: VariantMetadata> PartialOrd for Array<T> {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        let op_less = |lhs, rhs| unsafe {
            let mut result = false;
            sys::builtin_call! {
                array_operator_less(lhs, rhs, result.sys_mut())
            };
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
/// Example:
/// ```no_run
/// # use godot::prelude::*;
/// let arr = array![3, 1, 4];  // Array<i32>
/// ```
///
/// To create an `Array` of variants, see the [`varray!`] macro.
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
/// The type of the array is always [`Variant`].
///
/// Example:
/// ```no_run
/// # use godot::prelude::*;
/// let arr: VariantArray = varray![42_i64, "hello", true];
/// ```
///
/// To create a typed `Array` with a single element type, see the [`array!`] macro.
#[macro_export]
macro_rules! varray {
    // Note: use to_variant() and not Variant::from(), as that works with both references and values
    ($($elements:expr),* $(,)?) => {
        {
            use $crate::builtin::ToVariant as _;
            let mut array = $crate::builtin::VariantArray::default();
            $(
                array.push($elements.to_variant());
            )*
            array
        }
    };
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

/// Represents the type information of a Godot array. See
/// [`set_typed`](https://docs.godotengine.org/en/latest/classes/class_array.html#class-array-method-set-typed).
///
/// We ignore the `script` parameter because it has no impact on typing in Godot.
#[derive(PartialEq, Eq)]
struct TypeInfo {
    variant_type: VariantType,
    class_name: StringName,
}

impl TypeInfo {
    fn new<T: VariantMetadata>() -> Self {
        let variant_type = T::variant_type();
        let class_name: StringName = T::class_name().into();
        Self {
            variant_type,
            class_name,
        }
    }

    fn is_typed(&self) -> bool {
        self.variant_type != VariantType::Nil
    }
}

impl fmt::Debug for TypeInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let class = self.class_name.to_string();
        let class_str = if class.is_empty() {
            String::new()
        } else {
            format!(" (class={class})")
        };

        write!(f, "{:?}{}", self.variant_type, class_str)
    }
}
