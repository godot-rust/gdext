/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use godot_ffi as sys;

use crate::builtin::*;
use crate::meta::ToGodot;
use std::{fmt, ops, ptr};
use sys::types::*;
use sys::{ffi_methods, interface_fn, GodotFfi};

// FIXME remove dependency on these types
use sys::{__GdextString, __GdextType};

// TODO(bromeon): ensure and test that all element types can be packed.
// Many builtin types don't have a #[repr] themselves, but they are used in packed arrays, which assumes certain size and alignment.
// This is mostly a problem for as_slice(), which reinterprets the FFI representation into the "frontend" type like GString.

/// Defines and implements a single packed array type. This macro is not hygienic and is meant to
/// be used only in the current module.
macro_rules! impl_packed_array {
    (
        // Name of the type to define, e.g. `PackedByteArray`.
        type_name: $PackedArray:ident,
        // Name of the VariantType constant, e.g. `PACKED_BYTE_ARRAY`.
        variant_type: $VariantType:ident,
        // Type of elements contained in the array, e.g. `u8`.
        element_type: $Element:ty,
        // Name of wrapped opaque type, e.g. `OpaquePackedByteArray`.
        opaque_type: $Opaque:ty,
        // Name of inner type, e.g. `InnerPackedByteArray`.
        inner_type: $Inner:ident,
        // Name of type that represents elements in function call arguments, e.g. `i64`. See
        // `Self::into_arg()`.
        argument_type: $Arg:ty,
        // Type that is returned from `$operator_index` and `$operator_index_const`.
        return_type: $IndexRetType:ty,
        // Name of constructor function from `Array` from FFI, e.g. `packed_byte_array_from_array`.
        from_array: $from_array:ident,
        // Name of index operator from FFI, e.g. `packed_byte_array_operator_index`.
        operator_index: $operator_index:ident,
        // Name of const index operator from FFI, e.g. `packed_byte_array_operator_index_const`.
        operator_index_const: $operator_index_const:ident,
        // Invocation passed to `impl_builtin_traits!` macro.
        trait_impls: {
            $($trait_impls:tt)*
        },
    ) => {
        // TODO expand type names in doc comments (use e.g. `paste` crate)
        #[doc = concat!("Implements Godot's `", stringify!($PackedArray), "` type,")]
        #[doc = concat!("which is a space-efficient array of `", stringify!($Element), "`s.")]
        ///
        /// Check out the [book](https://godot-rust.github.io/book/godot-api/builtins.html#packed-arrays) for a tutorial on packed arrays.
        ///
        /// Note that, unlike `Array`, this type has value semantics: each copy will be independent
        /// of the original. Under the hood, Godot uses copy-on-write, so copies are still cheap
        /// to make.
        ///
        /// # Registering properties
        ///
        /// You can use both `#[var]` and `#[export]` with packed arrays. However, since they use copy-on-write, GDScript (for `#[var]`) and the
        /// editor (for `#[export]`) will effectively keep an independent copy of the array. Writes to the packed array from Rust are thus not
        /// reflected on the other side -- you may need to replace the entire array.
        ///
        /// See also [godot/#76150](https://github.com/godotengine/godot/issues/76150) for details.
        ///
        /// # Thread safety
        ///
        #[doc = concat!("Usage is safe if the `", stringify!($PackedArray), "`")]
        /// is used on a single thread only. Concurrent reads on different threads are also safe,
        /// but any writes must be externally synchronized. The Rust compiler will enforce this as
        /// long as you use only Rust threads, but it cannot protect against concurrent modification
        /// on other threads (e.g. created through GDScript).
        pub struct $PackedArray {
            opaque: $Opaque,
        }

        impl $PackedArray {
            fn from_opaque(opaque: $Opaque) -> Self {
                Self { opaque }
            }

            /// Constructs an empty array.
            pub fn new() -> Self {
                Self::default()
            }

            /// Returns a copy of the value at the specified index, or `None` if out-of-bounds.
            ///
            /// If you know the index is valid, use the `[]` operator (`Index`/`IndexMut` traits) instead.
            pub fn get(&self, index: usize) -> Option<$Element> {
                let ptr = self.ptr_or_none(index)?;

                // SAFETY: if index was out of bounds, `ptr` would be `None` and return early.
                unsafe { Some((*ptr).clone()) }
            }

            /// Returns `true` if the array contains the given value.
            ///
            /// _Godot equivalent: `has`_
            #[doc(alias = "has")]
            pub fn contains(&self, value: &$Element) -> bool {
                self.as_inner().has(Self::to_arg(value))
            }

            /// Returns the number of times a value is in the array.
            pub fn count(&self, value: &$Element) -> usize {
                to_usize(self.as_inner().count(Self::to_arg(value)))
            }

            /// Returns the number of elements in the array. Equivalent of `size()` in Godot.
            pub fn len(&self) -> usize {
                to_usize(self.as_inner().size())
            }

            /// Returns `true` if the array is empty.
            pub fn is_empty(&self) -> bool {
                self.as_inner().is_empty()
            }

            /// Clears the array, removing all elements.
            pub fn clear(&mut self) {
                self.as_inner().clear();
            }

            /// Appends an element to the end of the array. Equivalent of `append` and `push_back`
            /// in GDScript.
            #[doc(alias = "append")]
            #[doc(alias = "push_back")]
            pub fn push(&mut self, value: $Element) {
                self.as_inner().push_back(Self::into_arg(value));
            }

            /// Inserts a new element at a given index in the array. The index must be valid, or at
            /// the end of the array (`index == len()`).
            ///
            /// Note: On large arrays, this method is much slower than `push` as it will move all
            /// the array's elements after the inserted element. The larger the array, the slower
            /// `insert` will be.
            pub fn insert(&mut self, index: usize, value: $Element) {
                // Intentional > and not >=.
                if index > self.len() {
                    self.panic_out_of_bounds(index);
                }

                self.as_inner().insert(to_i64(index), Self::into_arg(value));
            }

            /// Removes and returns the element at the specified index. Similar to `remove_at` in
            /// GDScript, but also returns the removed value.
            ///
            /// On large arrays, this method is much slower than `pop_back` as it will move all the array's
            /// elements after the removed element. The larger the array, the slower `remove` will be.
            ///
            /// # Panics
            ///
            /// If `index` is out of bounds.
            // Design note: This returns the removed value instead of `()` for consistency with
            // `Array` and with `Vec::remove`. Compared to shifting all the subsequent array
            // elements to their new position, the overhead of retrieving this element is trivial.
            #[doc(alias = "remove_at")]
            pub fn remove(&mut self, index: usize) -> $Element {
                let element = self[index].clone(); // panics on out-of-bounds
                self.as_inner().remove_at(to_i64(index));
                element
            }

            /// Assigns the given value to all elements in the array. This can be used together
            /// with `resize` to create an array with a given size and initialized elements.
            pub fn fill(&mut self, value: $Element) {
                self.as_inner().fill(Self::into_arg(value));
            }

            /// Resizes the array to contain a different number of elements. If the new size is
            /// smaller, elements are removed from the end. If the new size is larger, new elements
            /// are set to [`Default::default()`].
            pub fn resize(&mut self, size: usize) {
                self.as_inner().resize(to_i64(size));
            }

            /// Appends another array at the end of this array. Equivalent of `append_array` in GDScript.
            pub fn extend_array(&mut self, other: &$PackedArray) {
                self.as_inner().append_array(other.clone());
            }

            /// Converts this array to a Rust vector, making a copy of its contents.
            pub fn to_vec(&self) -> Vec<$Element> {
                let len = self.len();
                let mut vec = Vec::with_capacity(len);
                if len > 0 {
                    let ptr = self.ptr(0);
                    for offset in 0..to_isize(len) {
                        // SAFETY: Packed arrays are stored contiguously in memory, so we can use
                        // pointer arithmetic instead of going through `$operator_index_const` for
                        // every index.
                        // Note that we do need to use `.clone()` because `GString` is refcounted;
                        // we can't just do a memcpy.
                        let element = unsafe { (*ptr.offset(offset)).clone() };
                        vec.push(element);
                    }
                }
                vec
            }

            /// Returns a sub-range `begin..end`, as a new packed array.
            ///
            /// This method is called `slice()` in Godot.
            /// The values of `begin` (inclusive) and `end` (exclusive) will be clamped to the array size.
            ///
            /// To obtain Rust slices, see [`as_slice`][Self::as_slice] and [`as_mut_slice`][Self::as_mut_slice].
            #[doc(alias = "slice")]
            pub fn subarray(&self, begin: usize, end: usize) -> Self {
                let len = self.len();
                let begin = begin.min(len);
                let end = end.min(len);
                self.as_inner().slice(to_i64(begin), to_i64(end))
            }

            /// Returns a shared Rust slice of the array.
            ///
            /// The resulting slice can be further subdivided or converted into raw pointers.
            ///
            /// See also [`as_mut_slice`][Self::as_mut_slice] to get exclusive slices, and
            /// [`subarray`][Self::subarray] to get a sub-array as a copy.
            pub fn as_slice(&self) -> &[$Element] {
                if self.is_empty() {
                    &[]
                } else {
                    let data = self.ptr(0);

                    // SAFETY: PackedArray holds `len` elements in contiguous storage, all of which are initialized.
                    // The array uses copy-on-write semantics, so the slice may be aliased, but copies will use a new allocation.
                    unsafe {
                        std::slice::from_raw_parts(data, self.len())
                    }
                }
            }

            /// Returns an exclusive Rust slice of the array.
            ///
            /// The resulting slice can be further subdivided or converted into raw pointers.
            ///
            /// See also [`as_slice`][Self::as_slice] to get shared slices, and
            /// [`subarray`][Self::subarray] to get a sub-array as a copy.
            pub fn as_mut_slice(&mut self) -> &mut [$Element] {
                if self.is_empty() {
                    &mut []
                } else {
                    let data = self.ptr_mut(0);

                    // SAFETY: PackedArray holds `len` elements in contiguous storage, all of which are initialized.
                    // The array uses copy-on-write semantics. ptr_mut() triggers a copy if non-unique, after which the slice is never aliased.
                    unsafe {
                        std::slice::from_raw_parts_mut(data, self.len())
                    }
                }
            }

            /// Searches the array for the first occurrence of a value and returns its index, or
            /// `None` if not found. Starts searching at index `from`; pass `None` to search the
            /// entire array.
            pub fn find(&self, value: &$Element, from: Option<usize>) -> Option<usize> {
                let from = to_i64(from.unwrap_or(0));
                let index = self.as_inner().find(Self::to_arg(value), from);
                if index >= 0 {
                    Some(index.try_into().unwrap())
                } else {
                    None
                }
            }

            /// Searches the array backwards for the last occurrence of a value and returns its
            /// index, or `None` if not found. Starts searching at index `from`; pass `None` to
            /// search the entire array.
            pub fn rfind(&self, value: &$Element, from: Option<usize>) -> Option<usize> {
                let from = from.map(to_i64).unwrap_or(-1);
                let index = self.as_inner().rfind(Self::to_arg(value), from);
                // It's not documented, but `rfind` returns -1 if not found.
                if index >= 0 {
                    Some(to_usize(index))
                } else {
                    None
                }
            }

            /// Finds the index of an existing value in a _sorted_ array using binary search.
            ///
            /// If the value is not present in the array, returns the insertion index that would maintain sorting order.
            ///
            /// Calling `bsearch()` on an unsorted array results in unspecified (but safe) behavior.
            pub fn bsearch(&self, value: &$Element) -> usize {
                to_usize(self.as_inner().bsearch(Self::to_arg(value), true))
            }

            /// Reverses the order of the elements in the array.
            pub fn reverse(&mut self) {
                self.as_inner().reverse();
            }

            /// Sorts the elements of the array in ascending order.
            ///
            /// This sort is [stable](https://en.wikipedia.org/wiki/Sorting_algorithm#Stability), since elements inside packed arrays are
            /// indistinguishable. Relative order between equal elements thus isn't observable.
            pub fn sort(&mut self) {
                self.as_inner().sort();
            }

            // Include specific functions in the code only if the Packed*Array provides the function.
            impl_specific_packed_array_functions!($PackedArray);

            /// # Panics
            ///
            /// Always.
            fn panic_out_of_bounds(&self, index: usize) -> ! {
                panic!("Array index {index} is out of bounds: length is {}", self.len());
            }

            /// Returns a pointer to the element at the given index.
            ///
            /// # Panics
            ///
            /// If `index` is out of bounds.
            fn ptr(&self, index: usize) -> *const $Element {
                self.ptr_or_none(index).unwrap_or_else(|| self.panic_out_of_bounds(index))
            }

            /// Returns a pointer to the element at the given index, or `None` if out of bounds.
            fn ptr_or_none(&self, index: usize) -> Option<*const $Element> {
                // SAFETY: The packed array index operators return a null pointer on out-of-bounds.
                let item_ptr: *const $IndexRetType = unsafe {
                    interface_fn!($operator_index_const)(self.sys(), to_i64(index))
                };

                if item_ptr.is_null() {
                    None
                } else {
                    Some(item_ptr as *const $Element)
                }
            }

            /// Returns a mutable pointer to the element at the given index.
            ///
            /// # Panics
            ///
            /// If `index` is out of bounds.
            fn ptr_mut(&mut self, index: usize) -> *mut $Element {
                // SAFETY: The packed array index operators return a null pointer on out-of-bounds.
                let item_ptr: *mut $IndexRetType = unsafe {
                    interface_fn!($operator_index)(self.sys_mut(), to_i64(index))
                };

                if item_ptr.is_null() {
                    self.panic_out_of_bounds(index)
                } else {
                    item_ptr as *mut $Element
                }
            }

            #[doc = concat!("Converts a `", stringify!($Element), "` into a value that can be")]
            /// passed into API functions. For most types, this is a no-op. But `u8` and `i32` are
            /// widened to `i64`, and `real` is widened to `f64` if it is an `f32`.
            #[inline]
            fn into_arg(e: $Element) -> $Arg {
                e.into()
            }

            #[inline]
            fn to_arg(e: &$Element) -> $Arg {
                // Once PackedArra<T> is generic, this could use a better tailored implementation that may not need to clone.
                e.clone().into()
            }

            #[doc(hidden)]
            pub fn as_inner(&self) -> inner::$Inner<'_> {
                inner::$Inner::from_outer(self)
            }

            /// Create array filled with default elements.
            fn default_with_size(n: usize) -> Self {
                let mut array = Self::new();
                array.resize(n);
                array
            }

            /// Drops all elements in `self` and replaces them with data from an array of values.
            ///
            /// # Safety
            ///
            /// * Pointer must be valid slice of data with `len` size.
            /// * Pointer must not point to `self` data.
            /// * Length must be equal to `self.len()`.
            /// * Source data must not be dropped later.
            unsafe fn move_from_slice(&mut self, src: *const $Element, len: usize) {
                let ptr = self.ptr_mut(0);
                debug_assert_eq!(len, self.len(), "length precondition violated");
                // Drops all elements in place. Drop impl must not panic.
                ptr::drop_in_place(ptr::slice_from_raw_parts_mut(ptr, len));
                // Copy is okay since all elements are dropped.
                ptr.copy_from_nonoverlapping(src, len);
            }
        }

        impl_builtin_traits! {
            for $PackedArray {
                $($trait_impls)*
            }
        }

        impl ops::Index<usize> for $PackedArray {
            type Output = $Element;

            fn index(&self, index: usize) -> &Self::Output {
                let ptr = self.ptr(index);
                // SAFETY: `ptr` checked bounds.
                unsafe { &*ptr }
            }
        }

        impl ops::IndexMut<usize> for $PackedArray {
            fn index_mut(&mut self, index: usize) -> &mut Self::Output {
                let ptr = self.ptr_mut(index);
                // SAFETY: `ptr` checked bounds.
                unsafe { &mut *ptr }
            }
        }

        #[doc = concat!("Creates a `", stringify!($PackedArray), "` from the given Rust array.")]
        impl<const N: usize> From<&[$Element; N]> for $PackedArray {
            fn from(arr: &[$Element; N]) -> Self {
                Self::from(&arr[..])
            }
        }

        #[doc = concat!("Creates a `", stringify!($PackedArray), "` from the given slice.")]
        impl From<&[$Element]> for $PackedArray {
            fn from(slice: &[$Element]) -> Self {
                if slice.is_empty() {
                    return Self::new();
                }
                let mut array = Self::default_with_size(slice.len());

                // SAFETY: The array contains exactly `len` elements, stored contiguously in memory.
                let dst = unsafe { std::slice::from_raw_parts_mut(array.ptr_mut(0), slice.len()) };
                dst.clone_from_slice(slice);
                array
            }
        }

        #[doc = concat!("Creates a `", stringify!($PackedArray), "` from the given Rust array.")]
        impl<const N: usize> From<[$Element; N]> for $PackedArray {
            fn from(arr: [$Element; N]) -> Self {
                if N == 0 {
                    return Self::new();
                }
                let mut packed_array = Self::default_with_size(N);

                // Not using forget() so if move_from_slice somehow panics then there is no double-free.
                let arr = std::mem::ManuallyDrop::new(arr);

                // SAFETY: The packed array contains exactly N elements and the source array will be forgotten.
                unsafe {
                    packed_array.move_from_slice(arr.as_ptr(), N);
                }
                packed_array
            }
        }

        #[doc = concat!("Creates a `", stringify!($PackedArray), "` from the given Rust vec.")]
        impl From<Vec<$Element>> for $PackedArray {
            fn from(mut vec: Vec<$Element>) -> Self {
                if vec.is_empty() {
                    return Self::new();
                }
                let len = vec.len();
                let mut array = Self::default_with_size(len);

                // SAFETY: The packed array and vector contain exactly `len` elements.
                // The vector is forcibly set to empty, so its contents are forgotten.
                unsafe {
                    vec.set_len(0);
                    array.move_from_slice(vec.as_ptr(), len);
                }
                array
            }
        }

        #[doc = concat!("Creates a `", stringify!($PackedArray), "` from an iterator.")]
        impl FromIterator<$Element> for $PackedArray {
            fn from_iter<I: IntoIterator<Item = $Element>>(iter: I) -> Self {
                let mut array = $PackedArray::default();
                array.extend(iter);
                array
            }
        }

        #[doc = concat!("Extends a`", stringify!($PackedArray), "` with the contents of an iterator")]
        impl Extend<$Element> for $PackedArray {
            fn extend<I: IntoIterator<Item = $Element>>(&mut self, iter: I) {
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

        impl_builtin_froms!($PackedArray; VariantArray => $from_array);

        impl fmt::Debug for $PackedArray {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                // Going through `Variant` because there doesn't seem to be a direct way.
                write!(f, "{:?}", self.to_variant().stringify())
            }
        }

        impl fmt::Display for $PackedArray {
            /// Formats `PackedArray` to match Godot's string representation.
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                write!(f, "[")?;
                for (i, elem) in self.as_slice().iter().enumerate() {
                    if i != 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{elem}")?;
                }
                write!(f, "]")
            }
        }

        unsafe impl GodotFfi for $PackedArray {
            fn variant_type() -> sys::VariantType {
                sys::VariantType::$VariantType
            }

            ffi_methods! { type sys::GDExtensionTypePtr = *mut Opaque; .. }
        }

        $crate::meta::impl_godot_as_self!($PackedArray);

        impl $crate::registry::property::Export for $PackedArray {
            fn default_export_info() -> $crate::registry::property::PropertyHintInfo {
                // In 4.3 Godot can (and does) use type hint strings for packed arrays, see https://github.com/godotengine/godot/pull/82952.
                if sys::GdextBuild::since_api("4.3") {
                    $crate::registry::property::PropertyHintInfo {
                        hint: $crate::global::PropertyHint::TYPE_STRING,
                        hint_string: <$Element as $crate::registry::property::TypeStringHint>::type_string().into(),
                    }
                } else {
                    $crate::registry::property::PropertyHintInfo::with_hint_none(
                        <$PackedArray as $crate::meta::GodotType>::godot_type_name()
                    )
                }
            }
        }
    }
}

