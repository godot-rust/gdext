/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use sys::{__GdextString, __GdextType, interface_fn, types, GodotFfi};

use crate::builtin::PackedArray;
use crate::meta::{CowArg, GodotType, ToGodot};
use crate::registry::property::builtin_type_string;
use crate::{builtin, sys};

/// Marker trait to identify types that can be stored in `Packed*Array` types.
#[diagnostic::on_unimplemented(
    message = "`Packed*Array` can only store element types supported in Godot packed arrays.",
    label = "has invalid element type"
)]
pub trait PackedArrayElement: GodotType + Clone + ToGodot {
    /// The opaque FFI type for this packed array element.
    #[doc(hidden)]
    type OpaqueType: 'static;

    /// The inner API wrapper type for this packed array element.
    #[doc(hidden)]
    type Inner<'a>;

    /// The type used for function arguments when passing elements.
    #[doc(hidden)]
    type Arg<'a>;

    /// The type returned from FFI index operations.
    #[doc(hidden)]
    type ReturnType;

    /// The variant type constant for this packed array.
    #[doc(hidden)]
    const VARIANT_TYPE: sys::VariantType;

    /// The size in bytes of each element in the packed array.
    #[doc(hidden)]
    const ELEMENT_SIZE: usize;

    // ----------------------------------------------------------------------------------------------------------------------------------------------
    // Property-related API

    /// See [`crate::meta::traits::ArrayElement::element_type_string()`].
    #[doc(hidden)]
    fn element_type_string() -> String {
        builtin_type_string::<Self>()
    }

    // ----------------------------------------------------------------------------------------------------------------------------------------------
    // Individual Packed*Array delegates

    /// Variant conversion methods for PackedArray FFI.
    #[doc(hidden)]
    unsafe fn ffi_to_variant(
        opaque_ptr: sys::GDExtensionConstTypePtr,
        variant_ptr: sys::GDExtensionVariantPtr,
    );

    #[doc(hidden)]
    unsafe fn ffi_from_variant(
        variant_ptr: sys::GDExtensionConstVariantPtr,
        opaque_ptr: sys::GDExtensionTypePtr,
    );

    /// FFI constructor, destructor and comparison methods.
    #[doc(hidden)]
    unsafe fn ffi_default(ptr: sys::GDExtensionTypePtr);

    #[doc(hidden)]
    unsafe fn ffi_copy(dst_ptr: sys::GDExtensionTypePtr, src_ptr: sys::GDExtensionConstTypePtr);

    #[doc(hidden)]
    unsafe fn ffi_destroy(ptr: sys::GDExtensionTypePtr);

    #[doc(hidden)]
    unsafe fn ffi_equals(
        left_ptr: sys::GDExtensionConstTypePtr,
        right_ptr: sys::GDExtensionConstTypePtr,
    ) -> bool;

    #[doc(hidden)]
    fn index_const(opaque: sys::GDExtensionConstTypePtr, index: i64) -> *const Self::ReturnType;

    #[doc(hidden)]
    fn index_mut(opaque: sys::GDExtensionTypePtr, index: i64) -> *mut Self::ReturnType;

    /// Helper method to check if a packed array contains a value.
    #[doc(hidden)]
    fn op_has(inner: &Self::Inner<'_>, value: CowArg<'_, Self>) -> bool;

    /// Helper method to count occurrences of a value in packed array.
    #[doc(hidden)]
    fn op_count(inner: &Self::Inner<'_>, value: CowArg<'_, Self>) -> i64;

