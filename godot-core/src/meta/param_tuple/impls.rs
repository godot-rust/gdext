/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

#![deny(unsafe_op_in_unsafe_fn)]
#![allow(unused_attributes)]

use std::fmt;

use godot_ffi as sys;
use sys::GodotFfi;

use crate::builtin::Variant;
use crate::meta::error::{CallError, ConvertError};
use crate::meta::{
    signature, CallContext, FromGodot, GodotConvert, GodotFfiVariant, GodotType, InParamTuple,
    OutParamTuple, ParamTuple, ToGodot,
};

macro_rules! count_idents {
    () => { 0 };
    ($id:ident $($rest:ident)*) => { 1 + count_idents!($($rest)*)};
}

macro_rules! unsafe_impl_param_tuple {
    ($(($p:ident, $n:tt): $P:ident),*) => {
        impl<$($P),*> ParamTuple for ($($P,)*) where $($P: GodotConvert + fmt::Debug),* {
            const LEN: usize = count_idents!($($P)*);

            #[doc(hidden)]
            fn param_info(
                index: usize,
                param_name: &str,
            ) -> Option<crate::registry::method::MethodParamOrReturnInfo> {
                match index {
                    $(
                        $n => Some($P::Via::argument_info(param_name)),
                    )*
                    _ => None,
                }
            }

            fn format_args(&self) -> String {
                format!(
                    // This repeat expression is basically just `"{$n:?}"`, the rest is only needed so that
                    // the repetition separator can be `", "` instead of `,`.
                    concat!("" $(, "{", $n, ":?}",)", "*),
                    $(self.$n),*
                )
            }
        }

        impl<$($P),*> InParamTuple for ($($P,)*) where $($P: FromGodot + fmt::Debug),* {
            unsafe fn from_varcall_args(
                args_ptr: *const sys::GDExtensionConstVariantPtr,
                call_ctx: &crate::meta::CallContext,
            ) -> signature::CallResult<Self> {
                let args = (
                    $(
                        // SAFETY: `args_ptr` is an array with length `Self::LEN` and each element is a valid pointer, since they
                        // are all reborrowable as references.
                        unsafe { *args_ptr.offset($n) },
                    )*
                );

                let param_tuple = (
                    $(
                        // SAFETY: Each pointer in `args_ptr` is reborrowable as a `&Variant` for the duration of this call.
                        unsafe { varcall_arg::<$P>(args.$n, call_ctx, $n)? },
                    )*
                );

                Ok(param_tuple)
            }

            unsafe fn from_ptrcall_args(
                args_ptr: *const sys::GDExtensionConstTypePtr,
                call_type: sys::PtrcallType,
                call_ctx: &crate::meta::CallContext,
            ) -> Self {
                (
                    $(
                        // SAFETY: `args_ptr` has length `Self::LEN` and `$n` is less than `Self::LEN`, and `args_ptr` must be an array whose
                        // `$n`-th element is of type `$P`.
                        unsafe { ptrcall_arg::<$P, $n>(args_ptr, call_ctx, call_type) },
                    )*
                )
            }

            fn from_variant_array(array: &[&Variant]) -> Self {
                assert_array_length::<Self>(array);
                let mut iter = array.iter();
                (
                    $(
                        {
                            let variant = iter.next().unwrap_or_else(
                                || panic!("ParamTuple: {} access out-of-bounds (len {})", stringify!($p), array.len()));

                            variant.to_relaxed_or_panic(
                                || format!("ParamTuple: failed to convert parameter {}", stringify!($p)))
                        },
                    )*
                )
            }
        }

        impl<$($P),*> OutParamTuple for ($($P,)*) where $($P: ToGodot + fmt::Debug),* {
            fn with_variants<F, R>(self, f: F) -> R
            where
                F: FnOnce(&[Variant]) -> R,
            {
                let ffi_args = (
                    $(
                        GodotType::into_ffi(ToGodot::to_godot(&self.$n)),
                    )*
                );

                let variant_args = [
                    $(
                        GodotFfiVariant::ffi_to_variant(&ffi_args.$n),
                    )*
                ];

                f(&variant_args)
            }

            fn with_variant_pointers<F, R>(self, f: F) -> R
            where
                F: FnOnce(&[godot_ffi::GDExtensionConstVariantPtr]) -> R,
            {
                self.with_variants(|variants| {
                    let sys_args = [
                        $(
                            Variant::var_sys(&variants[$n]),
                        )*
                    ];
                    f(&sys_args)
                })
            }

            fn with_type_pointers<F, R>(self, f: F) -> R
            where
                F: FnOnce(&[godot_ffi::GDExtensionConstTypePtr]) -> R,
            {
                let ffi_args = (
                    $(
                        GodotType::into_ffi(ToGodot::to_godot(&self.$n)),
                    )*
                );

                let ptr_args = [
                    $(
                        sys::GodotFfi::as_arg_ptr(&ffi_args.$n),
                    )*
                ];

                f(&ptr_args)
            }

            fn to_variant_array(&self) -> Vec<Variant> {
                let ($($p,)*) = self;

                vec![
                    $( $p.to_variant(), )*
                ]
            }
        }
    };
}

