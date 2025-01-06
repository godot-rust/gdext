/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! Emulates variadic argument lists (via tuples), related to functions and signals.

// https://geo-ant.github.io/blog/2021/rust-traits-and-variadic-functions
//
// Could be generalized with R return type, and not special-casing `self`. But keep simple until actually needed.

use crate::builtin::Variant;
use crate::meta;

pub trait AsFunc<I, Ps> {
    fn call(&mut self, maybe_instance: I, params: Ps);
}

macro_rules! impl_signal_recipient {
    ($( $args:ident : $Ps:ident ),*) => {
        // Global and associated functions.
        impl<F, R, $($Ps,)*> AsFunc<(), ( $($Ps,)* )> for F
            where F: FnMut( $($Ps,)* ) -> R
        {
            fn call(&mut self, _no_instance: (), ($($args,)*): ( $($Ps,)* )) {
                self($($args,)*);
            }
        }

        // Methods.
        impl<F, R, C, $($Ps,)*> AsFunc<&mut C, ( $($Ps,)* )> for F
            where F: FnMut( &mut C, $($Ps,)* ) -> R
        {
            fn call(&mut self, instance: &mut C, ($($args,)*): ( $($Ps,)* )) {
                self(instance, $($args,)*);
            }
        }
    };
}

impl_signal_recipient!();
impl_signal_recipient!(arg0: P0);
impl_signal_recipient!(arg0: P0, arg1: P1);
impl_signal_recipient!(arg0: P0, arg1: P1, arg2: P2);

// ----------------------------------------------------------------------------------------------------------------------------------------------

pub trait ParamTuple {
    fn to_variant_array(&self) -> Vec<Variant>;
    fn from_variant_array(array: &[&Variant]) -> Self;
}

macro_rules! impl_param_tuple {
    ($($args:ident : $Ps:ident),*) => {
        impl<$($Ps),*> ParamTuple for ($($Ps,)*)
        where
            $($Ps: meta::ToGodot + meta::FromGodot),*
        {
            fn to_variant_array(&self) -> Vec<Variant> {
                let ($($args,)*) = self;

                vec![
                    $( $args.to_variant(), )*
                ]
            }

            #[allow(unused_variables, unused_mut, clippy::unused_unit)]
            fn from_variant_array(array: &[&Variant]) -> Self {
               let mut iter = array.iter();
               ( $(
                  <$Ps>::from_variant(
                        iter.next().unwrap_or_else(|| panic!("ParamTuple: {} access out-of-bounds (len {})", stringify!($args), array.len()))
                  ),
               )* )
            }
        }
    };
}

impl_param_tuple!();
impl_param_tuple!(arg0: P0);
impl_param_tuple!(arg0: P0, arg1: P1);
impl_param_tuple!(arg0: P0, arg1: P1, arg2: P2);
