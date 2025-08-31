/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use sys::{interface_fn, GodotFfi, SysPtr};

use crate::builtin::collections::extend_buffer::{ExtendBuffer, ExtendBufferTrait};
use crate::builtin::PackedArray;
use crate::meta::{CowArg, FromGodot, GodotType, ToGodot};
use crate::registry::property::builtin_type_string;
use crate::{builtin, sys};

/// Marker trait to identify types that can be stored in [`PackedArray<T>`][crate::builtin::PackedArray].
#[diagnostic::on_unimplemented(
    message = "`PackedArray<T>` can only store element types supported in Godot packed arrays.",
    label = "has invalid element type"
)]
// FromGodot isn't used, but can come in handy as an implied bound.
// ToGodot is needed for AsArg<T>.
pub trait PackedArrayElement: GodotType + Clone + ToGodot + FromGodot {
    /// Element variant type.
    #[doc(hidden)]
    const VARIANT_TYPE: sys::VariantType;

    /// Code-generated inner type, e.g. `InnerPackedStringArray`.
    #[doc(hidden)]
    type Inner<'a>;

    /// The type used for function arguments when passing elements, e.g. `&'a GString` or `i64`.
    #[doc(hidden)]
    type Arg<'a>;

    /// The pointee type returned from FFI index operations, e.g. `i64` or `sys::__GdextString` (opaque type behind `GDExtensionTypePtr`).
    #[doc(hidden)]
    type Indexed;

    /// ExtendBuffer type with appropriate capacity `N` for this element type.
    #[doc(hidden)]
    type ExtendBuffer: Default + ExtendBufferTrait<Self>;

    // ----------------------------------------------------------------------------------------------------------------------------------------------
    // Property-related API

    /// See [`crate::meta::traits::ArrayElement::element_type_string()`].
    #[doc(hidden)]
    fn element_type_string() -> String {
        builtin_type_string::<Self>()
    }

    // ----------------------------------------------------------------------------------------------------------------------------------------------
    // FFI operations

    #[doc(hidden)]
    unsafe fn ffi_to_variant(
        type_ptr: sys::GDExtensionConstTypePtr,
        variant_ptr: sys::GDExtensionVariantPtr,
    );

    #[doc(hidden)]
    unsafe fn ffi_from_variant(
        variant_ptr: sys::GDExtensionConstVariantPtr,
        type_ptr: sys::GDExtensionTypePtr,
    );

    #[doc(hidden)]
    unsafe fn ffi_default(type_ptr: sys::GDExtensionTypePtr);

    #[doc(hidden)]
    unsafe fn ffi_copy(src_ptr: sys::GDExtensionConstTypePtr, dst_ptr: sys::GDExtensionTypePtr);

    #[doc(hidden)]
    unsafe fn ffi_destroy(type_ptr: sys::GDExtensionTypePtr);

    #[doc(hidden)]
    unsafe fn ffi_equals(
        left_ptr: sys::GDExtensionConstTypePtr,
        right_ptr: sys::GDExtensionConstTypePtr,
    ) -> bool;

    #[doc(hidden)]
    unsafe fn ffi_index_const(
        type_ptr: sys::GDExtensionConstTypePtr,
        index: i64,
    ) -> *const Self::Indexed;

    #[doc(hidden)]
    unsafe fn ffi_index_mut(type_ptr: sys::GDExtensionTypePtr, index: i64) -> *mut Self::Indexed;

    // ----------------------------------------------------------------------------------------------------------------------------------------------
    // Delegates to inner Packed*Array API

    #[doc(hidden)]
    fn op_has(inner: Self::Inner<'_>, value: CowArg<'_, Self>) -> bool;

    #[doc(hidden)]
    fn op_count(inner: Self::Inner<'_>, value: CowArg<'_, Self>) -> i64;

