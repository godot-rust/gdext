/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use godot_ffi as sys;
use sys::GodotFfi;

use super::rust_variant::{RustVariant, USE_RUST_MARSHAL};
use crate::builtin::*;
use crate::meta::error::{ConvertError, FromVariantError};
use crate::meta::sealed::Sealed;
use crate::meta::{Element, GodotFfiVariant, GodotType, RefArg};
use crate::registry::info::ParamMetadata;
use crate::task::{DynamicSend, IntoDynamicSend, ThreadConfined, impl_dynamic_send};

// For godot-cpp, see https://github.com/godotengine/godot-cpp/blob/master/include/godot_cpp/core/type_info.hpp.

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Macro definitions

// Historical note: In Godot 4.0, certain types needed to be passed as initialized pointers in their from_variant implementations, because
// 4.0 used `*ptr = value` to return the type, and some types in C++ override `operator=` in a way that requires the pointer to be initialized.
// However, those same types would cause memory leaks in Godot 4.1 if pre-initialized. A compat layer `new_with_uninit_or_init()` addressed this.
// As these Godot versions are no longer supported, the current implementation uses `new_with_uninit()` uniformly for all versions.
macro_rules! impl_ffi_variant {
    // Entry points with RustVariant optimization. By-val only: all current RustMarshal types are POD and passed by value; no
    // by-ref arm exists since nothing invokes it (RustMarshal types don't need a by-ref conversion path).
    ($T:ty, $from_fn:ident, $to_fn:ident; $metadata:expr, @rust_variant) => {
        impl_ffi_variant!(@rust_variant_impls by_val, $metadata; $T, $from_fn, $to_fn);
    };
    ($T:ty, $from_fn:ident, $to_fn:ident, @rust_variant) => {
        impl_ffi_variant!(@rust_variant_impls by_val, ParamMetadata::NONE; $T, $from_fn, $to_fn);
    };

    // Entry points without RustVariant (standard FFI path). Use @ffi to opt in explicitly.
    (ref $T:ty, $from_fn:ident, $to_fn:ident; $metadata:expr, @ffi) => {
        impl_ffi_variant!(@ffi_only_impls by_ref, $metadata, main_thread; $T, $from_fn, $to_fn);
    };
    ($T:ty, $from_fn:ident, $to_fn:ident; $metadata:expr, @ffi) => {
        impl_ffi_variant!(@ffi_only_impls by_val, $metadata, main_thread; $T, $from_fn, $to_fn);
    };
    (ref $T:ty, $from_fn:ident, $to_fn:ident, @ffi) => {
        impl_ffi_variant!(@ffi_only_impls by_ref, ParamMetadata::NONE, main_thread; $T, $from_fn, $to_fn);
    };
    ($T:ty, $from_fn:ident, $to_fn:ident, @ffi) => {
        impl_ffi_variant!(@ffi_only_impls by_val, ParamMetadata::NONE, main_thread; $T, $from_fn, $to_fn);
    };

    // Thread-safe variant: the to/from-variant converters resolve through the reviewed `sys::thread_safe_lifecycle()` subset instead of the
    // main-thread-only `builtin_fn!` (string value types only touch caller-owned memory).
    (thread_safe ref $T:ty, $from_fn:ident, $to_fn:ident, @ffi) => {
        impl_ffi_variant!(@ffi_only_impls by_ref, ParamMetadata::NONE, thread_safe; $T, $from_fn, $to_fn);
    };

    // Converter resolution: `main_thread` uses the main-thread table, `thread_safe` the reviewed subset.
    (@converter main_thread, $fn:ident) => { sys::builtin_fn!($fn) };
    (@converter thread_safe, $fn:ident) => { sys::thread_safe_lifecycle().$fn };

    // Shared to/from-variant bodies for `@rust_variant_impls` fallback and `@ffi_only_impls`. Routing through `@converter $mode`
    // keeps a `thread_safe` type off the main-thread table. `$self`/`$variant` passed explicitly (macro hygiene).
    (@ffi_to_variant_body $self:expr, $mode:ident, $from_fn:ident) => {
        unsafe {
            Variant::new_with_var_uninit(|variant_ptr| {
                let converter = impl_ffi_variant!(@converter $mode, $from_fn);
                converter(variant_ptr, sys::SysPtr::force_mut(($self).sys()));
            })
        }
    };
    (@ffi_from_variant_body $variant:expr, $mode:ident, $to_fn:ident) => {
        {
            let variant = $variant;
            if variant.get_type() != Self::VARIANT_TYPE.variant_as_nil() {
                return Err(FromVariantError::BadType {
                    expected: Self::VARIANT_TYPE.variant_as_nil(),
                    actual: variant.get_type(),
                }
                .into_error(variant.clone()));
            }

            let result = unsafe {
                Self::new_with_uninit(|self_ptr| {
                    let converter = impl_ffi_variant!(@converter $mode, $to_fn);
                    converter(self_ptr, sys::SysPtr::force_mut(variant.var_sys()));
                })
            };

            Ok(result)
        }
    };

    // Implementation with RustVariant optimization.
    (@rust_variant_impls $by_ref_or_val:ident, $metadata:expr; $T:ty, $from_fn:ident, $to_fn:ident) => {
        // Single source of truth for the RustMarshal type set: each `@rust_variant` invocation registers the type and checks the size
        // precondition here (regardless of feature flags), so marker impl and conversion path cannot drift.
        // SAFETY: `@rust_variant` is only used for `#[repr(C)]` POD types matching Godot's in-memory layout; the `assert!` below upholds the size contract.
        const _: () = {
            use crate::builtin::variant::rust_variant::{RustMarshal, VARIANT_DATA_SIZE};
            assert!(
                std::mem::size_of::<$T>() <= VARIANT_DATA_SIZE,
                "Type is too large for RustVariant"
            );
            assert!(
                <$T as RustMarshal>::VARIANT_TYPE.is_inplace_variant(),
                "RustMarshal type must be stored in-place in Variant"
            );
        };
        // Full path avoids importing `RustMarshal`, which would make `Self::VARIANT_TYPE` below ambiguous (also defined on `GodotFfi`).
        unsafe impl crate::builtin::variant::rust_variant::RustMarshal for $T {}

        impl GodotFfiVariant for $T {
            fn ffi_to_variant(&self) -> Variant {
                if USE_RUST_MARSHAL {
                    RustVariant::from_pod(*self)
                } else {
                    impl_ffi_variant!(@ffi_to_variant_body self, main_thread, $from_fn)
                }
            }

            fn ffi_from_variant(variant: &Variant) -> Result<Self, ConvertError> {
                if USE_RUST_MARSHAL {
                    return RustVariant::view(variant).get_value::<Self>().ok_or_else(|| {
                        FromVariantError::BadType {
                            expected: Self::VARIANT_TYPE.variant_as_nil(),
                            actual: variant.get_type(),
                        }
                        .into_error(variant.clone())
                    });
                }

                impl_ffi_variant!(@ffi_from_variant_body variant, main_thread, $to_fn)
            }
        }

        impl_ffi_variant!(@shared_impls $by_ref_or_val, $metadata; $T);
    };

    // Implementation without RustVariant (standard FFI, with converter mode selection).
    (@ffi_only_impls $by_ref_or_val:ident, $metadata:expr, $mode:ident; $T:ty, $from_fn:ident, $to_fn:ident) => {
        impl GodotFfiVariant for $T {
            fn ffi_to_variant(&self) -> Variant {
                impl_ffi_variant!(@ffi_to_variant_body self, $mode, $from_fn)
            }

            fn ffi_from_variant(variant: &Variant) -> Result<Self, ConvertError> {
                impl_ffi_variant!(@ffi_from_variant_body variant, $mode, $to_fn)
            }
        }

        impl_ffi_variant!(@shared_impls $by_ref_or_val, $metadata; $T);
    };

    // Shared implementations (GodotType, Element).
    (@shared_impls $by_ref_or_val:ident, $metadata:expr; $T:ty) => {
        impl GodotType for $T {
            type Ffi = Self;
            impl_ffi_variant!(@assoc_to_ffi $by_ref_or_val);

            fn into_ffi(self) -> Self::Ffi {
                self
            }

            fn try_from_ffi(ffi: Self::Ffi) -> Result<Self, ConvertError> {
                Ok(ffi)
            }

            fn default_metadata() -> ParamMetadata {
                $metadata
            }
        }

        impl Element for $T {}
    };

    (@assoc_to_ffi by_ref) => {
        type ToFfi<'a> =  RefArg<'a, Self>;

        fn to_ffi(&self) -> Self::ToFfi<'_> {
            RefArg::new(self)
        }
    };

    (@assoc_to_ffi by_val) => {
        type ToFfi<'a> = Self;

        fn to_ffi(&self) -> Self::ToFfi<'_> {
            self.clone()
        }
    };
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// General impls