    /// Helper method to get the size of a packed array.
    #[doc(hidden)]
    fn op_size(inner: &Self::Inner<'_>) -> i64;

    /// Helper method to check if a packed array is empty.
    #[doc(hidden)]
    fn op_is_empty(inner: &Self::Inner<'_>) -> bool;

    /// Helper method to clear a packed array.
    #[doc(hidden)]
    fn op_clear(inner: &mut Self::Inner<'_>);

    /// Helper method to push an element to the end of a packed array.
    #[doc(hidden)]
    fn op_push_back(inner: &mut Self::Inner<'_>, value: CowArg<'_, Self>);

    /// Helper method to insert an element at a specific index in a packed array.
    #[doc(hidden)]
    fn op_insert(inner: &mut Self::Inner<'_>, index: i64, value: CowArg<'_, Self>);

    /// Helper method to remove an element at a specific index in a packed array.
    #[doc(hidden)]
    fn op_remove_at(inner: &mut Self::Inner<'_>, index: i64);

    /// Helper method to fill a packed array with a value.
    #[doc(hidden)]
    fn op_fill(inner: &mut Self::Inner<'_>, value: CowArg<'_, Self>);

    /// Helper method to resize a packed array.
    #[doc(hidden)]
    fn op_resize(inner: &mut Self::Inner<'_>, size: i64);

    /// Helper method to append another packed array.
    #[doc(hidden)]
    fn op_append_array(inner: &mut Self::Inner<'_>, other: &PackedArray<Self>);

    /// Helper method to get a slice of a packed array.
    #[doc(hidden)]
    fn op_slice(inner: &Self::Inner<'_>, begin: i64, end: i64) -> Self::OpaqueType;

    /// Helper method to find an element in a packed array.
    #[doc(hidden)]
    fn op_find(inner: &Self::Inner<'_>, value: CowArg<'_, Self>, from: i64) -> i64;

    /// Helper method to find an element from the end in a packed array.
    #[doc(hidden)]
    fn op_rfind(inner: &Self::Inner<'_>, value: CowArg<'_, Self>, from: i64) -> i64;

    /// Helper method for binary search in a packed array.
    #[doc(hidden)]
    fn op_bsearch(inner: &mut Self::Inner<'_>, value: CowArg<'_, Self>, before: bool) -> i64;

    /// Helper method to reverse a packed array.
    #[doc(hidden)]
    fn op_reverse(inner: &mut Self::Inner<'_>);

    /// Helper method to sort a packed array.
    #[doc(hidden)]
    fn op_sort(inner: &mut Self::Inner<'_>);

    #[doc(hidden)]
    fn inner<'a>(array: &PackedArray<Self>) -> Self::Inner<'a>;

    #[doc(hidden)]
    fn cow_to_arg(value: CowArg<'_, Self>) -> Self::Arg<'_>;
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// General impl definition

macro_rules! impl_packed_array_element {
    (
        element_type: $Element:ty,
        argument_type: $Arg:ty,
        argument_conversion: $ArgConv:ident,
        return_type: $IndexRetType:ty,
        variant_type: $VariantType:ident,
        inner_type: $Inner:ident,
        opaque_type: $Opaque:ty,
        default_fn: $default_fn:ident,
        copy_fn: $copy_fn:ident,
        destroy_fn: $destroy_fn:ident,
        equals_fn: $equals_fn:ident,
        to_variant_fn: $to_variant_fn:ident,
        from_variant_fn: $from_variant_fn:ident,
        index_mut_fn: $index_mut_fn:ident,
        index_const_fn: $index_const_fn:ident,
    ) => {
        impl PackedArrayElement for $Element {
            type OpaqueType = $Opaque;
            type Inner<'a> = crate::builtin::inner::$Inner<'a>;
            type Arg<'a> = $Arg;
            type ReturnType = $IndexRetType;
            const VARIANT_TYPE: sys::VariantType = sys::VariantType::$VariantType;
            const ELEMENT_SIZE: usize = std::mem::size_of::<$Element>();

            unsafe fn ffi_to_variant(opaque_ptr: sys::GDExtensionConstTypePtr, variant_ptr: sys::GDExtensionVariantPtr) {
                let converter = sys::builtin_fn!($to_variant_fn);
                converter(variant_ptr as sys::GDExtensionUninitializedVariantPtr, opaque_ptr as sys::GDExtensionTypePtr);
            }

            unsafe fn ffi_from_variant(variant_ptr: sys::GDExtensionConstVariantPtr, opaque_ptr: sys::GDExtensionTypePtr) {
                let converter = sys::builtin_fn!($from_variant_fn);
                converter(opaque_ptr as sys::GDExtensionUninitializedTypePtr, variant_ptr as sys::GDExtensionVariantPtr);
            }

            unsafe fn ffi_default(ptr: sys::GDExtensionTypePtr) {
                let ctor = sys::builtin_fn!($default_fn);
                ctor(ptr as sys::GDExtensionUninitializedTypePtr, std::ptr::null_mut());
            }

            unsafe fn ffi_copy(dst_ptr: sys::GDExtensionTypePtr, src_ptr: sys::GDExtensionConstTypePtr) {
                let ctor = sys::builtin_fn!($copy_fn);
                let args = [src_ptr];
                ctor(dst_ptr as sys::GDExtensionUninitializedTypePtr, args.as_ptr());
            }

            unsafe fn ffi_destroy(ptr: sys::GDExtensionTypePtr) {
                let destructor = sys::builtin_fn!($destroy_fn @1);
                destructor(ptr);
            }

            unsafe fn ffi_equals(left_ptr: sys::GDExtensionConstTypePtr, right_ptr: sys::GDExtensionConstTypePtr) -> bool {
                let mut result = false;
                sys::builtin_call! {
                    $equals_fn(left_ptr, right_ptr, result.sys_mut())
                };
                result
            }

            fn index_const(opaque: sys::GDExtensionConstTypePtr, index: i64) -> *const Self::ReturnType {
                let opaque_ptr = opaque as *const Self::OpaqueType;
                unsafe {
                    interface_fn!($index_const_fn)(opaque_ptr as sys::GDExtensionConstTypePtr, index)
                }
            }

            fn index_mut(opaque: sys::GDExtensionTypePtr, index: i64) -> *mut Self::ReturnType {
                let opaque_ptr = opaque as *mut Self::OpaqueType;
                unsafe {
                    let result_ptr = interface_fn!($index_mut_fn)(opaque_ptr as sys::GDExtensionTypePtr, index);
                    result_ptr as *mut Self::ReturnType
                }
            }

            fn op_has(inner: &Self::Inner<'_>, value: CowArg<'_, Self>) -> bool {
                inner.has(Self::cow_to_arg(value))
            }

            fn op_count(inner: &Self::Inner<'_>, value: CowArg<'_, Self>) -> i64 {
                inner.count(Self::cow_to_arg(value))
            }

            fn op_size(inner: &Self::Inner<'_>) -> i64 {
                inner.size()
            }

            fn op_is_empty(inner: &Self::Inner<'_>) -> bool {
                inner.is_empty()
            }

            fn op_clear(inner: &mut Self::Inner<'_>) {
                inner.clear();
            }

            fn op_push_back(inner: &mut Self::Inner<'_>, value: CowArg<'_, Self>) {
                inner.push_back(Self::cow_to_arg(value));
            }

            fn op_insert(inner: &mut Self::Inner<'_>, index: i64, value: CowArg<'_, Self>) {
                inner.insert(index, Self::cow_to_arg(value));
            }

            fn op_remove_at(inner: &mut Self::Inner<'_>, index: i64) {
                inner.remove_at(index);
            }

            fn op_fill(inner: &mut Self::Inner<'_>, value: CowArg<'_, Self>) {
                inner.fill(Self::cow_to_arg(value));
            }

            fn op_resize(inner: &mut Self::Inner<'_>, size: i64) {
                inner.resize(size);
            }

            fn op_append_array(inner: &mut Self::Inner<'_>, other: &PackedArray<Self>) {
                inner.append_array(other);
            }

            fn op_slice(inner: &Self::Inner<'_>, begin: i64, end: i64) -> Self::OpaqueType {
                // Return the opaque representation directly
                unsafe { std::mem::transmute(inner.slice(begin, end).into_ffi()) }
            }

            fn op_find(inner: &Self::Inner<'_>, value: CowArg<'_, Self>, from: i64) -> i64 {
                inner.find(Self::cow_to_arg(value), from)
            }

            fn op_rfind(inner: &Self::Inner<'_>, value: CowArg<'_, Self>, from: i64) -> i64 {
                inner.rfind(Self::cow_to_arg(value), from)
            }

            fn op_bsearch(inner: &mut Self::Inner<'_>, value: CowArg<'_, Self>, before: bool) -> i64 {
                inner.bsearch(Self::cow_to_arg(value), before)
            }

            fn op_reverse(inner: &mut Self::Inner<'_>) {
                inner.reverse()
            }

            fn op_sort(inner: &mut Self::Inner<'_>) {
                inner.sort()
            }

            fn inner<'a>(array: &PackedArray<$Element>) -> Self::Inner<'a> {
                crate::builtin::inner::$Inner::from_outer(array)
            }

            impl_packed_array_element!(@cow_to_arg $ArgConv);
        }
    };

    // Specialization for by-value/by-ref passing (only GString).
    (@cow_to_arg ByValue) => {
        fn cow_to_arg(value: CowArg<'_, Self>) -> Self::Arg<'_> {
            // into() promotes u8/i32 to i64 (FFI expects i64 for those).
            value.cow_into_owned().into()
        }
    };

    (@cow_to_arg ByRef) => {
        fn cow_to_arg(value: CowArg<'_, Self>) -> Self::Arg<'_> {
            // For ByRef types, we need to return a reference.
            // The trick is that Self::Arg<'_> for ByRef types is &'_ Self,
            // and cow_as_ref() returns the right reference.
            match value {
                CowArg::Owned(owned) => {
                    // TODO: This case should ideally not happen for ByRef types,
                    // but we're getting owned values in some cases (e.g. indexing operations).
                    // For now, leak the value to get a reference. This needs to be fixed properly.
                    Box::leak(Box::new(owned))
                }
                CowArg::Borrowed(r) => r,
            }
        }
    };

    // Specialization for Extend implementation.
    (@extend_dispatch u8, $packed_array:ident, $iter:ident) => {
        $packed_array.extend_fast($iter);
    };

    (@extend_dispatch $element:ty, $packed_array:ident, $iter:ident) => {
        for item in $iter {
            $packed_array.push_owned(item);
        }
    };
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Concrete impls for different element types

impl_packed_array_element!(
    element_type: u8,
    argument_type: i64,
    argument_conversion: ByValue,
    return_type: u8,
    variant_type: PACKED_BYTE_ARRAY,
    inner_type: InnerPackedByteArray,
    opaque_type: types::OpaquePackedByteArray,
    default_fn: packed_byte_array_construct_default,
    copy_fn: packed_byte_array_construct_copy,
    destroy_fn: packed_byte_array_destroy,
    equals_fn: packed_byte_array_operator_equal,
    to_variant_fn: packed_byte_array_to_variant,
    from_variant_fn: packed_byte_array_from_variant,
    index_mut_fn: packed_byte_array_operator_index,
    index_const_fn: packed_byte_array_operator_index_const,
);

impl_packed_array_element!(
    element_type: i32,
    argument_type: i64,
    argument_conversion: ByValue,
    return_type: i32,
    variant_type: PACKED_INT32_ARRAY,
    inner_type: InnerPackedInt32Array,
    opaque_type: types::OpaquePackedInt32Array,
    default_fn: packed_int32_array_construct_default,
    copy_fn: packed_int32_array_construct_copy,
    destroy_fn: packed_int32_array_destroy,
    equals_fn: packed_int32_array_operator_equal,
    to_variant_fn: packed_int32_array_to_variant,
    from_variant_fn: packed_int32_array_from_variant,
    index_mut_fn: packed_int32_array_operator_index,
    index_const_fn: packed_int32_array_operator_index_const,
);

impl_packed_array_element!(
    element_type: i64,
    argument_type: i64,
    argument_conversion: ByValue,
    return_type: i64,
    variant_type: PACKED_INT64_ARRAY,
    inner_type: InnerPackedInt64Array,
    opaque_type: types::OpaquePackedInt64Array,
    default_fn: packed_int64_array_construct_default,
    copy_fn: packed_int64_array_construct_copy,
    destroy_fn: packed_int64_array_destroy,
    equals_fn: packed_int64_array_operator_equal,
    to_variant_fn: packed_int64_array_to_variant,
    from_variant_fn: packed_int64_array_from_variant,
    index_mut_fn: packed_int64_array_operator_index,
    index_const_fn: packed_int64_array_operator_index_const,
);

impl_packed_array_element!(
    element_type: f32,
    argument_type: f64,
    argument_conversion: ByValue,
    return_type: f32,
    variant_type: PACKED_FLOAT32_ARRAY,
    inner_type: InnerPackedFloat32Array,
    opaque_type: types::OpaquePackedFloat32Array,
    default_fn: packed_float32_array_construct_default,
    copy_fn: packed_float32_array_construct_copy,
    destroy_fn: packed_float32_array_destroy,
    equals_fn: packed_float32_array_operator_equal,
    to_variant_fn: packed_float32_array_to_variant,
    from_variant_fn: packed_float32_array_from_variant,
    index_mut_fn: packed_float32_array_operator_index,
    index_const_fn: packed_float32_array_operator_index_const,
);

impl_packed_array_element!(
    element_type: f64,
    argument_type: f64,
    argument_conversion: ByValue,
    return_type: f64,
    variant_type: PACKED_FLOAT64_ARRAY,
    inner_type: InnerPackedFloat64Array,
    opaque_type: types::OpaquePackedFloat64Array,
    default_fn: packed_float64_array_construct_default,
    copy_fn: packed_float64_array_construct_copy,
    destroy_fn: packed_float64_array_destroy,
    equals_fn: packed_float64_array_operator_equal,
    to_variant_fn: packed_float64_array_to_variant,
    from_variant_fn: packed_float64_array_from_variant,
    index_mut_fn: packed_float64_array_operator_index,
    index_const_fn: packed_float64_array_operator_index_const,
);

impl_packed_array_element!(
    element_type: builtin::Vector2,
    argument_type: builtin::Vector2,
    argument_conversion: ByValue,
    return_type: __GdextType,
    variant_type: PACKED_VECTOR2_ARRAY,
    inner_type: InnerPackedVector2Array,
    opaque_type: types::OpaquePackedVector2Array,
    default_fn: packed_vector2_array_construct_default,
    copy_fn: packed_vector2_array_construct_copy,
    destroy_fn: packed_vector2_array_destroy,
    equals_fn: packed_vector2_array_operator_equal,
    to_variant_fn: packed_vector2_array_to_variant,
    from_variant_fn: packed_vector2_array_from_variant,
    index_mut_fn: packed_vector2_array_operator_index,
    index_const_fn: packed_vector2_array_operator_index_const,
);

impl_packed_array_element!(
    element_type: builtin::Vector3,
    argument_type: builtin::Vector3,
    argument_conversion: ByValue,
    return_type: __GdextType,
    variant_type: PACKED_VECTOR3_ARRAY,
    inner_type: InnerPackedVector3Array,
    opaque_type: types::OpaquePackedVector3Array,
    default_fn: packed_vector3_array_construct_default,
    copy_fn: packed_vector3_array_construct_copy,
    destroy_fn: packed_vector3_array_destroy,
    equals_fn: packed_vector3_array_operator_equal,
    to_variant_fn: packed_vector3_array_to_variant,
    from_variant_fn: packed_vector3_array_from_variant,
    index_mut_fn: packed_vector3_array_operator_index,
    index_const_fn: packed_vector3_array_operator_index_const,
);

#[cfg(since_api = "4.3")]
impl_packed_array_element!(
    element_type: builtin::Vector4,
    argument_type: builtin::Vector4,
    argument_conversion: ByValue,
    return_type: __GdextType,
    variant_type: PACKED_VECTOR4_ARRAY,
    inner_type: InnerPackedVector4Array,
    opaque_type: types::OpaquePackedVector4Array,
    default_fn: packed_vector4_array_construct_default,
    copy_fn: packed_vector4_array_construct_copy,
    destroy_fn: packed_vector4_array_destroy,
    equals_fn: packed_vector4_array_operator_equal,
    to_variant_fn: packed_vector4_array_to_variant,
    from_variant_fn: packed_vector4_array_from_variant,
    index_mut_fn: packed_vector4_array_operator_index,
    index_const_fn: packed_vector4_array_operator_index_const,
);

impl_packed_array_element!(
    element_type: builtin::Color,
    argument_type: builtin::Color,
    argument_conversion: ByValue,
    return_type: __GdextType,
    variant_type: PACKED_COLOR_ARRAY,
    inner_type: InnerPackedColorArray,
    opaque_type: types::OpaquePackedColorArray,
    default_fn: packed_color_array_construct_default,
    copy_fn: packed_color_array_construct_copy,
    destroy_fn: packed_color_array_destroy,
    equals_fn: packed_color_array_operator_equal,
    to_variant_fn: packed_color_array_to_variant,
    from_variant_fn: packed_color_array_from_variant,
    index_mut_fn: packed_color_array_operator_index,
    index_const_fn: packed_color_array_operator_index_const,
);

impl_packed_array_element!(
    element_type: builtin::GString,
    argument_type: &'a builtin::GString,
    argument_conversion: ByRef,
    return_type: __GdextString,
    variant_type: PACKED_STRING_ARRAY,
    inner_type: InnerPackedStringArray,
    opaque_type: types::OpaquePackedStringArray,
    default_fn: packed_string_array_construct_default,
    copy_fn: packed_string_array_construct_copy,
    destroy_fn: packed_string_array_destroy,
    equals_fn: packed_string_array_operator_equal,
    to_variant_fn: packed_string_array_to_variant,
    from_variant_fn: packed_string_array_from_variant,
    index_mut_fn: packed_string_array_operator_index,
    index_const_fn: packed_string_array_operator_index_const,
);
