/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! Extra functionality to enrich low-level C API.

use crate::gen::gdextension_interface::*;
use crate::VariantType;

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Static checks

// The impls only compile if those are different types -- ensures type safety through patch
trait Distinct {}
impl Distinct for GDExtensionVariantPtr {}
impl Distinct for GDExtensionTypePtr {}
impl Distinct for GDExtensionConstTypePtr {}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Extension traits for conversion

/// Convert a GDExtension pointer type to its uninitialized version.
pub trait AsUninit {
    type Ptr;

    #[allow(clippy::wrong_self_convention)]
    fn as_uninit(self) -> Self::Ptr;

    fn force_init(uninit: Self::Ptr) -> Self;
}

macro_rules! impl_as_uninit {
    ($Ptr:ty, $Uninit:ty) => {
        impl AsUninit for $Ptr {
            type Ptr = $Uninit;

            fn as_uninit(self) -> $Uninit {
                self as $Uninit
            }

            fn force_init(uninit: Self::Ptr) -> Self {
                uninit as Self
            }
        }
    };
}

#[rustfmt::skip]
impl_as_uninit!(GDExtensionStringNamePtr, GDExtensionUninitializedStringNamePtr);
impl_as_uninit!(GDExtensionVariantPtr, GDExtensionUninitializedVariantPtr);
impl_as_uninit!(GDExtensionStringPtr, GDExtensionUninitializedStringPtr);
impl_as_uninit!(GDExtensionObjectPtr, GDExtensionUninitializedObjectPtr);
impl_as_uninit!(GDExtensionTypePtr, GDExtensionUninitializedTypePtr);

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Helper functions

/// Differentiate from `sys::GDEXTENSION_CALL_ERROR_*` codes.
pub const GODOT_RUST_CALL_ERROR: GDExtensionCallErrorType = 40;

#[doc(hidden)]
#[inline]
pub fn default_call_error() -> GDExtensionCallError {
    GDExtensionCallError {
        error: GDEXTENSION_CALL_OK,
        argument: -1,
        expected: -1,
    }
}

#[doc(hidden)]
#[inline]
#[track_caller] // panic message points to call site
pub fn panic_call_error(
    err: &GDExtensionCallError,
    function_name: &str,
    vararg_types: &[VariantType],
) -> ! {
    // This specializes on reflection-style calls, e.g. call(), rpc() etc.
    // In these cases, varargs are the _actual_ arguments, with required args being metadata such as method name.

    debug_assert_ne!(err.error, GDEXTENSION_CALL_OK); // already checked outside

    let GDExtensionCallError {
        error,
        argument,
        expected,
    } = *err;

    let argc = vararg_types.len();
    let reason = match error {
        GDEXTENSION_CALL_ERROR_INVALID_METHOD => "method not found".to_string(),
        GDEXTENSION_CALL_ERROR_INVALID_ARGUMENT => {
            let from = vararg_types[argument as usize];
            let to = VariantType::from_sys(expected as GDExtensionVariantType);
            let i = argument + 1;

            format!("cannot convert argument #{i} from {from:?} to {to:?}")
        }
        GDEXTENSION_CALL_ERROR_TOO_MANY_ARGUMENTS => {
            format!("too many arguments; expected {argument}, but called with {argc}")
        }
        GDEXTENSION_CALL_ERROR_TOO_FEW_ARGUMENTS => {
            format!("too few arguments; expected {argument}, but called with {argc}")
        }
        GDEXTENSION_CALL_ERROR_INSTANCE_IS_NULL => "instance is null".to_string(),
        GDEXTENSION_CALL_ERROR_METHOD_NOT_CONST => "method is not const".to_string(), // not handled in Godot
        GODOT_RUST_CALL_ERROR => "godot-rust function call failed".to_string(),
        _ => format!("unknown reason (error code {error})"),
    };

    // Note: Godot also outputs thread ID
    // In Godot source: variant.cpp:3043 or core_bind.cpp:2742
    panic!("Function call failed:  {function_name} -- {reason}.");
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Lazy method table key types
// Could reuse them in normal load functions, but less code when passing separate parameters -> faster parsing.

#[cfg(feature = "codegen-lazy-fptrs")]
pub mod lazy_keys {
    #[derive(Clone, Eq, PartialEq, Hash)]
    pub struct ClassMethodKey {
        pub class_name: &'static str,
        pub method_name: &'static str,
        pub hash: i64,
    }

    #[derive(Clone, Eq, PartialEq, Hash)]
    pub struct BuiltinMethodKey {
        pub variant_type: crate::VariantType,
        pub variant_type_str: &'static str,
        pub method_name: &'static str,
        pub hash: i64,
    }
}