#[rustfmt::skip]
#[allow(clippy::module_inception)]
mod impls {
    use super::*;

    // IMPORTANT: the presence/absence of `ref` here should be aligned with the ArgPassing variant
    // used in codegen get_builtin_arg_passing().

    // Types with RustVariant optimization (fit in Variant data, no destructors).
    impl_ffi_variant!(bool, bool_to_variant, bool_from_variant, @rust_variant);
    impl_ffi_variant!(i64, int_to_variant, int_from_variant; ParamMetadata::INT_IS_INT64, @rust_variant);
    impl_ffi_variant!(f64, float_to_variant, float_from_variant; ParamMetadata::REAL_IS_DOUBLE, @rust_variant);
    impl_ffi_variant!(Vector2i, vector2i_to_variant, vector2i_from_variant, @rust_variant);
    impl_ffi_variant!(Vector3i, vector3i_to_variant, vector3i_from_variant, @rust_variant);
    impl_ffi_variant!(Vector4i, vector4i_to_variant, vector4i_from_variant, @rust_variant);
    impl_ffi_variant!(Color, color_to_variant, color_from_variant, @rust_variant);
    impl_ffi_variant!(Rect2i, rect2i_to_variant, rect2i_from_variant, @rust_variant);
    impl_ffi_variant!(Rid, rid_to_variant, rid_from_variant, @rust_variant);