// Helper macro to only include specific functions in the code if the Packed*Array provides the function.
macro_rules! impl_specific_packed_array_functions {
    (PackedByteArray) => {
        /// Returns a copy of the data converted to a `PackedFloat32Array`, where each block of 4 bytes has been converted to a 32-bit float.
        ///
        /// The size of the input array must be a multiple of 4 (size of 32-bit float). The size of the new array will be `byte_array.size() / 4`.
        ///
        /// If the original data can't be converted to 32-bit floats, the resulting data is undefined.
        pub fn to_float32_array(&self) -> PackedFloat32Array {
            self.as_inner().to_float32_array()
        }

        /// Returns a copy of the data converted to a `PackedFloat64Array`, where each block of 8 bytes has been converted to a 64-bit float.
        ///
        /// The size of the input array must be a multiple of 8 (size of 64-bit float). The size of the new array will be `byte_array.size() / 8`.
        ///
        /// If the original data can't be converted to 64-bit floats, the resulting data is undefined.
        pub fn to_float64_array(&self) -> PackedFloat64Array {
            self.as_inner().to_float64_array()
        }

        /// Returns a copy of the data converted to a `PackedInt32Array`, where each block of 4 bytes has been converted to a 32-bit integer.
        ///
        /// The size of the input array must be a multiple of 4 (size of 32-bit integer). The size of the new array will be `byte_array.size() / 4`.
        ///
        /// If the original data can't be converted to 32-bit integers, the resulting data is undefined.
        pub fn to_int32_array(&self) -> PackedInt32Array {
            self.as_inner().to_int32_array()
        }

        /// Returns a copy of the data converted to a `PackedInt64Array`, where each block of 8 bytes has been converted to a 64-bit integer.
        ///
        /// The size of the input array must be a multiple of 8 (size of 64-bit integer). The size of the new array will be `byte_array.size() / 8`.
        ///
        /// If the original data can't be converted to 64-bit integers, the resulting data is undefined.
        pub fn to_int64_array(&self) -> PackedInt64Array {
            self.as_inner().to_int64_array()
        }
    };
    ($PackedArray:ident) => {
        /// Returns a `PackedByteArray` with each value encoded as bytes.
        pub fn to_byte_array(&self) -> PackedByteArray {
            self.as_inner().to_byte_array()
        }
    };
}

