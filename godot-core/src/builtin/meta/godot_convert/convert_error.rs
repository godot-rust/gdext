/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::error::Error;
use std::fmt;

use godot_ffi::VariantType;

use crate::builtin::{array_inner, meta::ClassName};

type Cause = Box<dyn Error + Send + Sync>;

/// Represents errors that can occur when converting values from Godot.
#[derive(Debug)]
pub struct ConvertError {
    kind: ErrorKind,
    cause: Option<Cause>,
    value_str: Option<String>,
}

impl ConvertError {
    // Constructors are private (or hidden) as only the library or its proc-macros should construct this type.

    /// Create a new custom error for a conversion.
    fn custom() -> Self {
        Self {
            kind: ErrorKind::Custom,
            cause: None,
            value_str: None,
        }
    }

    /// Create a new custom error for a conversion with the value that failed to convert.
    pub(crate) fn with_kind_value<V>(kind: ErrorKind, value: V) -> Self
    where
        V: fmt::Debug,
    {
        Self {
            kind,
            cause: None,
            value_str: Some(format!("{value:?}")),
        }
    }

    /// Create a new custom error with a rust-error as an underlying cause for the conversion error.
    #[doc(hidden)]
    pub fn with_cause<C>(cause: C) -> Self
    where
        C: Into<Cause>,
    {
        let mut err = Self::custom();
        err.cause = Some(cause.into());
        err
    }

    /// Create a new custom error with a rust-error as an underlying cause for the conversion error, and the
    /// value that failed to convert.
    #[doc(hidden)]
    pub fn with_cause_value<C, V>(cause: C, value: V) -> Self
    where
        C: Into<Cause>,
        V: fmt::Debug,
    {
        let mut err = Self::custom();
        err.cause = Some(cause.into());
        err.value_str = Some(format!("{value:?}"));
        err
    }

    /// Returns the rust-error that caused this error, if one exists.
    pub fn cause(&self) -> Option<&(dyn Error + Send + Sync)> {
        self.cause.as_deref()
    }

    /// Returns a string representation of the value that failed to convert, if one exists.
    pub fn value_str(&self) -> Option<&str> {
        self.value_str.as_deref()
    }

    fn description(&self) -> Option<String> {
        self.kind.description()
    }
}

impl fmt::Display for ConvertError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match (self.description(), self.cause.as_ref()) {
            (Some(desc), Some(cause)) => write!(f, "{desc}: {cause}")?,
            (Some(desc), None) => write!(f, "{desc}")?,
            (None, Some(cause)) => write!(f, "{cause}")?,
            (None, None) => write!(f, "unknown error: {:?}", self.kind)?,
        }

        if let Some(value) = self.value_str.as_ref() {
            write!(f, ": {value}")?;
        }

        Ok(())
    }
}

impl Error for ConvertError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.cause
            .as_ref()
            .map(|cause| &**cause as &(dyn Error + 'static))
    }
}

#[derive(Eq, PartialEq, Debug)]
pub(crate) enum ErrorKind {
    FromGodot(FromGodotError),
    FromFfi(FromFfiError),
    FromVariant(FromVariantError),
    Custom,
}

impl ErrorKind {
    fn description(&self) -> Option<String> {
        match self {
            Self::FromGodot(from_godot) => Some(from_godot.description()),
            Self::FromVariant(from_variant) => Some(from_variant.description()),
            Self::FromFfi(from_ffi) => Some(from_ffi.description()),
            Self::Custom => None,
        }
    }
}

/// Conversion failed during a [`FromGodot`](crate::builtin::meta::FromGodot) call.
#[derive(Eq, PartialEq, Debug)]
pub(crate) enum FromGodotError {
    BadArrayType {
        expected: array_inner::TypeInfo,
        got: array_inner::TypeInfo,
    },
    /// InvalidEnum is also used by bitfields.
    InvalidEnum,
    ZeroInstanceId,
}

impl FromGodotError {
    pub fn into_error<V>(self, value: V) -> ConvertError
    where
        V: fmt::Debug,
    {
        ConvertError::with_kind_value(ErrorKind::FromGodot(self), value)
    }

    fn description(&self) -> String {
        match self {
            Self::BadArrayType { expected, got } => {
                if expected.variant_type() != got.variant_type() {
                    if expected.is_typed() {
                        return format!(
                            "expected array of type {:?}, got array of type {:?}",
                            expected.variant_type(),
                            got.variant_type()
                        );
                    } else {
                        return format!(
                            "expected untyped array, got array of type {:?}",
                            got.variant_type()
                        );
                    }
                }

                assert_ne!(
                    expected.class_name(),
                    got.class_name(),
                    "BadArrayType with expected == got, this is a gdext bug"
                );

                format!(
                    "expected array of class {}, got array of class {}",
                    expected.class_name(),
                    got.class_name()
                )
            }
            Self::InvalidEnum => "invalid engine enum value".into(),
            Self::ZeroInstanceId => "`InstanceId` cannot be 0".into(),
        }
    }
}

/// Conversion failed during a [`GodotType::try_from_ffi()`](crate::builtin::meta::GodotType::try_from_ffi()) call.
#[derive(Eq, PartialEq, Debug)]
#[non_exhaustive]
pub(crate) enum FromFfiError {
    NullRawGd,
    WrongObjectType,
    I32,
    I16,
    I8,
    U32,
    U16,
    U8,
}

impl FromFfiError {
    pub fn into_error<V>(self, value: V) -> ConvertError
    where
        V: fmt::Debug,
    {
        ConvertError::with_kind_value(ErrorKind::FromFfi(self), value)
    }

    fn description(&self) -> String {
        let target = match self {
            Self::NullRawGd => return "`Gd` cannot be null".into(),
            Self::WrongObjectType => return "given object cannot be cast to target type".into(),
            Self::I32 => "i32",
            Self::I16 => "i16",
            Self::I8 => "i8",
            Self::U32 => "u32",
            Self::U16 => "u16",
            Self::U8 => "u8",
        };

        format!("`{target}` cannot store the given value")
    }
}

#[derive(Eq, PartialEq, Debug)]
pub(crate) enum FromVariantError {
    /// Variant type does not match expected type
    BadType {
        expected: VariantType,
        actual: VariantType,
    },

    WrongClass {
        expected: ClassName,
    },
}

impl FromVariantError {
    pub fn into_error<V>(self, value: V) -> ConvertError
    where
        V: fmt::Debug,
    {
        ConvertError::with_kind_value(ErrorKind::FromVariant(self), value)
    }

    fn description(&self) -> String {
        match self {
            Self::BadType { expected, actual } => {
                // Note: wording is the same as in CallError::failed_param_conversion_engine()
                format!("expected type `{expected:?}`, got `{actual:?}`")
            }
            Self::WrongClass { expected } => {
                format!("expected class `{expected}`")
            }
        }
    }
}

fn __ensure_send_sync() {
    fn check<T: Send + Sync>() {}
    check::<ConvertError>();
}
