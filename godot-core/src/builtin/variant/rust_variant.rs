/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! Pure-Rust view into Variant memory, enabling FFI-free access to scalar types.

use godot_ffi as sys;

use crate::builtin::{Variant, VariantType};

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Layout types (internal)

// Variant data size depends on precision:
// - Single precision (real_t=float): 16 bytes data, 24 bytes total
// - Double precision (real_t=double): 32 bytes data, 40 bytes total
#[cfg(not(feature = "double-precision"))]
pub(crate) const VARIANT_DATA_SIZE: usize = 16;

#[cfg(feature = "double-precision")]
pub(crate) const VARIANT_DATA_SIZE: usize = 32;

// Compile-time size/alignment checks.
const _: () = {
    use std::mem::{align_of, size_of};

    use crate::builtin::{Plane, Quaternion, Rect2, Vector2, Vector3, Vector4};

    // Verify that our size/alignment assumption is correct.
    sys::static_assert_eq_size_align!(RustVariant, sys::types::OpaqueVariant);

    // Verify alignment.
    assert!(align_of::<RustVariant>() == 8);

    // Verify field offsets (tag at 0, data at 8).
    assert!(std::mem::offset_of!(RustVariant, type_tag) == 0);
    assert!(std::mem::offset_of!(RustVariant, data) == 8);

    // Size depends on precision feature.
    // Note: Use `assert!` instead of `assert_eq!` in const contexts (`assert_eq!` is not yet const-compatible).
    #[cfg(not(feature = "double-precision"))]
    {
        assert!(size_of::<RustVariant>() == 24);
    }
    #[cfg(feature = "double-precision")]
    {
        assert!(size_of::<RustVariant>() == 40);
    }

    // Verify precision-dependent types fit in VARIANT_DATA_SIZE.
    assert!(size_of::<Vector2>() <= VARIANT_DATA_SIZE);
    assert!(size_of::<Vector3>() <= VARIANT_DATA_SIZE);
    assert!(size_of::<Vector4>() <= VARIANT_DATA_SIZE);
    assert!(size_of::<Quaternion>() <= VARIANT_DATA_SIZE);
    assert!(size_of::<Plane>() <= VARIANT_DATA_SIZE);
    assert!(size_of::<Rect2>() <= VARIANT_DATA_SIZE);
};

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Public API

/// Marker trait for plain-old-data types that can be marshalled from/to `Variant` in pure Rust.
///
/// This trait enables direct memory marshalling into the Variant's data union, without going through FFI.
/// Types implementing this trait must fit within `VARIANT_DATA_SIZE` and have a memory layout compatible with Godot's expectations.
///
/// # Safety
/// Implementors must:
/// - Fit in `VARIANT_DATA_SIZE` (16 or 32 bytes, verified at macro expansion site).
/// - Have `#[repr(C)]` layout matching Godot's memory representation.
pub unsafe trait RustMarshal: Copy + sys::GodotFfi {}