    // Precision-dependent types with RustVariant optimization.
    impl_ffi_variant!(Vector2, vector2_to_variant, vector2_from_variant, @rust_variant);
    impl_ffi_variant!(Vector3, vector3_to_variant, vector3_from_variant, @rust_variant);
    impl_ffi_variant!(Vector4, vector4_to_variant, vector4_from_variant, @rust_variant);
    impl_ffi_variant!(Quaternion, quaternion_to_variant, quaternion_from_variant, @rust_variant);
    impl_ffi_variant!(Plane, plane_to_variant, plane_from_variant, @rust_variant);
    impl_ffi_variant!(Rect2, rect2_to_variant, rect2_from_variant, @rust_variant);

    // Large value types: heap-allocated inside Variant, not eligible for RustMarshal (see is_inplace_variant() doc).
    impl_ffi_variant!(Transform2D, transform_2d_to_variant, transform_2d_from_variant, @ffi);
    impl_ffi_variant!(Transform3D, transform_3d_to_variant, transform_3d_from_variant, @ffi);
    impl_ffi_variant!(Basis, basis_to_variant, basis_from_variant, @ffi);
    impl_ffi_variant!(Projection, projection_to_variant, projection_from_variant, @ffi);
    impl_ffi_variant!(Aabb, aabb_to_variant, aabb_from_variant, @ffi);

    // GString and StringName are string value types that only touch caller-owned memory, so their variant conversions are thread-safe.
    impl_ffi_variant!(thread_safe ref GString, string_to_variant, string_from_variant, @ffi);
    impl_ffi_variant!(thread_safe ref StringName, string_name_to_variant, string_name_from_variant, @ffi);

