/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::sys;

/// Enum representing different errors during script instance calls.
///
/// Provides a type-safe way to handle call errors when implementing [`ScriptInstance::call()`](crate::obj::script::ScriptInstance::call).
/// It maps to the underlying `GDExtensionCallErrorType` constants from Godot's C API.
///
/// Note that the `OK` variant is not included here, as it represents a successful call. This type is meant to be used in
/// `Result<T, CallErrorType>`, where `Ok(T)` indicates success.
///
/// See [`SiMut::base_mut()`][crate::obj::script::SiMut::base_mut] for an example.
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
#[repr(C)]
#[non_exhaustive]
pub enum CallErrorType {
    /// The method is invalid or was not found.
    InvalidMethod = sys::GDEXTENSION_CALL_ERROR_INVALID_METHOD as isize,

    /// One or more arguments cannot be converted to the expected parameter types.
    InvalidArgument = sys::GDEXTENSION_CALL_ERROR_INVALID_ARGUMENT as isize,

    /// Too many arguments were provided to the method.
    TooManyArguments = sys::GDEXTENSION_CALL_ERROR_TOO_MANY_ARGUMENTS as isize,

    /// Too few arguments were provided to the method.
    TooFewArguments = sys::GDEXTENSION_CALL_ERROR_TOO_FEW_ARGUMENTS as isize,

    /// The instance is null.
    InstanceIsNull = sys::GDEXTENSION_CALL_ERROR_INSTANCE_IS_NULL as isize,

    /// The method is not const, but was called on a const instance.
    MethodNotConst = sys::GDEXTENSION_CALL_ERROR_METHOD_NOT_CONST as isize,
}

impl CallErrorType {
    /// Converts the enum variant to its underlying `GDExtensionCallErrorType` value.
    // Used to be result_to_sys(), but not really simplifying things at use site.
    #[doc(hidden)]
    pub const fn to_sys(self) -> sys::GDExtensionCallErrorType {
        self as sys::GDExtensionCallErrorType
    }

    /// Creates a `CallErrorType` from a `GDExtensionCallErrorType` value.
    ///
    /// Returns `None` if the value doesn't correspond to a known error type.
    ///
    /// # Panics (Debug)
    /// If the input doesn't match any known error type.
    #[doc(hidden)]
    pub fn result_from_sys(value: sys::GDExtensionCallErrorType) -> Result<(), Self> {
        match value {
            sys::GDEXTENSION_CALL_OK => Ok(()),
            sys::GDEXTENSION_CALL_ERROR_INVALID_METHOD => Err(Self::InvalidMethod),
            sys::GDEXTENSION_CALL_ERROR_INVALID_ARGUMENT => Err(Self::InvalidArgument),
            sys::GDEXTENSION_CALL_ERROR_TOO_MANY_ARGUMENTS => Err(Self::TooManyArguments),
            sys::GDEXTENSION_CALL_ERROR_TOO_FEW_ARGUMENTS => Err(Self::TooFewArguments),
            sys::GDEXTENSION_CALL_ERROR_INSTANCE_IS_NULL => Err(Self::InstanceIsNull),
            sys::GDEXTENSION_CALL_ERROR_METHOD_NOT_CONST => Err(Self::MethodNotConst),
            _ => {
                sys::strict_assert!(false, "Unknown GDExtensionCallErrorType value: {value}");
                Err(Self::InvalidMethod) // in Release builds, return known error.
            }
        }
    }
}
