/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::builtin::Variant;
use crate::meta::signature;
use crate::meta::{
    FromGodot, GodotConvert, GodotFfiVariant, GodotType, InParamTuple, OutParamTuple, ParamTuple,
    ToGodot,
};
use godot_ffi as sys;
use std::fmt;

macro_rules! impl_param_tuple {
    ($Len:literal; $(($p:ident, $n:tt): $P:ident),+) => {
        impl<$($P),+> ParamTuple for ($($P,)+) where $($P: GodotConvert + fmt::Debug),+ {
            const LEN: usize = $Len;

            fn property_info(index: usize, param_name: &str) -> crate::meta::PropertyInfo {
                match index {
                    $(
                        $n => $P::Via::property_info(param_name),
                    )*
                    _ => unreachable!("property_info: unavailable for index {}", index),
                }
            }

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
                let ($($p,)*) = self;
                format!(
                    // This repeat expression is basically just `"{$p:?}"`, the rest is only needed so that
                    // the repeat separator can be `", "` instead of `,`.
                    concat!("" $(, "{", stringify!($p), ":?}" ,)", "*),
                    $($p=$p),*
                )
            }
        }

        impl<$($P),+> InParamTuple for ($($P,)+) where $($P: FromGodot + fmt::Debug),+ {
            unsafe fn from_varcall_args(
                args_ptr: *const sys::GDExtensionConstVariantPtr,
                call_ctx: &crate::meta::CallContext,
            ) -> signature::CallResult<Self> {
                let args = (
                    $(
                        signature::varcall_arg::<$P, $n>(args_ptr, call_ctx)?,
                    )+
                );

                Ok(args)
            }

            unsafe fn from_ptrcall_args(
                args_ptr: *const sys::GDExtensionConstTypePtr,
                call_type: sys::PtrcallType,
                call_ctx: &crate::meta::CallContext,
            ) -> Self {
                (
                    $(
                        signature::ptrcall_arg::<$P, $n>(args_ptr, call_ctx, call_type),
                    )+
                )
            }

            fn from_variant_array(array: &[&Variant]) -> Self {
                assert_array_length::<Self>(array);
                let mut iter = array.iter();
                (
                    $(
                        <$P>::from_variant(
                            iter.next().unwrap_or_else(|| panic!("ParamTuple: {} access out-of-bounds (len {})", stringify!($p), array.len()))
                    ),
                    )*
                )
            }
        }

        impl<$($P),+> OutParamTuple for ($($P,)+) where $($P: ToGodot + fmt::Debug),+ {
            fn with_args<F, R>(self, f: F) -> R
            where
                F: FnOnce(&[crate::builtin::Variant], &[godot_ffi::GDExtensionConstVariantPtr]) -> R,
            {
                let ffi_args = (
                    $(
                        GodotType::into_ffi(ToGodot::to_godot(&self.$n)),
                    )+
                );

                let variant_args = [
                    $(
                        GodotFfiVariant::ffi_to_variant(&ffi_args.$n),
                    )+
                ];

                let sys_args = [
                    $(
                        Variant::var_sys(&variant_args[$n]),
                    )+
                ];

                f(&variant_args, &sys_args)
            }

            fn with_ptr_args<F, R>(self, f: F) -> R
            where
                F: FnOnce(&[godot_ffi::GDExtensionConstTypePtr]) -> R,
            {
                let ffi_args = (
                    $(
                        GodotType::into_ffi(ToGodot::to_godot(&self.$n)),
                    )+
                );

                let ptr_args = [
                    $(
                        sys::GodotFfi::as_arg_ptr(&ffi_args.$n),
                    )+
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

impl_param_tuple!(1; (p0, 0): P0);
impl_param_tuple!(2; (p0, 0): P0, (p1, 1): P1);
impl_param_tuple!(3; (p0, 0): P0, (p1, 1): P1, (p2, 2): P2);
impl_param_tuple!(4; (p0, 0): P0, (p1, 1): P1, (p2, 2): P2, (p3, 3): P3);
impl_param_tuple!(5; (p0, 0): P0, (p1, 1): P1, (p2, 2): P2, (p3, 3): P3, (p4, 4): P4);
impl_param_tuple!(6; (p0, 0): P0, (p1, 1): P1, (p2, 2): P2, (p3, 3): P3, (p4, 4): P4, (p5, 5): P5);
impl_param_tuple!(7; (p0, 0): P0, (p1, 1): P1, (p2, 2): P2, (p3, 3): P3, (p4, 4): P4, (p5, 5): P5, (p6, 6): P6);
impl_param_tuple!(8; (p0, 0): P0, (p1, 1): P1, (p2, 2): P2, (p3, 3): P3, (p4, 4): P4, (p5, 5): P5, (p6, 6): P6, (p7, 7): P7);
impl_param_tuple!(9; (p0, 0): P0, (p1, 1): P1, (p2, 2): P2, (p3, 3): P3, (p4, 4): P4, (p5, 5): P5, (p6, 6): P6, (p7, 7): P7, (p8, 8): P8);
impl_param_tuple!(10; (p0, 0): P0, (p1, 1): P1, (p2, 2): P2, (p3, 3): P3, (p4, 4): P4, (p5, 5): P5, (p6, 6): P6, (p7, 7): P7, (p8, 8): P8, (p9, 9): P9);
impl_param_tuple!(11; (p0, 0): P0, (p1, 1): P1, (p2, 2): P2, (p3, 3): P3, (p4, 4): P4, (p5, 5): P5, (p6, 6): P6, (p7, 7): P7, (p8, 8): P8, (p9, 9): P9, (p10, 10): P10);
impl_param_tuple!(12; (p0, 0): P0, (p1, 1): P1, (p2, 2): P2, (p3, 3): P3, (p4, 4): P4, (p5, 5): P5, (p6, 6): P6, (p7, 7): P7, (p8, 8): P8, (p9, 9): P9, (p10, 10): P10, (p11, 11): P11);
impl_param_tuple!(13; (p0, 0): P0, (p1, 1): P1, (p2, 2): P2, (p3, 3): P3, (p4, 4): P4, (p5, 5): P5, (p6, 6): P6, (p7, 7): P7, (p8, 8): P8, (p9, 9): P9, (p10, 10): P10, (p11, 11): P11, (p12, 12): P12);
impl_param_tuple!(14; (p0, 0): P0, (p1, 1): P1, (p2, 2): P2, (p3, 3): P3, (p4, 4): P4, (p5, 5): P5, (p6, 6): P6, (p7, 7): P7, (p8, 8): P8, (p9, 9): P9, (p10, 10): P10, (p11, 11): P11, (p12, 12): P12, (p13, 13): P13);

impl ParamTuple for () {
    const LEN: usize = 0;

    fn property_info(_index: usize, _param_name: &str) -> crate::meta::PropertyInfo {
        unreachable!("empty argument list has no parameters")
    }

    fn param_info(
        _index: usize,
        _param_name: &str,
    ) -> Option<crate::registry::method::MethodParamOrReturnInfo> {
        None
    }

    fn format_args(&self) -> String {
        String::new()
    }
}

impl InParamTuple for () {
    unsafe fn from_varcall_args(
        _args_ptr: *const godot_ffi::GDExtensionConstVariantPtr,
        _call_ctx: &crate::meta::CallContext,
    ) -> signature::CallResult<Self> {
        Ok(())
    }

    unsafe fn from_ptrcall_args(
        _args_ptr: *const godot_ffi::GDExtensionConstTypePtr,
        _call_type: godot_ffi::PtrcallType,
        _call_ctx: &crate::meta::CallContext,
    ) -> Self {
        ()
    }

    fn from_variant_array(array: &[&Variant]) -> Self {
        assert_array_length::<()>(array);
        ()
    }
}

impl OutParamTuple for () {
    fn with_args<F, R>(self, f: F) -> R
    where
        F: FnOnce(&[crate::builtin::Variant], &[godot_ffi::GDExtensionConstVariantPtr]) -> R,
    {
        f(&[], &[])
    }

    fn with_ptr_args<F, R>(self, f: F) -> R
    where
        F: FnOnce(&[godot_ffi::GDExtensionConstTypePtr]) -> R,
    {
        f(&[])
    }

    fn to_variant_array(&self) -> Vec<Variant> {
        vec![]
    }
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
