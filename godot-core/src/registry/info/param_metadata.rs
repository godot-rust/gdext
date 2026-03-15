/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use godot_ffi as sys;

/// Metadata for a method parameter or return value, describing the precise numeric type.
///
/// Used in method registration to convey extra type information beyond the basic [`VariantType`][crate::builtin::VariantType]. For example,
/// distinguishing `i8` from `i64` even though both are represented as `INT` in Godot's type system. While irrelevant for GDScript, this can be
/// helpful for other languages. The FFI representation is **not** affected by this, underlying types are always `i64`, `f64` or object pointers.
#[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
#[allow(non_camel_case_types)]
#[non_exhaustive]
pub enum ParamMetadata {
    /// No special metadata; use the default representation.
    #[default]
    NONE,

    /// Rust `i8`.
    INT_IS_INT8,

    /// Rust `i16`.
    INT_IS_INT16,

    /// Rust `i32`.
    INT_IS_INT32,

    /// Rust `i64`.
    INT_IS_INT64,

    /// Rust `u8`.
    INT_IS_UINT8,

    /// Rust `u16`.
    INT_IS_UINT16,

    /// Rust `u32`.
    INT_IS_UINT32,

    /// Rust `u64`.
    INT_IS_UINT64,

    /// 16-bit character (UTF-16).
    INT_IS_CHAR16,

    /// 32-bit character (UTF-32).
    INT_IS_CHAR32,

    /// Rust `f32` single-precision float.
    REAL_IS_FLOAT,

    /// Rust `f64` double-precision float.
    REAL_IS_DOUBLE,

    /// Object that must not be null (non-nullable `Gd<T>` parameter).
    ///
    /// **Compatibility:** Only has an effect in Godot 4.6+. In earlier versions, this behaves like `NONE`, effectively allowing null objects.
    OBJECT_IS_REQUIRED,
}

impl ParamMetadata {
    /// Converts to the raw GDExtension constant.
    pub fn to_sys(self) -> sys::GDExtensionClassMethodArgumentMetadata {
        match self {
            Self::INT_IS_INT8 => sys::GDEXTENSION_METHOD_ARGUMENT_METADATA_INT_IS_INT8,
            Self::INT_IS_INT16 => sys::GDEXTENSION_METHOD_ARGUMENT_METADATA_INT_IS_INT16,
            Self::INT_IS_INT32 => sys::GDEXTENSION_METHOD_ARGUMENT_METADATA_INT_IS_INT32,
            Self::INT_IS_INT64 => sys::GDEXTENSION_METHOD_ARGUMENT_METADATA_INT_IS_INT64,
            Self::INT_IS_UINT8 => sys::GDEXTENSION_METHOD_ARGUMENT_METADATA_INT_IS_UINT8,
            Self::INT_IS_UINT16 => sys::GDEXTENSION_METHOD_ARGUMENT_METADATA_INT_IS_UINT16,
            Self::INT_IS_UINT32 => sys::GDEXTENSION_METHOD_ARGUMENT_METADATA_INT_IS_UINT32,
            Self::INT_IS_UINT64 => sys::GDEXTENSION_METHOD_ARGUMENT_METADATA_INT_IS_UINT64,
            Self::REAL_IS_FLOAT => sys::GDEXTENSION_METHOD_ARGUMENT_METADATA_REAL_IS_FLOAT,
            Self::REAL_IS_DOUBLE => sys::GDEXTENSION_METHOD_ARGUMENT_METADATA_REAL_IS_DOUBLE,

            // Could technically memorize the number and use runtime check with GdextBuild.
            #[cfg(since_api = "4.4")]
            Self::INT_IS_CHAR16 => sys::GDEXTENSION_METHOD_ARGUMENT_METADATA_INT_IS_CHAR16,
            #[cfg(since_api = "4.4")]
            Self::INT_IS_CHAR32 => sys::GDEXTENSION_METHOD_ARGUMENT_METADATA_INT_IS_CHAR32,
            #[cfg(since_api = "4.6")]
            Self::OBJECT_IS_REQUIRED => {
                sys::GDEXTENSION_METHOD_ARGUMENT_METADATA_OBJECT_IS_REQUIRED
            }

            // Covers both NONE and unsupported metadata in older versions.
            _ => sys::GDEXTENSION_METHOD_ARGUMENT_METADATA_NONE,
        }
    }
}
