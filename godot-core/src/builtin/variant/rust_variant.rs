/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! Pure-Rust view into Variant memory, enabling FFI-free access to scalar types.

use godot_ffi as sys;

use crate::builtin::{Variant, VariantType, Vector3};

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Layout types (internal)

// Variant data size depends on precision:
// - Single precision (real_t=float): 16 bytes data, 24 bytes total
// - Double precision (real_t=double): 32 bytes data, 40 bytes total
#[cfg(not(feature = "double-precision"))]
pub(crate) const VARIANT_DATA_SIZE: usize = 16;

#[cfg(feature = "double-precision")]
pub(crate) const VARIANT_DATA_SIZE: usize = 32;

// Compile-time size/alignment checks for the layout itself. Per-type size checks (`size_of::<T>() <= VARIANT_DATA_SIZE`) live at the
// `impl_ffi_variant!(.., @rust_variant)` expansion site, so they cannot drift from the RustMarshal type set.
const _: () = {
    use std::mem::size_of;

    sys::static_assert_eq_size_align!(RustVariant, sys::types::OpaqueVariant);

    assert!(std::mem::align_of::<RustVariant>() == 8);
    assert!(std::mem::offset_of!(RustVariant, type_tag) == 0);
    assert!(std::mem::offset_of!(RustVariant, data) == 8);

    // Size depends on precision feature.
    // Note: Use `assert!` instead of `assert_eq!` in const contexts (`assert_eq!` is not yet const-compatible).
    #[cfg(not(feature = "double-precision"))]
    assert!(size_of::<RustVariant>() == 24);

    #[cfg(feature = "double-precision")]
    assert!(size_of::<RustVariant>() == 40);
};

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Feature toggle

/// `true` when Rust-side variant marshalling is enabled (default). `false` selects the FFI fallback (`variant-ffi-marshal` feature).
///
/// Use this constant in `if`-expressions instead of `#[cfg]` blocks: the dead branch is eliminated by LLVM, but both branches type-check
/// under either feature setting, keeping the source uniform.
pub(crate) const USE_RUST_MARSHAL: bool = !cfg!(feature = "variant-ffi-marshal");

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Public API

/// Marker trait for plain-old-data types that marshal to/from `Variant`'s data union directly in Rust, without going through FFI.
///
/// The supertrait `GodotType<Ffi = Self>` encodes the core contract: the type is its own FFI carrier (no widening needed), enabling sound
/// in-memory marshalling.
///
/// # Safety
/// Implementors must:
/// - Fit in `VARIANT_DATA_SIZE` (16 or 32 bytes depending on precision; verified at macro expansion site).
/// - Have `#[repr(C)]` layout matching Godot's in-memory representation for the corresponding variant type.
///
/// TODO(v0.6): `Rid` doesn't literally satisfy `#[repr(C)]` (uses niche-optimized enum layout instead, see `rid.rs`). Revisit.
pub unsafe trait RustMarshal:
    crate::meta::GodotType<Ffi = Self> + Copy + sys::GodotFfi
{
    /// The Godot [`VariantType`] for this type.
    const VARIANT_TYPE: VariantType = <Self as sys::GodotFfi>::VARIANT_TYPE.variant_as_nil();
}

/// Mutable or immutable view into a [`Variant`], providing FFI-free read/write access for POD types that implement [`RustMarshal`].
///
/// [`GodotFfiVariant`][crate::meta::GodotFfiVariant] dispatches internally: `RustMarshal` types use direct memory access, all others go
/// through Godot's FFI layer. Internal type.
///
/// # Memory management
/// **POD types** (`bool`, `i64`, `f64`, `Vector2/3/4`, `Color`, etc.) copy by value: each variant holds independent data, has no destructor,
/// and can be overwritten via `set_value()` without leaking.
///
/// **Shared types** (`Object`, `Array`, `Dictionary`, `GString`, etc.) are reference-counted and require destruction, so `set_value()` rejects
/// them -- overwriting would silently leak the resource.
#[repr(C, align(8))]
pub struct RustVariant {
    // Godot stores the variant type as a C++ enum (32-bit signed). We use u32 here because the Rust binding may expose GDExtensionVariantType
    // as either i32 or u32 depending on platform. All valid type ordinals are non-negative (0..=38), so the cast between the two is always safe.
    type_tag: u32,                 // 4 bytes.
    _padding: u32,                 // 4 bytes padding to align the data union to 8 bytes.
    data: [u8; VARIANT_DATA_SIZE], // 16 bytes (32 in double-precision).
}