/// Mutable or immutable view into a [`Variant`], providing safe and unsafe access without FFI calls.
///
/// Enables reading and writing plain-old-data (POD) types directly in Rust memory.
/// Supported types all implement the [`RustMarshal`] trait and fit within the Variant's inline data storage, enabling FFI-free read/write access.
///
/// Enables pure-Rust reading and writing of scalar variant values by directly accessing the variant's memory layout.
/// Const-correct: `view()` returns `&Self`, `view_mut()` returns `&mut Self`.
///
/// # Variant Conversion Paths
///
/// Two conversion paths are available for types implementing [`RustMarshal`]:
///
/// 1. **Optimized path** (default): Uses `RustMarshal` for direct memory access without going through Godot's FFI layer.
///    Accessed via `GodotFfiVariant::rust_to_variant()` and `rust_from_variant()`.
///    Benefits from direct memory access for compatible types and falls back to FFI for types that don't implement `RustMarshal`.
///
/// 2. **FFI-only path**: Always uses Godot's C interface, bypassing `RustMarshal`.
///    Accessed via `GodotFfiVariant::rust_to_variant_ffi()` and `rust_from_variant_ffi()`.
///    Used for testing and verification that both paths produce identical results.
///    Marked with `#[doc(hidden)]` as it's primarily for internal use.
///
/// # Example
/// ```no_run
/// use godot::builtin::{Variant, RustVariant};
///
/// let mut variant = Variant::from(42i64);
/// let mut view = RustVariant::view_mut(&mut variant);
///
/// // Read value.
/// if let Some(val) = view.get_value::<i64>() {
///     println!("Got int: {}", val);
/// }
///
/// // Mutate in place. Fails if variant holds a non-POD type and would require destruction.
/// view.set_value(3.14).unwrap();
/// ```
///
/// # Memory layout
/// From Godot's variant.h:
/// ```cpp
/// Type type = NIL;  // 4 bytes
/// union { ... } _data alignas(8);  // 16+ bytes (32 in double-precision)
/// ```
///
/// # Memory Management and Thread Safety
///
/// Godot's `Variant` uses different strategies depending on the data type.
///
/// **Plain old data types** (POD) like `bool`, `i64`, `f64`, `Vector2/3/4`, `Color`, etc. use value copying, not copy-on-write or reference
/// counting. When you assign or clone a `Variant` containing scalars, the value is copied directly into the new variant, and each variant has
/// independent data. Modifying one variant does not affect clones, and no locking is needed, since each thread gets its own copy.
///
/// **Shared types** like `Object`, `Array`, `Dictionary`, `GString` use reference counting.
/// Data is shared between variants through reference-counted pointers, so modifications may affect other variants referencing the same data.
/// Thread safety measures are required when accessing shared data across threads.
/// These types require destruction and cannot use `RustVariant::set_value()`.
///
/// The key insight: `RustMarshal` types are always POD scalars with value semantics, making them inherently thread-safe for concurrent reads when
/// each thread has its own variant copy.
#[repr(C, align(8))]
pub struct RustVariant {
    type_tag: u32,
    _padding: u32,
    data: [u8; VARIANT_DATA_SIZE],
}

impl RustVariant {
    /// Create an immutable view from a Variant reference.
    pub fn view(variant: &Variant) -> &Self {
        // SAFETY: OpaqueVariant and RustVariant have the same size/alignment (verified at compile time).
        unsafe { std::mem::transmute::<&Variant, &RustVariant>(variant) }
    }

    /// Create a mutable view from a Variant reference.
    pub fn view_mut(variant: &mut Variant) -> &mut Self {
        // SAFETY: OpaqueVariant and RustVariant have the same size/alignment (verified at compile time).
        unsafe { std::mem::transmute::<&mut Variant, &mut RustVariant>(variant) }
    }

    /// Get the raw type tag without FFI.
    ///
    /// Unlike [`Variant::get_type()`], this does not handle the special case of null object pointers.
    #[inline]
    pub fn type_tag(&self) -> sys::GDExtensionVariantType {
        self.type_tag as sys::GDExtensionVariantType
    }

    /// Get the variant type without FFI.
    ///
    /// Unlike [`Variant::get_type()`], this does not handle the special case of null object pointers.
    pub fn get_type(&self) -> VariantType {
        VariantType::from_sys(self.type_tag())
    }

    /// Get a typed value from the variant without going through FFI.
    ///
    /// Returns `None` if the variant's type doesn't match `T`.
    pub fn get_value<T: RustMarshal>(&self) -> Option<T> {
        let expected_type = T::VARIANT_TYPE.variant_as_nil();
        if self.get_type() != expected_type {
            return None;
        }

        // SAFETY: We verified the type matches. Cast bytes to target type.
        let value: T = unsafe { self.data_ptr::<T>().read() };
        Some(value)
    }

    /// Set the variant to a value without going through FFI.
    ///
    /// Returns `Err` if the variant currently holds a type that requires destruction.
    pub fn set_value<T: RustMarshal>(&mut self, value: T) -> Result<(), SetError> {
        self.ensure_pod_type()?;

        // Get type tag from existing GodotFfi trait.
        let variant_type = T::VARIANT_TYPE.variant_as_nil();
        self.type_tag = variant_type.ord as u32;

        // SAFETY: Variant now holds T, which is verified to be POD.
        unsafe { self.data_ptr_mut::<T>().write(value) };

        Ok(())
    }

