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

// The impls only compile if those are different types -- ensures type safety through patch.
#[allow(dead_code)]
trait Distinct {}
impl Distinct for GDExtensionVariantPtr {}
impl Distinct for GDExtensionTypePtr {}
impl Distinct for GDExtensionConstTypePtr {}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Extension traits for conversion

/// Convert a GDExtension pointer type to its uninitialized version.
pub trait SysPtr {
    type Const;
    type Uninit;

    #[allow(clippy::wrong_self_convention)]
    fn as_const(self) -> Self::Const;
    #[allow(clippy::wrong_self_convention)]
    fn as_uninit(self) -> Self::Uninit;

    fn force_mut(const_ptr: Self::Const) -> Self;
    fn force_init(uninit_ptr: Self::Uninit) -> Self;
}

macro_rules! impl_sys_ptr {
    ($Ptr:ty, $Const:ty, $Uninit:ty) => {
        impl SysPtr for $Ptr {
            type Const = $Const;
            type Uninit = $Uninit;

            fn as_const(self) -> Self::Const {
                self as Self::Const
            }

            #[allow(clippy::wrong_self_convention)]
            fn as_uninit(self) -> Self::Uninit {
                self as Self::Uninit
            }

            fn force_mut(const_ptr: Self::Const) -> Self {
                const_ptr as Self
            }

            fn force_init(uninit_ptr: Self::Uninit) -> Self {
                uninit_ptr as Self
            }
        }
    };
}

impl_sys_ptr!(
    GDExtensionStringNamePtr,
    GDExtensionConstStringNamePtr,
    GDExtensionUninitializedStringNamePtr
);
impl_sys_ptr!(
    GDExtensionVariantPtr,
    GDExtensionConstVariantPtr,
    GDExtensionUninitializedVariantPtr
);
impl_sys_ptr!(
    GDExtensionStringPtr,
    GDExtensionConstStringPtr,
    GDExtensionUninitializedStringPtr
);
impl_sys_ptr!(
    GDExtensionObjectPtr,
    GDExtensionConstObjectPtr,
    GDExtensionUninitializedObjectPtr
);
impl_sys_ptr!(
    GDExtensionTypePtr,
    GDExtensionConstTypePtr,
    GDExtensionUninitializedTypePtr
);

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Helper functions

/// Differentiate from `sys::GDEXTENSION_CALL_ERROR_*` codes.
// Note: ASan marks 40 as an invalid enum value in C++. However, it's unclear if there's another way, as Godot doesn't foresee custom error types.
// core/extension/gdextension_interface.cpp:1213:53: runtime error: load of value 40, which is not a valid value for type 'Callable::CallError::Error'
// In practice, it should work because the type holding the error must be at least equivalent to int8_t.
pub const GODOT_RUST_CUSTOM_CALL_ERROR: GDExtensionCallErrorType = 40;

#[doc(hidden)]
#[inline]
pub fn default_call_error() -> GDExtensionCallError {
    GDExtensionCallError {
        error: GDEXTENSION_CALL_OK,
        argument: -1,
        expected: -1,
    }
}

// TODO remove this, in favor of CallError
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
        GODOT_RUST_CUSTOM_CALL_ERROR => "godot-rust function call failed".to_string(),
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