    #[doc(hidden)]
    fn op_size(inner: Self::Inner<'_>) -> i64;

    #[doc(hidden)]
    fn op_is_empty(inner: Self::Inner<'_>) -> bool;

    #[doc(hidden)]
    fn op_clear(inner: Self::Inner<'_>);

    #[doc(hidden)]
    fn op_push_back(inner: Self::Inner<'_>, value: CowArg<'_, Self>);

    #[doc(hidden)]
    fn op_insert(inner: Self::Inner<'_>, index: i64, value: CowArg<'_, Self>);

    #[doc(hidden)]
    fn op_remove_at(inner: Self::Inner<'_>, index: i64);

    #[doc(hidden)]
    fn op_fill(inner: Self::Inner<'_>, value: CowArg<'_, Self>);

    #[doc(hidden)]
    fn op_resize(inner: Self::Inner<'_>, size: i64);

    #[doc(hidden)]
    fn op_append_array(inner: Self::Inner<'_>, other: &PackedArray<Self>);

    #[doc(hidden)]
    fn op_slice(inner: Self::Inner<'_>, begin: i64, end: i64) -> PackedArray<Self>;

    #[doc(hidden)]
    fn op_find(inner: Self::Inner<'_>, value: CowArg<'_, Self>, from: i64) -> i64;

    #[doc(hidden)]
    fn op_rfind(inner: Self::Inner<'_>, value: CowArg<'_, Self>, from: i64) -> i64;

    #[doc(hidden)]
    fn op_bsearch(inner: Self::Inner<'_>, value: CowArg<'_, Self>, before: bool) -> i64;

    #[doc(hidden)]
    fn op_reverse(inner: Self::Inner<'_>);

    #[doc(hidden)]
    fn op_sort(inner: Self::Inner<'_>);

    #[doc(hidden)]
    fn inner<'a>(array: &PackedArray<Self>) -> Self::Inner<'a>;

    /// Call inner function with an element-type argument.
    ///
    /// Has this functional design for a reason: to pass `CowArg` as either value or ref, it needs to be consumed. However we can then not
    /// return a reference, as the `CowArg` would be dropped inside the function.
    #[doc(hidden)]
    fn with_arg<F, R>(value: CowArg<'_, Self>, f: F) -> R
    where
        F: FnOnce(Self::Arg<'_>) -> R;
}

/// Helper because `usize::max()` is not const.
const fn const_max(a: usize, b: usize) -> usize {
    if a > b {
        a
    } else {
        b
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Macro to implement the spec for a concrete element type

macro_rules! impl_packed_array_element {
    (
        element_type: $Element:ty,
        argument_type: $Arg:ty,
        argument_pass: $ArgPass:ident,
        indexed_type: $IndexRetType:ty,
        variant_type: $VariantType:ident,
        inner_type: $Inner:ident,
        default_fn: $default_fn:ident,
        copy_fn: $copy_fn:ident,
        destroy_fn: $destroy_fn:ident,
        to_variant_fn: $to_variant_fn:ident,
        from_variant_fn: $from_variant_fn:ident,
        equals_fn: $equals_fn:ident,
        index_mut_fn: $index_mut_fn:ident,
        index_const_fn: $index_const_fn:ident,
    ) => {
        impl PackedArrayElement for $Element {
            const VARIANT_TYPE: sys::VariantType = sys::VariantType::$VariantType;

            type Inner<'a> = crate::builtin::inner::$Inner<'a>;
            type Arg<'a> = $Arg;
            type Indexed = $IndexRetType;
            type ExtendBuffer = ExtendBuffer<$Element, {
                const_max(1, 2048 / std::mem::size_of::<$Element>())
            }>;

            unsafe fn ffi_default(type_ptr: sys::GDExtensionTypePtr) {
                let constructor = sys::builtin_fn!($default_fn);
                constructor(SysPtr::as_uninit(type_ptr), std::ptr::null_mut());
            }

            unsafe fn ffi_copy(src_ptr: sys::GDExtensionConstTypePtr, dst_ptr: sys::GDExtensionTypePtr) {
                let constructor = sys::builtin_fn!($copy_fn);
                let args = [src_ptr];
                constructor(SysPtr::as_uninit(dst_ptr), args.as_ptr());
            }

            unsafe fn ffi_destroy(type_ptr: sys::GDExtensionTypePtr) {
                let destructor = sys::builtin_fn!($destroy_fn @1);
                destructor(type_ptr);
            }

            unsafe fn ffi_to_variant(type_ptr: sys::GDExtensionConstTypePtr, variant_ptr: sys::GDExtensionVariantPtr) {
                let converter = sys::builtin_fn!($to_variant_fn);
                converter(SysPtr::as_uninit(variant_ptr), SysPtr::force_mut(type_ptr));
            }

            unsafe fn ffi_from_variant(variant_ptr: sys::GDExtensionConstVariantPtr, type_ptr: sys::GDExtensionTypePtr) {
                let converter = sys::builtin_fn!($from_variant_fn);
                converter(SysPtr::as_uninit(type_ptr), SysPtr::force_mut(variant_ptr));
            }

            unsafe fn ffi_equals(left_ptr: sys::GDExtensionConstTypePtr, right_ptr: sys::GDExtensionConstTypePtr) -> bool {
                let mut result = false;
                sys::builtin_call! {
                    $equals_fn(left_ptr, right_ptr, result.sys_mut())
                };
                result
            }

            unsafe fn ffi_index_const(type_ptr: sys::GDExtensionConstTypePtr, index: i64) -> *const Self::Indexed {
                unsafe {
                    interface_fn!($index_const_fn)(type_ptr, index)
                }
            }

            unsafe fn ffi_index_mut(type_ptr: sys::GDExtensionTypePtr, index: i64) -> *mut Self::Indexed {
                unsafe {
                    interface_fn!($index_mut_fn)(type_ptr, index)
                }
            }

            fn op_has(inner: Self::Inner<'_>, value: CowArg<'_, Self>) -> bool {
                Self::with_arg(value, |arg| inner.has(arg))
            }

            fn op_count(inner: Self::Inner<'_>, value: CowArg<'_, Self>) -> i64 {
                Self::with_arg(value, |arg| inner.count(arg))
            }

            fn op_size(inner: Self::Inner<'_>) -> i64 {
                inner.size()
            }

            fn op_is_empty(inner: Self::Inner<'_>) -> bool {
                inner.is_empty()
            }

            fn op_clear(mut inner: Self::Inner<'_>) {
                inner.clear();
            }

            fn op_push_back(mut inner: Self::Inner<'_>, value: CowArg<'_, Self>) {
                Self::with_arg(value, |arg| inner.push_back(arg));
            }

            fn op_insert(mut inner: Self::Inner<'_>, index: i64, value: CowArg<'_, Self>) {
                Self::with_arg(value, |arg| inner.insert(index, arg));
            }

            fn op_remove_at(mut inner: Self::Inner<'_>, index: i64) {
                inner.remove_at(index);
            }

            fn op_fill(mut inner: Self::Inner<'_>, value: CowArg<'_, Self>) {
                Self::with_arg(value, |arg| inner.fill(arg));
            }

            fn op_resize(mut inner: Self::Inner<'_>, size: i64) {
                inner.resize(size);
            }

            fn op_append_array(mut inner: Self::Inner<'_>, other: &PackedArray<Self>) {
                inner.append_array(other);
            }

            fn op_slice(inner: Self::Inner<'_>, begin: i64, end: i64) -> PackedArray<Self> {
                inner.slice(begin, end)
            }

            fn op_find(inner: Self::Inner<'_>, value: CowArg<'_, Self>, from: i64) -> i64 {
                Self::with_arg(value, |arg| inner.find(arg, from))
            }

            fn op_rfind(inner: Self::Inner<'_>, value: CowArg<'_, Self>, from: i64) -> i64 {
                Self::with_arg(value, |arg| inner.rfind(arg, from))
            }

            fn op_bsearch(mut inner: Self::Inner<'_>, value: CowArg<'_, Self>, before: bool) -> i64 {
                Self::with_arg(value, |arg| inner.bsearch(arg, before))
            }

            fn op_reverse(mut inner: Self::Inner<'_>) {
                inner.reverse()
            }

            fn op_sort(mut inner: Self::Inner<'_>) {
                inner.sort()
            }

            fn inner<'a>(array: &PackedArray<$Element>) -> Self::Inner<'a> {
                crate::builtin::inner::$Inner::from_outer(array)
            }

            impl_packed_array_element!(@with_arg $ArgPass);
        }
    };

    // Specialization for by-value/by-ref passing (only GString).
    (@with_arg ByValue) => {
        fn with_arg<F, R>(value: CowArg<'_, Self>, f: F) -> R
        where
            F: FnOnce(Self::Arg<'_>) -> R,
        {
            // into() allows conversions from u8|i32 -> i64 (Godot APIs take i64 even for Packed{Byte,Int32}Array).
            f(value.cow_into_owned().into())
        }
    };

    (@with_arg ByRef) => {
        fn with_arg<F, R>(value: CowArg<'_, Self>, f: F) -> R
        where
            F: FnOnce(Self::Arg<'_>) -> R,
        {
            f(value.cow_as_ref())
        }
    };
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Concrete impls for different element types

impl_packed_array_element!(
    element_type: u8,
    argument_type: i64,
    argument_pass: ByValue,
    indexed_type: u8,
    variant_type: PACKED_BYTE_ARRAY,
    inner_type: InnerPackedByteArray,
    default_fn: packed_byte_array_construct_default,
    copy_fn: packed_byte_array_construct_copy,
    destroy_fn: packed_byte_array_destroy,
    to_variant_fn: packed_byte_array_to_variant,
    from_variant_fn: packed_byte_array_from_variant,
    equals_fn: packed_byte_array_operator_equal,
    index_mut_fn: packed_byte_array_operator_index,
    index_const_fn: packed_byte_array_operator_index_const,
);

impl_packed_array_element!(
    element_type: i32,
    argument_type: i64,
    argument_pass: ByValue,
    indexed_type: i32,
    variant_type: PACKED_INT32_ARRAY,
    inner_type: InnerPackedInt32Array,
    default_fn: packed_int32_array_construct_default,
    copy_fn: packed_int32_array_construct_copy,
    destroy_fn: packed_int32_array_destroy,
    to_variant_fn: packed_int32_array_to_variant,
    from_variant_fn: packed_int32_array_from_variant,
    equals_fn: packed_int32_array_operator_equal,
    index_mut_fn: packed_int32_array_operator_index,
    index_const_fn: packed_int32_array_operator_index_const,
);

impl_packed_array_element!(
    element_type: i64,
    argument_type: i64,
    argument_pass: ByValue,
    indexed_type: i64,
    variant_type: PACKED_INT64_ARRAY,
    inner_type: InnerPackedInt64Array,
    default_fn: packed_int64_array_construct_default,
    copy_fn: packed_int64_array_construct_copy,
    destroy_fn: packed_int64_array_destroy,
    to_variant_fn: packed_int64_array_to_variant,
    from_variant_fn: packed_int64_array_from_variant,
    equals_fn: packed_int64_array_operator_equal,
    index_mut_fn: packed_int64_array_operator_index,
    index_const_fn: packed_int64_array_operator_index_const,
);

impl_packed_array_element!(
    element_type: f32,
    argument_type: f64,
    argument_pass: ByValue,
    indexed_type: f32,
    variant_type: PACKED_FLOAT32_ARRAY,
    inner_type: InnerPackedFloat32Array,
    default_fn: packed_float32_array_construct_default,
    copy_fn: packed_float32_array_construct_copy,
    destroy_fn: packed_float32_array_destroy,
    to_variant_fn: packed_float32_array_to_variant,
    from_variant_fn: packed_float32_array_from_variant,
    equals_fn: packed_float32_array_operator_equal,
    index_mut_fn: packed_float32_array_operator_index,
    index_const_fn: packed_float32_array_operator_index_const,
);

impl_packed_array_element!(
    element_type: f64,
    argument_type: f64,
    argument_pass: ByValue,
    indexed_type: f64,
    variant_type: PACKED_FLOAT64_ARRAY,
    inner_type: InnerPackedFloat64Array,
    default_fn: packed_float64_array_construct_default,
    copy_fn: packed_float64_array_construct_copy,
    destroy_fn: packed_float64_array_destroy,
    to_variant_fn: packed_float64_array_to_variant,
    from_variant_fn: packed_float64_array_from_variant,
    equals_fn: packed_float64_array_operator_equal,
    index_mut_fn: packed_float64_array_operator_index,
    index_const_fn: packed_float64_array_operator_index_const,
);

impl_packed_array_element!(
    element_type: builtin::Vector2,
    argument_type: builtin::Vector2,
    argument_pass: ByValue,
    indexed_type: sys::__GdextType,
    variant_type: PACKED_VECTOR2_ARRAY,
    inner_type: InnerPackedVector2Array,
    default_fn: packed_vector2_array_construct_default,
    copy_fn: packed_vector2_array_construct_copy,
    destroy_fn: packed_vector2_array_destroy,
    to_variant_fn: packed_vector2_array_to_variant,
    from_variant_fn: packed_vector2_array_from_variant,
    equals_fn: packed_vector2_array_operator_equal,
    index_mut_fn: packed_vector2_array_operator_index,
    index_const_fn: packed_vector2_array_operator_index_const,
);

impl_packed_array_element!(
    element_type: builtin::Vector3,
    argument_type: builtin::Vector3,
    argument_pass: ByValue,
    indexed_type: sys::__GdextType,
    variant_type: PACKED_VECTOR3_ARRAY,
    inner_type: InnerPackedVector3Array,
    default_fn: packed_vector3_array_construct_default,
    copy_fn: packed_vector3_array_construct_copy,
    destroy_fn: packed_vector3_array_destroy,
    to_variant_fn: packed_vector3_array_to_variant,
    from_variant_fn: packed_vector3_array_from_variant,
    equals_fn: packed_vector3_array_operator_equal,
    index_mut_fn: packed_vector3_array_operator_index,
    index_const_fn: packed_vector3_array_operator_index_const,
);

#[cfg(since_api = "4.3")]
impl_packed_array_element!(
    element_type: builtin::Vector4,
    argument_type: builtin::Vector4,
    argument_pass: ByValue,
    indexed_type: sys::__GdextType,
    variant_type: PACKED_VECTOR4_ARRAY,
    inner_type: InnerPackedVector4Array,
    default_fn: packed_vector4_array_construct_default,
    copy_fn: packed_vector4_array_construct_copy,
    destroy_fn: packed_vector4_array_destroy,
    to_variant_fn: packed_vector4_array_to_variant,
    from_variant_fn: packed_vector4_array_from_variant,
    equals_fn: packed_vector4_array_operator_equal,
    index_mut_fn: packed_vector4_array_operator_index,
    index_const_fn: packed_vector4_array_operator_index_const,
);

impl_packed_array_element!(
    element_type: builtin::Color,
    argument_type: builtin::Color,
    argument_pass: ByValue,
    indexed_type: sys::__GdextType,
    variant_type: PACKED_COLOR_ARRAY,
    inner_type: InnerPackedColorArray,
    default_fn: packed_color_array_construct_default,
    copy_fn: packed_color_array_construct_copy,
    destroy_fn: packed_color_array_destroy,
    to_variant_fn: packed_color_array_to_variant,
    from_variant_fn: packed_color_array_from_variant,
    equals_fn: packed_color_array_operator_equal,
    index_mut_fn: packed_color_array_operator_index,
    index_const_fn: packed_color_array_operator_index_const,
);

impl_packed_array_element!(
    element_type: builtin::GString,
    argument_type: &'a builtin::GString,
    argument_pass: ByRef,
    indexed_type: sys::__GdextString,
    variant_type: PACKED_STRING_ARRAY,
    inner_type: InnerPackedStringArray,
    default_fn: packed_string_array_construct_default,
    copy_fn: packed_string_array_construct_copy,
    destroy_fn: packed_string_array_destroy,
    to_variant_fn: packed_string_array_to_variant,
    from_variant_fn: packed_string_array_from_variant,
    equals_fn: packed_string_array_operator_equal,
    index_mut_fn: packed_string_array_operator_index,
    index_const_fn: packed_string_array_operator_index_const,
);