    /// Write data directly to the variant, assuming the type tag is already correctly set.
    ///
    /// This is faster than [`set_value`][Self::set_value] as it skips type tag writes and checks.
    /// Use when the type tag has been set externally (e.g., by typed array constructors).
    ///
    /// # Safety
    /// The variant's type tag must already be set to `T`'s type.
    /// (Since only `RustMarshal` types are supported, this implies that the variant doesn't hold a type requiring destruction).
    ///
    /// # Panics (strict safeguards)
    /// On safety violation.
    pub unsafe fn set_assuming_type<T: RustMarshal>(&mut self, value: T) {
        sys::strict_assert_eq!(
            self.get_type(),
            T::VARIANT_TYPE.variant_as_nil(),
            "set_assuming_type called with wrong type: expected {:?}, got {:?}",
            T::VARIANT_TYPE.variant_as_nil(),
            self.get_type()
        );

        self.data_ptr_mut::<T>().write(value);
    }

    /// Get a typed pointer to the variant's data for reading.
    ///
    /// While this method is safe, the caller must ensure it's OK to read the data as `T`.
    #[inline]
    fn data_ptr<T>(&self) -> *const T {
        self.data.as_ptr() as *const T
    }

    /// Get a typed mutable pointer to the variant's data for writing.
    ///
    /// While this method is safe, the caller must ensure it's OK to read/write the data as `T`.
    #[inline]
    fn data_ptr_mut<T>(&mut self) -> *mut T {
        self.data.as_mut_ptr() as *mut T
    }

    /// Check if the current variant type is "plain old data" and can be safely overwritten without destruction.
    fn ensure_pod_type(&self) -> Result<(), SetError> {
        let current_type = self.get_type();

        // Scalar types (nil, bool, int, float) don't need destruction.
        if Self::is_pod_type(current_type) {
            Ok(())
        } else {
            Err(SetError { current_type })
        }
    }

    /// Returns true if the type is a scalar or simple Copy type that doesn't require destruction.
    fn is_pod_type(ty: VariantType) -> bool {
        matches!(
            ty,
            VariantType::NIL
                | VariantType::BOOL
                | VariantType::INT
                | VariantType::FLOAT
                | VariantType::VECTOR2
                | VariantType::VECTOR2I
                | VariantType::VECTOR3
                | VariantType::VECTOR3I
                | VariantType::VECTOR4
                | VariantType::VECTOR4I
                | VariantType::QUATERNION
                | VariantType::PLANE
                | VariantType::COLOR
                | VariantType::RECT2
                | VariantType::RECT2I
                | VariantType::RID
        )
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Error types

/// Error returned when trying to overwrite a variant that holds a complex type.
///
/// Complex types (String, Array, Dictionary, etc.) require destruction before
/// being overwritten. Use the standard FFI-based conversion for these types.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetError {
    /// The type currently held by the variant.
    pub current_type: VariantType,
}

impl std::fmt::Display for SetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "cannot overwrite variant of type {:?} without destruction",
            self.current_type
        )
    }
}

impl std::error::Error for SetError {}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// RustMarshal implementations

use crate::builtin::{
    Color, Plane, Quaternion, Rect2, Rect2i, Rid, Vector2, Vector2i, Vector3, Vector3i, Vector4,
    Vector4i,
};

// Following types always fit into Variant data segment (precision-independent):
unsafe impl RustMarshal for bool {}
unsafe impl RustMarshal for i64 {}
unsafe impl RustMarshal for f64 {}
unsafe impl RustMarshal for Vector2i {}
unsafe impl RustMarshal for Vector3i {}
unsafe impl RustMarshal for Vector4i {}
unsafe impl RustMarshal for Color {}
unsafe impl RustMarshal for Rect2i {}
unsafe impl RustMarshal for Rid {}

// Precision-dependent types that fit in both single and double precision modes.
unsafe impl RustMarshal for Vector2 {}
unsafe impl RustMarshal for Vector3 {}
unsafe impl RustMarshal for Vector4 {}
unsafe impl RustMarshal for Quaternion {}
unsafe impl RustMarshal for Plane {}
unsafe impl RustMarshal for Rect2 {}