impl RustVariant {
    /// Create an immutable view from a Variant reference.
    pub fn view(variant: &Variant) -> &Self {
        // SAFETY: OpaqueVariant and RustVariant have the same size/alignment (verified at compile time).
        unsafe { std::mem::transmute::<&Variant, &RustVariant>(variant) }
    }

    /// Create a mutable view from a Variant reference.
    ///
    /// Safe even for ref-counted variants: `&mut Variant` guarantees exclusive access to the handle, and the guarded `set_value()` prevents
    /// overwriting such types without destruction. Gated on `itest` since mutable views (and `set_value`/`SetError`) are only used by `itest/`,
    /// not the production fast path, so they need not widen the internal API surface.
    #[cfg(feature = "itest")]
    pub fn view_mut(variant: &mut Variant) -> &mut Self {
        // SAFETY: OpaqueVariant and RustVariant have the same size/alignment (verified at compile time).
        unsafe { std::mem::transmute::<&mut Variant, &mut RustVariant>(variant) }
    }

    /// Construct a [`Variant`] from a POD value without going through FFI.
    ///
    /// Writes the type tag and data bytes directly into a stack-allocated `RustVariant`, then transmutes it to `Variant`.
    /// `Variant::drop()` calls `variant_destroy`, which is a no-op for POD types (`T: RustMarshal`).
    pub(crate) fn from_pod<T: RustMarshal>(value: T) -> Variant {
        let mut rv = RustVariant {
            // VariantType::ord is i32, ordinals are non-negative -> cast to u32 is safe.
            type_tag: <T as RustMarshal>::VARIANT_TYPE.ord as u32,
            _padding: 0,
            data: [0u8; VARIANT_DATA_SIZE],
        };

        // SAFETY: `RustMarshal` guarantees `T` has `#[repr(C)]` layout matching Godot's in-memory representation and fits in `VARIANT_DATA_SIZE`.
        unsafe { rv.data_ptr_mut::<T>().write(value) };

        // SAFETY: `Variant` is `#[repr(transparent)]` over `OpaqueVariant`; `RustVariant` has identical size and alignment (asserted above).
        unsafe { std::mem::transmute::<RustVariant, Variant>(rv) }
    }

    /// Get the raw type tag without FFI.
    ///
    /// Unlike [`Variant::get_type()`], this does not handle the special case of null object pointers.
    #[inline]
    pub(crate) fn type_tag(&self) -> sys::GDExtensionVariantType {
        self.type_tag as sys::GDExtensionVariantType
    }

    /// Get the variant type without FFI.
    ///
    /// "Unchecked" refers to the fact that, unlike [`Variant::get_type()`], this does not normalize the special case of null object
    /// pointers to `NIL` -- it returns the raw stored type tag. This is not unsafe; the distinction only matters for `OBJECT` variants.
    pub fn get_type_unchecked(&self) -> VariantType {
        VariantType::from_sys(self.type_tag())
    }

    /// Returns `true` if the variant currently holds a value whose `Variant` can be bit-copied and bit-dropped without FFI.
    ///
    /// This is a property of the *variant*, not the payload: large math types like `Transform2D` are `Copy` in Rust but heap-allocated inside
    /// `Variant`, so they return `false`. Delegates to [`VariantType::is_inplace_variant()`]; used by `Variant`'s lifecycle fast paths
    /// (Clone/Drop) and `set_value`.
    #[inline]
    pub(crate) fn has_inplace_type(&self) -> bool {
        self.get_type_unchecked().is_inplace_variant()
    }

    /// Returns `true` if the variant currently holds a value of type `T`.
    #[inline]
    pub(crate) fn is_type<T: RustMarshal>(&self) -> bool {
        self.get_type_unchecked() == <T as RustMarshal>::VARIANT_TYPE
    }

    /// Get a typed value from the variant without going through FFI.
    ///
    /// Returns `None` if the variant's type doesn't match `T`.
    pub fn get_value<T: RustMarshal>(&self) -> Option<T> {
        if !self.is_type::<T>() {
            return None;
        }

        // SAFETY: type was verified to match T via `is_type`.
        let value: T = unsafe { self.data_ptr::<T>().read() };
        Some(value)
    }