impl_packed_array!(
    type_name: PackedByteArray,
    variant_type: PACKED_BYTE_ARRAY,
    element_type: u8,
    opaque_type: OpaquePackedByteArray,
    inner_type: InnerPackedByteArray,
    argument_type: i64,
    return_type: u8,
    from_array: packed_byte_array_from_array,
    operator_index: packed_byte_array_operator_index,
    operator_index_const: packed_byte_array_operator_index_const,
    trait_impls: {
        Default => packed_byte_array_construct_default;
        Clone => packed_byte_array_construct_copy;
        Drop => packed_byte_array_destroy;
        Eq => packed_byte_array_operator_equal;
    },
);

impl_packed_array!(
    type_name: PackedInt32Array,
    variant_type: PACKED_INT32_ARRAY,
    element_type: i32,
    opaque_type: OpaquePackedInt32Array,
    inner_type: InnerPackedInt32Array,
    argument_type: i64,
    return_type: i32,
    from_array: packed_int32_array_from_array,
    operator_index: packed_int32_array_operator_index,
    operator_index_const: packed_int32_array_operator_index_const,
    trait_impls: {
        Default => packed_int32_array_construct_default;
        Clone => packed_int32_array_construct_copy;
        Drop => packed_int32_array_destroy;
        Eq => packed_int32_array_operator_equal;
    },
);

