/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

/// Pushes a warning message to Godot's built-in debugger and to the OS terminal.
///
/// _Godot equivalent: @GlobalScope.push_warning()_
#[macro_export]
macro_rules! godot_warn {
    ($fmt:literal $(, $args:expr)* $(,)?) => {
        unsafe {
            let msg = format!("{}\0", format_args!($fmt $(, $args)*));
            assert!(msg.is_ascii(), "godot_warn: message must be ASCII");

            $crate::sys::interface_fn!(print_warning)(
                $crate::sys::c_str_from_str(&msg),
                $crate::sys::c_str(b"<function unset>\0"),
                $crate::sys::c_str_from_str(concat!(file!(), "\0")),
                line!() as i32,
                false as $crate::sys::GDExtensionBool, // whether to create a toast notification in editor
            );
        }
    };
}

/// Pushes an error message to Godot's built-in debugger and to the OS terminal.
///
/// _Godot equivalent: @GlobalScope.push_error()_
#[macro_export]
macro_rules! godot_error {
    // FIXME expr needs to be parenthesised, see usages
    ($fmt:literal $(, $args:expr)* $(,)?) => {
    //($($args:tt),* $(,)?) => {
        unsafe {
            let msg = format!("{}\0", format_args!($fmt $(, $args)*));
            assert!(msg.is_ascii(), "godot_error: message must be ASCII");

            $crate::sys::interface_fn!(print_error)(
                $crate::sys::c_str_from_str(&msg),
                $crate::sys::c_str(b"<function unset>\0"),
                $crate::sys::c_str_from_str(concat!(file!(), "\0")),
                line!() as i32,
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
            assert!(msg.is_ascii(), "godot_script_error: message must be ASCII");

            $crate::sys::interface_fn!(print_script_error)(
                $crate::sys::c_str_from_str(&msg),
                $crate::sys::c_str(b"<function unset>\0"),
                $crate::sys::c_str_from_str(concat!(file!(), "\0")),
                line!() as i32,
                false as $crate::sys::GDExtensionBool, // whether to create a toast notification in editor
            );
        }
    };
}

/// Prints to the Godot console.
///
/// _Godot equivalent: @GlobalScope.print()_
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

/// Prints to the Godot console, used by the godot_print! macro.
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