    /// Set the variant to a value without going through FFI.
    ///
    /// Returns `Err` if the variant currently holds a shared type (e.g. `Array`, `GString`) that requires FFI destruction.
    /// Overwriting such types without calling their destructor would silently leak the ref-counted resource.
    /// For those types, use the standard `to_variant()` / `from_variant()` API instead.
    #[cfg(feature = "itest")]
    pub fn set_value<T: RustMarshal>(&mut self, value: T) -> Result<(), SetError> {
        if !self.has_inplace_type() {
            return Err(SetError {
                current_type: self.get_type_unchecked(),
            });
        }

        self.type_tag = <T as RustMarshal>::VARIANT_TYPE.ord as u32;

        // SAFETY: current type is POD (checked above), so no destructor is skipped. `type_tag` now matches `T`.
        unsafe { self.data_ptr_mut::<T>().write(value) };

        Ok(())
    }

    /// Returns a raw typed pointer to the variant's data for reading.
    #[inline]
    fn data_ptr<T: RustMarshal>(&self) -> *const T {
        sys::strict_assert!(self.is_type::<T>());
        self.data.as_ptr().cast::<T>()
    }

    /// Returns a raw typed mutable pointer to the variant's data for writing.
    #[inline]
    fn data_ptr_mut<T: RustMarshal>(&mut self) -> *mut T {
        sys::strict_assert!(self.is_type::<T>());
        self.data.as_mut_ptr().cast::<T>()
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Error types

/// Error returned when trying to overwrite a variant that holds a complex type.
///
/// Complex types (`String`, `Array`, `Dictionary`, etc.) require destruction before being overwritten.
/// Use the standard `to_variant()` / `from_variant()` API for those types.
#[cfg(feature = "itest")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetError {
    /// The type currently held by the variant.
    pub current_type: VariantType,
}

#[cfg(feature = "itest")]
impl std::fmt::Display for SetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "cannot overwrite variant of type {:?} without destruction",
            self.current_type
        )
    }
}

#[cfg(feature = "itest")]
impl std::error::Error for SetError {}

// `RustMarshal` is implemented centrally by the `impl_ffi_variant!(.., @rust_variant)` macro in `impls.rs`, keeping the marshalled type set
// in a single place (the macro also enforces the size precondition at its expansion site).

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Startup self-check

/// Checks Godot's actual Variant layout (tag@0, data@8) against what RustMarshal assumes. Builds variants via raw FFI (bypassing
/// `USE_RUST_MARSHAL`) and compares against `RustVariant` reads.
///
/// # Safety
/// Must be called from the main thread, after the Godot interface/binding is initialized.
pub(crate) unsafe fn check_layout_matches_godot() {
    if !USE_RUST_MARSHAL {
        return;
    }

    unsafe {
        check_scalar_layout();
        check_vector_layout();
    }
}

fn panic_layout_mismatch(type_name: &str) -> ! {
    panic!(
        "gdext: Variant layout self-check failed for `{type_name}`; enable Cargo feature `variant-ffi-marshal` and report this bug."
    );
}

unsafe fn check_scalar_layout() {
    const SENTINEL: i64 = 0x1234_5678_9ABC_DEF0;

    let variant = unsafe {
        Variant::new_with_var_uninit(|variant_ptr| {
            let converter = sys::builtin_fn!(int_to_variant);
            converter(
                variant_ptr,
                sys::SysPtr::force_mut(sys::GodotFfi::sys(&SENTINEL)),
            );
        })
    };

    let view = RustVariant::view(&variant);
    let matches =
        view.get_type_unchecked() == VariantType::INT && view.get_value::<i64>() == Some(SENTINEL);

    if !matches {
        panic_layout_mismatch("i64");
    }
}

// Distinct x/y/z sentinel values catch a field-order transposition, not just an offset shift.
unsafe fn check_vector_layout() {
    const SENTINEL: Vector3 = Vector3::new(1.5, 2.5, 3.5);

    let variant = unsafe {
        Variant::new_with_var_uninit(|variant_ptr| {
            let converter = sys::builtin_fn!(vector3_to_variant);
            converter(
                variant_ptr,
                sys::SysPtr::force_mut(sys::GodotFfi::sys(&SENTINEL)),
            );
        })
    };

    let view = RustVariant::view(&variant);
    let matches = view.get_type_unchecked() == VariantType::VECTOR3
        && view.get_value::<Vector3>() == Some(SENTINEL);

    if !matches {
        panic_layout_mismatch("Vector3");
    }
}
