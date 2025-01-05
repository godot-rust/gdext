/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

// https://geo-ant.github.io/blog/2021/rust-traits-and-variadic-functions
//
// Could be generalized with R return type, and not special-casing `self`. But keep simple until actually needed.

pub trait AsFunc<I, Ps> {
    fn call(&mut self, maybe_instance: I, params: Ps);
}

// pub trait AsMethod<C, Ps> {
//     fn call(&mut self, instance: &mut C, params: Ps);
// }

// Now generalize via macro:
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
