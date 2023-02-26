/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

#[macro_export]
macro_rules! godot_warn {
    ($fmt:literal $(, $args:expr)* $(,)?) => {
        unsafe {
            let msg = format!("{}\0", format_args!($fmt $(, $args)*));

            $crate::sys::interface_fn!(print_warning)(
                msg.as_bytes().as_ptr() as *const _,
                "<function unset>\0".as_bytes().as_ptr() as *const _,
                concat!(file!(), "\0").as_ptr() as *const _,
                line!() as _,
                false as $crate::sys::GDExtensionBool, // whether to create a toast notification in editor
            );
        }
    };
}

#[macro_export]
macro_rules! godot_error {
    // FIXME expr needs to be parenthesised, see usages
    ($fmt:literal $(, $args:expr)* $(,)?) => {
    //($($args:tt),* $(,)?) => {
        unsafe {
            let msg = format!("{}\0", format_args!($fmt $(, $args)*));

            $crate::sys::interface_fn!(print_error)(
                msg.as_bytes().as_ptr() as *const _,
                "<function unset>\0".as_bytes().as_ptr() as *const _,
                concat!(file!(), "\0").as_ptr() as *const _,
                line!() as _,
                false as $crate::sys::GDExtensionBool, // whether to create a toast notification in editor
            );
        }
    };
}

#[macro_export]
macro_rules! godot_script_error {
    ($fmt:literal $(, $args:expr)* $(,)?) => {
        unsafe {
            let msg = format!("{}\0", format_args!($fmt $(, $args)*));

            $crate::sys::interface_fn!(print_script_error)(
                msg.as_bytes().as_ptr() as *const _,
                "<function unset>\0".as_bytes().as_ptr() as *const _,
                concat!(file!(), "\0").as_ptr() as *const _,
                line!() as _,
                false as $crate::sys::GDExtensionBool, // whether to create a toast notification in editor
            );
        }
    };
}

#[macro_export]
macro_rules! godot_print {
    ($fmt:literal $(, $args:expr)* $(,)?) => {
        $crate::log::print(&[
            $crate::builtin::Variant::from(
                $crate::builtin::GodotString::from(
                    format!($fmt $(, $args)*)
                )
            )
        ])
    };
}

pub use crate::{godot_error, godot_print, godot_script_error, godot_warn};

use crate::builtin::{StringName, Variant};
use crate::sys::{self, GodotFfi};

pub fn print(varargs: &[Variant]) {
    unsafe {
        let method_name = StringName::from("print");
        let call_fn = sys::interface_fn!(variant_get_ptr_utility_function)(
            method_name.string_sys(),
            2648703342i64,
        );
        let call_fn = call_fn.unwrap_unchecked();

        let mut args = Vec::new();
        args.extend(varargs.iter().map(Variant::sys_const));

        let args_ptr = args.as_ptr();
        let _variant = Variant::from_sys_init_default(|return_ptr| {
            call_fn(return_ptr, args_ptr, args.len() as i32);
        });
    }

    // TODO use generated method, but figure out how print() with zero args can be called
    // crate::engine::utilities::print(head, rest);
}