impl_packed_array!(
    type_name: PackedInt64Array,
    variant_type: PACKED_INT64_ARRAY,
    element_type: i64,
    opaque_type: OpaquePackedInt64Array,
    inner_type: InnerPackedInt64Array,
    argument_type: i64,
    return_type: i64,
    from_array: packed_int64_array_from_array,
    operator_index: packed_int64_array_operator_index,
    operator_index_const: packed_int64_array_operator_index_const,
    trait_impls: {
        Default => packed_int64_array_construct_default;
        Clone => packed_int64_array_construct_copy;
        Drop => packed_int64_array_destroy;
        Eq => packed_int64_array_operator_equal;
    },
);

impl_packed_array!(
    type_name: PackedFloat32Array,
    variant_type: PACKED_FLOAT32_ARRAY,
    element_type: f32,
    opaque_type: OpaquePackedFloat32Array,
    inner_type: InnerPackedFloat32Array,
    argument_type: f64,
    return_type: f32,
    from_array: packed_float32_array_from_array,
    operator_index: packed_float32_array_operator_index,
    operator_index_const: packed_float32_array_operator_index_const,
    trait_impls: {
        Default => packed_float32_array_construct_default;
        Clone => packed_float32_array_construct_copy;
        Drop => packed_float32_array_destroy;
        PartialEq => packed_float32_array_operator_equal;
    },
);

