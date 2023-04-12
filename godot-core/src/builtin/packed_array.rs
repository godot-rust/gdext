/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use godot_ffi as sys;

use crate::builtin::*;
use std::fmt;
use sys::types::*;
use sys::{ffi_methods, interface_fn, GodotFfi};

// FIXME remove dependency on these types
use sys::{__GdextString, __GdextType};

/// Defines and implements a single packed array type. This macro is not hygienic and is meant to
/// be used only in the current module.
macro_rules! impl_packed_array {
    (
        // Name of the type to define, e.g. `PackedByteArray`.
        type_name: $PackedArray:ident,
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
        /// Implements Godot's `$PackedArray` type, which is an efficient array of `$Element`s.
        ///
        /// Note that, unlike `Array`, this type has value semantics: each copy will be independent
        /// of the original. (Under the hood, Godot uses copy-on-write, so copies are still cheap
        /// to make.)
        ///
        /// # Thread safety
        ///
        /// Usage is safe if the `$PackedArray` is used on a single thread only. Concurrent reads on
        /// different threads are also safe, but any writes must be externally synchronized. The
        /// Rust compiler will enforce this as long as you use only Rust threads, but it cannot
        /// protect against concurrent modification on other threads (e.g. created through
        /// GDScript).
        #[repr(C)]
        pub struct $PackedArray {
            opaque: $Opaque,
        }

        impl $PackedArray {
            fn from_opaque(opaque: $Opaque) -> Self {
                Self { opaque }
            }
        }

        // This impl relies on `$Inner` which is not (yet) available in unit tests
        impl $PackedArray {
            /// Constructs an empty array.
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
                        // Note that we do need to use `.clone()` because `GodotString` is refcounted;
                        // we can't just do a memcpy.
                        let element = unsafe { (*ptr.offset(offset)).clone() };
                        vec.push(element);
                    }
                }
                vec
            }

            /// Clears the array, removing all elements.
            pub fn clear(&mut self) {
                self.as_inner().clear();
            }

            /// Resizes the array to contain a different number of elements. If the new size is
            /// smaller, elements are removed from the end. If the new size is larger, new elements
            /// are set to [`Default::default()`].
            pub fn resize(&mut self, size: usize) {
                self.as_inner().resize(to_i64(size));
            }

            /// Returns a slice of the array, from `begin` (inclusive) to `end` (exclusive), as a
            /// new array.
            ///
            /// The values of `begin` and `end` will be clamped to the array size.
            pub fn slice(&self, begin: usize, end: usize) -> Self {
                let len = self.len();
                let begin = begin.min(len);
                let end = end.min(len);
                self.as_inner().slice(to_i64(begin), to_i64(end))
            }

            /// Returns a copy of the value at the specified index.
            ///
            /// # Panics
            ///
            /// If `index` is out of bounds.
            pub fn get(&self, index: usize) -> $Element {
                let ptr = self.ptr(index);
                // SAFETY: `ptr` just verified that the index is not out of bounds.
                unsafe { (*ptr).clone() }
            }

            /// Finds the index of an existing value in a sorted array using binary search.
            /// Equivalent of `bsearch` in GDScript.
            ///
            /// If the value is not present in the array, returns the insertion index that would
            /// maintain sorting order.
            ///
            /// Calling `binary_search` on an unsorted array results in unspecified behavior.
            pub fn binary_search(&self, value: $Element) -> usize {
                to_usize(self.as_inner().bsearch(Self::into_arg(value), true))
            }

            /// Returns the number of times a value is in the array.
            pub fn count(&self, value: $Element) -> usize {
                to_usize(self.as_inner().count(Self::into_arg(value)))
            }

            /// Returns `true` if the array contains the given value. Equivalent of `has` in
            /// GDScript.
            pub fn contains(&self, value: $Element) -> bool {
                self.as_inner().has(Self::into_arg(value))
            }

            /// Searches the array for the first occurrence of a value and returns its index, or
            /// `None` if not found. Starts searching at index `from`; pass `None` to search the
            /// entire array.
            pub fn find(&self, value: $Element, from: Option<usize>) -> Option<usize> {
                let from = to_i64(from.unwrap_or(0));
                let index = self.as_inner().find(Self::into_arg(value), from);
                if index >= 0 {
                    Some(index.try_into().unwrap())
                } else {
                    None
                }
            }

            /// Searches the array backwards for the last occurrence of a value and returns its
            /// index, or `None` if not found. Starts searching at index `from`; pass `None` to
            /// search the entire array.
            pub fn rfind(&self, value: $Element, from: Option<usize>) -> Option<usize> {
                let from = from.map(to_i64).unwrap_or(-1);
                let index = self.as_inner().rfind(Self::into_arg(value), from);
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
            pub fn set(&mut self, index: usize, value: $Element) {
                let ptr_mut = self.ptr_mut(index);
                // SAFETY: `ptr_mut` just checked that the index is not out of bounds.
                unsafe {
                    *ptr_mut = value;
                }
            }

            /// Appends an element to the end of the array. Equivalent of `append` and `push_back`
            /// in GDScript.
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
                let len = self.len();
                assert!(
                    index <= len,
                    "Array insertion index {index} is out of bounds: length is {len}");
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
            pub fn remove(&mut self, index: usize) -> $Element {
                self.check_bounds(index);
                let element = self.get(index);
                self.as_inner().remove_at(to_i64(index));
                element
            }

            /// Assigns the given value to all elements in the array. This can be used together
            /// with `resize` to create an array with a given size and initialized elements.
            pub fn fill(&mut self, value: $Element) {
                self.as_inner().fill(Self::into_arg(value));
            }

            /// Appends another array at the end of this array. Equivalent of `append_array` in
            /// GDScript.
            pub fn extend_array(&mut self, other: &$PackedArray) {
                self.as_inner().append_array(other.clone());
            }

            /// Reverses the order of the elements in the array.
            pub fn reverse(&mut self) {
                self.as_inner().reverse();
            }

            /// Sorts the elements of the array in ascending order.
            // Presumably, just like `Array`, this is not a stable sort so we might call it
            // `sort_unstable`. But Packed*Array elements that compare equal are always identical,
            // so it doesn't matter.
            pub fn sort(&mut self) {
                self.as_inner().sort();
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
                    "Array index {index} is out of bounds: length is {len}");
            }

            /// Returns a pointer to the element at the given index.
            ///
            /// # Panics
            ///
            /// If `index` is out of bounds.
            fn ptr(&self, index: usize) -> *const $Element {
                self.check_bounds(index);
                // SAFETY: We just checked that the index is not out of bounds.
                let ptr = unsafe {
                    let item_ptr: *const $IndexRetType =
                        (interface_fn!($operator_index_const))(self.sys(), to_i64(index));
                    item_ptr as *const $Element
                };
                assert!(!ptr.is_null());
                ptr
            }

            /// Returns a mutable pointer to the element at the given index.
            ///
            /// # Panics
            ///
            /// If `index` is out of bounds.
            fn ptr_mut(&self, index: usize) -> *mut $Element {
                self.check_bounds(index);
                // SAFETY: We just checked that the index is not out of bounds.
                let ptr = unsafe {
                    let item_ptr: *mut $IndexRetType =
                        (interface_fn!($operator_index))(self.sys(), to_i64(index));
                    item_ptr as *mut $Element
                };
                assert!(!ptr.is_null());
                ptr
            }

            /// Converts an `$Element` into a value that can be passed into API functions. For most
            /// types, this is a no-op. But `u8` and `i32` are widened to `i64`, and `real` is
            /// widened to `f64` if it is an `f32`.
            #[inline]
            fn into_arg(e: $Element) -> $Arg {
                e.into()
            }

            #[doc(hidden)]
            pub fn as_inner(&self) -> inner::$Inner<'_> {
                inner::$Inner::from_outer(self)
            }
        }

        impl_builtin_traits! {
            for $PackedArray {
                $($trait_impls)*
            }
        }

        /// Creates a `$PackedArray` from the given Rust array.
        impl<const N: usize> From<&[$Element; N]> for $PackedArray {
            fn from(arr: &[$Element; N]) -> Self {
                Self::from(&arr[..])
            }
        }

        /// Creates a `$PackedArray` from the given slice.
        impl From<&[$Element]> for $PackedArray {
            fn from(slice: &[$Element]) -> Self {
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
                        // `GodotString` does not implement `Copy` so we have to call `.clone()`
                        // here.
                        *ptr.offset(to_isize(i)) = element.clone();
                    }
                }
                array
            }
        }

        /// Creates a `$PackedArray` from an iterator.
        impl FromIterator<$Element> for $PackedArray {
            fn from_iter<I: IntoIterator<Item = $Element>>(iter: I) -> Self {
                let mut array = $PackedArray::default();
                array.extend(iter);
                array
            }
        }

        /// Extends a `$PackedArray` with the contents of an iterator.
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

        unsafe impl GodotFfi for $PackedArray {
            ffi_methods! { type sys::GDExtensionTypePtr = *mut Opaque;
                fn from_sys;
                fn sys;
                fn from_sys_init;
                // SAFETY:
                // Nothing special needs to be done beyond a `std::mem::swap` when returning a packed array.
                fn move_return_ptr;
            }

            // SAFETY:
            // Packed arrays are properly initialized through a `from_sys` call, but the ref-count should be
            // incremented as that is the callee's responsibility.
            //
            // Using `std::mem::forget(array.clone())` increments the ref count.
            unsafe fn from_arg_ptr(ptr: sys::GDExtensionTypePtr, _call_type: sys::PtrcallType) -> Self {
                let array = Self::from_sys(ptr);
                std::mem::forget(array.clone());
                array
            }

            unsafe fn from_sys_init_default(init_fn: impl FnOnce(sys::GDExtensionTypePtr)) -> Self {
                let mut result = Self::default();
                init_fn(result.sys_mut());
                result
            }

        }
    }
}

impl_packed_array!(
    type_name: PackedByteArray,
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
    element_type: GodotString,
    opaque_type: OpaquePackedStringArray,
    inner_type: InnerPackedStringArray,
    argument_type: GodotString,
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

impl_packed_array!(
    type_name: PackedColorArray,
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