#[allow(unused_variables, unused_mut, clippy::unused_unit)]
mod unit_impl {
    use super::*;
    unsafe_impl_param_tuple!();
}
unsafe_impl_param_tuple!((p0, 0): P0);
unsafe_impl_param_tuple!((p0, 0): P0, (p1, 1): P1);
unsafe_impl_param_tuple!((p0, 0): P0, (p1, 1): P1, (p2, 2): P2);
unsafe_impl_param_tuple!((p0, 0): P0, (p1, 1): P1, (p2, 2): P2, (p3, 3): P3);
unsafe_impl_param_tuple!((p0, 0): P0, (p1, 1): P1, (p2, 2): P2, (p3, 3): P3, (p4, 4): P4);
unsafe_impl_param_tuple!((p0, 0): P0, (p1, 1): P1, (p2, 2): P2, (p3, 3): P3, (p4, 4): P4, (p5, 5): P5);
unsafe_impl_param_tuple!((p0, 0): P0, (p1, 1): P1, (p2, 2): P2, (p3, 3): P3, (p4, 4): P4, (p5, 5): P5, (p6, 6): P6);
unsafe_impl_param_tuple!((p0, 0): P0, (p1, 1): P1, (p2, 2): P2, (p3, 3): P3, (p4, 4): P4, (p5, 5): P5, (p6, 6): P6, (p7, 7): P7);
unsafe_impl_param_tuple!((p0, 0): P0, (p1, 1): P1, (p2, 2): P2, (p3, 3): P3, (p4, 4): P4, (p5, 5): P5, (p6, 6): P6, (p7, 7): P7, (p8, 8): P8);
unsafe_impl_param_tuple!((p0, 0): P0, (p1, 1): P1, (p2, 2): P2, (p3, 3): P3, (p4, 4): P4, (p5, 5): P5, (p6, 6): P6, (p7, 7): P7, (p8, 8): P8, (p9, 9): P9);
unsafe_impl_param_tuple!((p0, 0): P0, (p1, 1): P1, (p2, 2): P2, (p3, 3): P3, (p4, 4): P4, (p5, 5): P5, (p6, 6): P6, (p7, 7): P7, (p8, 8): P8, (p9, 9): P9, (p10, 10): P10);
unsafe_impl_param_tuple!((p0, 0): P0, (p1, 1): P1, (p2, 2): P2, (p3, 3): P3, (p4, 4): P4, (p5, 5): P5, (p6, 6): P6, (p7, 7): P7, (p8, 8): P8, (p9, 9): P9, (p10, 10): P10, (p11, 11): P11);
unsafe_impl_param_tuple!((p0, 0): P0, (p1, 1): P1, (p2, 2): P2, (p3, 3): P3, (p4, 4): P4, (p5, 5): P5, (p6, 6): P6, (p7, 7): P7, (p8, 8): P8, (p9, 9): P9, (p10, 10): P10, (p11, 11): P11, (p12, 12): P12);
unsafe_impl_param_tuple!((p0, 0): P0, (p1, 1): P1, (p2, 2): P2, (p3, 3): P3, (p4, 4): P4, (p5, 5): P5, (p6, 6): P6, (p7, 7): P7, (p8, 8): P8, (p9, 9): P9, (p10, 10): P10, (p11, 11): P11, (p12, 12): P12, (p13, 13): P13);

/// Convert the `N`th argument of `args_ptr` into a value of type `P`.
///
/// # Safety
/// - It must be safe to dereference the address at `args_ptr.offset(N)`.
/// - The pointer at `args_ptr.offset(N)` must follow the safety requirements as laid out in
///   [`GodotFfi::from_arg_ptr`].
pub(super) unsafe fn ptrcall_arg<P: FromGodot, const N: isize>(
    args_ptr: *const sys::GDExtensionConstTypePtr,
    call_ctx: &CallContext,
    call_type: sys::PtrcallType,
) -> P {
    // SAFETY: It is safe to dereference `args_ptr` at `N`.
    let offset_ptr = unsafe { *args_ptr.offset(N) };

    // SAFETY: The pointer follows the safety requirements from `GodotFfi::from_arg_ptr`.
    let ffi = unsafe {
        <P::Via as GodotType>::Ffi::from_arg_ptr(sys::force_mut_ptr(offset_ptr), call_type)
    };

    <P::Via as GodotType>::try_from_ffi(ffi)
        .and_then(P::try_from_godot)
        .unwrap_or_else(|err| param_error::<P>(call_ctx, N as i32, err))
}

/// Converts `arg` into a value of type `P`.
///
/// # Safety
///
/// - It must be safe to reborrow `arg` as a `&Variant` with a lifetime that lasts for the duration of the call.
pub(super) unsafe fn varcall_arg<P: FromGodot>(
    arg: sys::GDExtensionConstVariantPtr,
    call_ctx: &CallContext,
    param_index: isize,
) -> Result<P, CallError> {
    // SAFETY: It is safe to dereference `args_ptr` at `N` as a `Variant`.
    let variant_ref = unsafe { Variant::borrow_var_sys(arg) };

    variant_ref
        .try_to_relaxed::<P>()
        .map_err(|err| CallError::failed_param_conversion::<P>(call_ctx, param_index, err))
}

fn param_error<P>(call_ctx: &CallContext, index: i32, err: ConvertError) -> ! {
    let param_ty = std::any::type_name::<P>();
    panic!("in function `{call_ctx}` at parameter [{index}] of type {param_ty}: {err}");
}

fn assert_array_length<P: ParamTuple>(array: &[&Variant]) {
    assert_eq!(
        array.len(),
        P::LEN,
        "array {array:?} has wrong length, expected {} got {}",
        P::LEN,
        array.len()
    );
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn format_args_test() {
        assert_eq!(&().format_args(), "");
        assert_eq!(&(1, 2, 3).format_args(), "1, 2, 3");
    }

    #[test]
    fn count_idents_test() {
        assert_eq!(2, count_idents!(a b));
        assert_eq!(0, count_idents!());
        assert_eq!(5, count_idents!(a b b a d));
    }
}