impl_packed_array!(
    type_name: PackedFloat64Array,
    variant_type: PACKED_FLOAT64_ARRAY,
    element_type: f64,
    opaque_type: OpaquePackedFloat64Array,
    inner_type: InnerPackedFloat64Array,
    argument_type: f64,
    return_type: f64,
    from_array: packed_float64_array_from_array,
    operator_index: packed_float64_array_operator_index,
    operator_index_const: packed_float64_array_operator_index_const,
    trait_impls: {
        Default => packed_float64_array_construct_default;
        Clone => packed_float64_array_construct_copy;
        Drop => packed_float64_array_destroy;
        PartialEq => packed_float64_array_operator_equal;
    },
);

impl_packed_array!(
    type_name: PackedStringArray,
    variant_type: PACKED_STRING_ARRAY,
    element_type: GString,
    opaque_type: OpaquePackedStringArray,
    inner_type: InnerPackedStringArray,
    argument_type: GString,
    return_type: __GdextString,
    from_array: packed_string_array_from_array,
    operator_index: packed_string_array_operator_index,
    operator_index_const: packed_string_array_operator_index_const,
    trait_impls: {
        Default => packed_string_array_construct_default;
        Clone => packed_string_array_construct_copy;
        Drop => packed_string_array_destroy;
        Eq => packed_string_array_operator_equal;
    },
);