    // Ref-counted types: require FFI for construction/destruction; RustMarshal is not applicable.
    impl_ffi_variant!(ref NodePath, node_path_to_variant, node_path_from_variant, @ffi);
    impl_ffi_variant!(ref Signal, signal_to_variant, signal_from_variant, @ffi);
    impl_ffi_variant!(ref Callable, callable_to_variant, callable_from_variant, @ffi);
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Async trait support

impl<T: Element> Sealed for ThreadConfined<Array<T>> {}

unsafe impl<T: Element> DynamicSend for ThreadConfined<Array<T>> {
    type Inner = Array<T>;
    fn extract_if_safe(self) -> Option<Self::Inner> {
        self.extract()
    }
}

impl<T: Element> IntoDynamicSend for Array<T> {
    type Target = ThreadConfined<Array<T>>;
    fn into_dynamic_send(self) -> Self::Target {
        ThreadConfined::new(self)
    }
}

impl_dynamic_send!(
    Send;
    bool, u8, u16, u32, u64, i8, i16, i32, i64, f32, f64
);

impl_dynamic_send!(
    Send;
    StringName, Color, Rid,
    Vector2, Vector2i, Vector2Axis,
    Vector3, Vector3i, Vector3Axis,
    Vector4, Vector4i,
    Rect2, Rect2i, Aabb,
    Transform2D, Transform3D, Basis,
    Plane, Quaternion, Projection
);

impl_dynamic_send!(
    !Send;
    Variant, NodePath, GString, VarDictionary, Callable, Signal,
    PackedByteArray, PackedInt32Array, PackedInt64Array, PackedFloat32Array, PackedFloat64Array, PackedStringArray,
    PackedVector2Array, PackedVector3Array, PackedColorArray
);

// Keep in sync with `impl_signal_recipient!` invocations in crate::signal::signal_receiver.
impl_dynamic_send!(tuple; );
impl_dynamic_send!(tuple; arg1: A1);
impl_dynamic_send!(tuple; arg1: A1, arg2: A2);
impl_dynamic_send!(tuple; arg1: A1, arg2: A2, arg3: A3);
impl_dynamic_send!(tuple; arg1: A1, arg2: A2, arg3: A3, arg4: A4);
impl_dynamic_send!(tuple; arg1: A1, arg2: A2, arg3: A3, arg4: A4, arg5: A5);
impl_dynamic_send!(tuple; arg1: A1, arg2: A2, arg3: A3, arg4: A4, arg5: A5, arg6: A6);
impl_dynamic_send!(tuple; arg1: A1, arg2: A2, arg3: A3, arg4: A4, arg5: A5, arg6: A6, arg7: A7);
impl_dynamic_send!(tuple; arg1: A1, arg2: A2, arg3: A3, arg4: A4, arg5: A5, arg6: A6, arg7: A7, arg8: A8);
impl_dynamic_send!(tuple; arg1: A1, arg2: A2, arg3: A3, arg4: A4, arg5: A5, arg6: A6, arg7: A7, arg8: A8, arg9: A9);

#[cfg(since_api = "4.3")]
mod api_4_3 {
    use crate::task::impl_dynamic_send;

