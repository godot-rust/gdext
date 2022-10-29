/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

#[macro_export]
macro_rules! godot_warn {
    ($($args:tt),* $(,)?) => {
        unsafe {
            let msg = format!("{}\0", format_args!($($args,)*));

            $crate::sys::interface_fn!(print_warning)(
                msg.as_bytes().as_ptr() as *const _,
                "<function unset>\0".as_bytes().as_ptr() as *const _,
                concat!(file!(), "\0").as_ptr() as *const _,
                line!() as _,
            );
        }
    };
}

#[macro_export]
macro_rules! godot_error {
    // FIXME expr needs to be parenthesised, see usages
    ($($args:tt),* $(,)?) => {
        unsafe {
            let msg = format!("{}\0", format_args!($($args,)*));

            $crate::sys::interface_fn!(print_error)(
                msg.as_bytes().as_ptr() as *const _,
                "<function unset>\0".as_bytes().as_ptr() as *const _,
                concat!(file!(), "\0").as_ptr() as *const _,
                line!() as _,
            );
        }
    };
}

#[macro_export]
macro_rules! godot_script_error {
    ($($args:tt),* $(,)?) => {
        unsafe {
            let msg = format!("{}\0", format_args!($($args,)*));

            $crate::sys::interface_fn!(print_script_error)(
                msg.as_bytes().as_ptr() as *const _,
                "<function unset>\0".as_bytes().as_ptr() as *const _,
                concat!(file!(), "\0").as_ptr() as *const _,
                line!() as _,
            );
        }
    };
}

pub use crate::{godot_error, godot_script_error, godot_warn};