impl_packed_array!(
    type_name: PackedVector2Array,
    variant_type: PACKED_VECTOR2_ARRAY,
    element_type: Vector2,
    opaque_type: OpaquePackedVector2Array,
    inner_type: InnerPackedVector2Array,
    argument_type: Vector2,
    return_type: __GdextType,
    from_array: packed_vector2_array_from_array,
    operator_index: packed_vector2_array_operator_index,
    operator_index_const: packed_vector2_array_operator_index_const,
    trait_impls: {
        Default => packed_vector2_array_construct_default;
        Clone => packed_vector2_array_construct_copy;
        Drop => packed_vector2_array_destroy;
        PartialEq => packed_vector2_array_operator_equal;
    },
);

impl_packed_array!(
    type_name: PackedVector3Array,
    variant_type: PACKED_VECTOR3_ARRAY,
    element_type: Vector3,
    opaque_type: OpaquePackedVector3Array,
    inner_type: InnerPackedVector3Array,
    argument_type: Vector3,
    return_type: __GdextType,
    from_array: packed_vector3_array_from_array,
    operator_index: packed_vector3_array_operator_index,
    operator_index_const: packed_vector3_array_operator_index_const,
    trait_impls: {
        Default => packed_vector3_array_construct_default;
        Clone => packed_vector3_array_construct_copy;
        Drop => packed_vector3_array_destroy;
        PartialEq => packed_vector3_array_operator_equal;
    },
);