    impl_dynamic_send!(!Send; PackedVector4Array);
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Internal verification

// Compile time check that we cover all the Variant types with trait implementations for:
// - IntoDynamicSend
// - DynamicSend
// - GodotType
// - Element
const _: () = {
    use crate::classes::Object;
    use crate::obj::{Gd, IndexEnum};

    const fn variant_type<T: crate::task::IntoDynamicSend + GodotType + Element>() -> VariantType {
        <T::Ffi as sys::GodotFfi>::VARIANT_TYPE.variant_as_nil()
    }

    const NIL: VariantType = variant_type::<Variant>();
    const BOOL: VariantType = variant_type::<bool>();
    const I64: VariantType = variant_type::<i64>();
    const F64: VariantType = variant_type::<f64>();
    const GSTRING: VariantType = variant_type::<GString>();

    const VECTOR2: VariantType = variant_type::<Vector2>();
    const VECTOR2I: VariantType = variant_type::<Vector2i>();
    const RECT2: VariantType = variant_type::<Rect2>();
    const RECT2I: VariantType = variant_type::<Rect2i>();
    const VECTOR3: VariantType = variant_type::<Vector3>();
    const VECTOR3I: VariantType = variant_type::<Vector3i>();
    const TRANSFORM2D: VariantType = variant_type::<Transform2D>();
    const TRANSFORM3D: VariantType = variant_type::<Transform3D>();
    const VECTOR4: VariantType = variant_type::<Vector4>();
    const VECTOR4I: VariantType = variant_type::<Vector4i>();
    const PLANE: VariantType = variant_type::<Plane>();
    const QUATERNION: VariantType = variant_type::<Quaternion>();
    const AABB: VariantType = variant_type::<Aabb>();
    const BASIS: VariantType = variant_type::<Basis>();
    const PROJECTION: VariantType = variant_type::<Projection>();
    const COLOR: VariantType = variant_type::<Color>();
    const STRING_NAME: VariantType = variant_type::<StringName>();
    const NODE_PATH: VariantType = variant_type::<NodePath>();
    const RID: VariantType = variant_type::<Rid>();
    const OBJECT: VariantType = variant_type::<Gd<Object>>();
    const CALLABLE: VariantType = variant_type::<Callable>();
    const SIGNAL: VariantType = variant_type::<Signal>();
    const DICTIONARY: VariantType = variant_type::<VarDictionary>();
    const ARRAY: VariantType = variant_type::<VarArray>();
    const PACKED_BYTE_ARRAY: VariantType = variant_type::<PackedByteArray>();
    const PACKED_INT32_ARRAY: VariantType = variant_type::<PackedInt32Array>();
    const PACKED_INT64_ARRAY: VariantType = variant_type::<PackedInt64Array>();
    const PACKED_FLOAT32_ARRAY: VariantType = variant_type::<PackedFloat32Array>();
    const PACKED_FLOAT64_ARRAY: VariantType = variant_type::<PackedFloat64Array>();
    const PACKED_STRING_ARRAY: VariantType = variant_type::<PackedStringArray>();
    const PACKED_VECTOR2_ARRAY: VariantType = variant_type::<PackedVector2Array>();
    const PACKED_VECTOR3_ARRAY: VariantType = variant_type::<PackedVector3Array>();
    const PACKED_COLOR_ARRAY: VariantType = variant_type::<PackedColorArray>();

    #[cfg(since_api = "4.3")]
    const PACKED_VECTOR4_ARRAY: VariantType = variant_type::<PackedVector4Array>();

    const MAX: i32 = VariantType::ENUMERATOR_COUNT as i32;

    // The matched value is not relevant, we just want to ensure that the full list from 0 to MAX is covered.
    #[deny(unreachable_patterns)]
    match VariantType::STRING {
        VariantType { ord: i32::MIN..0 } => panic!("ord is out of defined range!"),
        NIL => (),
        BOOL => (),
        I64 => (),
        F64 => (),
        GSTRING => (),
        VECTOR2 => (),
        VECTOR2I => (),
        RECT2 => (),
        RECT2I => (),
        VECTOR3 => (),
        VECTOR3I => (),
        TRANSFORM2D => (),
        VECTOR4 => (),
        VECTOR4I => (),
        PLANE => (),
        QUATERNION => (),
        AABB => (),
        BASIS => (),
        TRANSFORM3D => (),
        PROJECTION => (),
        COLOR => (),
        STRING_NAME => (),
        NODE_PATH => (),
        RID => (),
        OBJECT => (),
        CALLABLE => (),
        SIGNAL => (),
        DICTIONARY => (),
        ARRAY => (),
        PACKED_BYTE_ARRAY => (),
        PACKED_INT32_ARRAY => (),
        PACKED_INT64_ARRAY => (),
        PACKED_FLOAT32_ARRAY => (),
        PACKED_FLOAT64_ARRAY => (),
        PACKED_STRING_ARRAY => (),
        PACKED_VECTOR2_ARRAY => (),
        PACKED_VECTOR3_ARRAY => (),
        PACKED_COLOR_ARRAY => (),

        #[cfg(since_api = "4.3")]
        PACKED_VECTOR4_ARRAY => (),
        VariantType { ord: MAX.. } => panic!("ord is out of defined range!"),
    }
};

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Explicit impls

// Unit
impl GodotFfiVariant for () {
    fn ffi_to_variant(&self) -> Variant {
        Variant::nil()
    }

    fn ffi_from_variant(variant: &Variant) -> Result<Self, ConvertError> {
        if variant.is_nil() {
            return Ok(());
        }

        Err(FromVariantError::BadType {
            expected: VariantType::NIL,
            actual: variant.get_type(),
        }
        .into_error(variant.clone()))
    }
}

impl GodotType for () {
    type Ffi = ();
    type ToFfi<'a> = ();

    fn to_ffi(&self) -> Self::ToFfi<'_> {}

    fn into_ffi(self) -> Self::Ffi {}

    fn try_from_ffi(_: Self::Ffi) -> Result<Self, ConvertError> {
        Ok(())
    }
}

impl GodotFfiVariant for Variant {
    fn ffi_to_variant(&self) -> Variant {
        self.clone()
    }

    fn ffi_from_variant(variant: &Variant) -> Result<Self, ConvertError> {
        Ok(variant.clone())
    }
}

impl GodotType for Variant {
    type Ffi = Variant;
    type ToFfi<'a> = RefArg<'a, Variant>;

    fn to_ffi(&self) -> Self::ToFfi<'_> {
        RefArg::new(self)
    }

    fn into_ffi(self) -> Self::Ffi {
        self
    }

    fn try_from_ffi(ffi: Self::Ffi) -> Result<Self, ConvertError> {
        Ok(ffi)
    }
}