#[cfg(since_api = "4.3")]
impl_packed_array!(
    type_name: PackedVector4Array,
    variant_type: PACKED_VECTOR4_ARRAY,
    element_type: Vector4,
    opaque_type: OpaquePackedVector4Array,
    inner_type: InnerPackedVector4Array,
    argument_type: Vector4,
    return_type: __GdextType,
    from_array: packed_vector4_array_from_array,
    operator_index: packed_vector4_array_operator_index,
    operator_index_const: packed_vector4_array_operator_index_const,
    trait_impls: {
        Default => packed_vector4_array_construct_default;
        Clone => packed_vector4_array_construct_copy;
        Drop => packed_vector4_array_destroy;
        PartialEq => packed_vector4_array_operator_equal;
    },
);

impl_packed_array!(
    type_name: PackedColorArray,
    variant_type: PACKED_COLOR_ARRAY,
    element_type: Color,
    opaque_type: OpaquePackedColorArray,
    inner_type: InnerPackedColorArray,
    argument_type: Color,
    return_type: __GdextType,
    from_array: packed_color_array_from_array,
    operator_index: packed_color_array_operator_index,
    operator_index_const: packed_color_array_operator_index_const,
    trait_impls: {
        Default => packed_color_array_construct_default;
        Clone => packed_color_array_construct_copy;
        Drop => packed_color_array_destroy;
        PartialEq => packed_color_array_operator_equal;
    },
);
