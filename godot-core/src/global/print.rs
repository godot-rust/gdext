/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! Printing and logging functionality.

// https://stackoverflow.com/a/40234666
#[macro_export]
#[doc(hidden)]
macro_rules! inner_function {
    () => {{
        fn f() {}
        fn type_name_of<T>(_: T) -> &'static str {
            std::any::type_name::<T>()
        }
        let name = type_name_of(f);
        name.strip_suffix("::f").unwrap()
    }};
}

#[macro_export]
#[doc(hidden)]
macro_rules! inner_godot_msg {
    // FIXME expr needs to be parenthesised, see usages
    ($godot_fn:ident; $fmt:literal $(, $args:expr)* $(,)?) => {
    //($($args:tt),* $(,)?) => {
        unsafe {
            let msg = format!("{}\0", format_args!($fmt $(, $args)*));
            // assert!(msg.is_ascii(), "godot_error: message must be ASCII");

            // Check whether engine is loaded, otherwise fall back to stderr.
            if $crate::sys::is_initialized() {
                let function = format!("{}\0", $crate::inner_function!());
                $crate::sys::interface_fn!($godot_fn)(
                    $crate::sys::c_str_from_str(&msg),
                    $crate::sys::c_str_from_str(&function),
                    $crate::sys::c_str_from_str(concat!(file!(), "\0")),
                    line!() as i32,
                    false as $crate::sys::GDExtensionBool, // whether to create a toast notification in editor
                );
            } else {
                eprintln!("[{}] {}", stringify!($godot_fn), &msg[..msg.len() - 1]);
            }
        }
    };
}

/// Pushes a warning message to Godot's built-in debugger and to the OS terminal.
///
/// _Godot equivalent: [`@GlobalScope.push_warning()`](https://docs.godotengine.org/en/stable/classes/class_@globalscope.html#class-globalscope-method-push-warning)_.
#[macro_export]
macro_rules! godot_warn {
    ($fmt:literal $(, $args:expr)* $(,)?) => {
        $crate::inner_godot_msg!(print_warning; $fmt $(, $args)*);
    };
}

/// Pushes an error message to Godot's built-in debugger and to the OS terminal.
///
/// _Godot equivalent: [`@GlobalScope.push_error()`](https://docs.godotengine.org/en/stable/classes/class_@globalscope.html#class-globalscope-method-push-error)_.
#[macro_export]
macro_rules! godot_error {
    ($fmt:literal $(, $args:expr)* $(,)?) => {
        $crate::inner_godot_msg!(print_error; $fmt $(, $args)*);
    };
}

/// Logs a script error to Godot's built-in debugger and to the OS terminal.
#[macro_export]
macro_rules! godot_script_error {
    ($fmt:literal $(, $args:expr)* $(,)?) => {
        $crate::inner_godot_msg!(print_script_error; $fmt $(, $args)*);
    };
}

/// Prints to the Godot console.
///
/// _Godot equivalent: [`@GlobalScope.print()`](https://docs.godotengine.org/en/stable/classes/class_@globalscope.html#class-globalscope-method-print)_.
#[macro_export]
macro_rules! godot_print {
    ($fmt:literal $(, $args:expr)* $(,)?) => {
        $crate::global::print(&[
            $crate::builtin::Variant::from(
                format!($fmt $(, $args)*)
            )
        ])
    };
}

/// Prints to the Godot console. Supports BBCode, color and URL tags.
///
/// Slower than [`godot_print!`].
///
/// _Godot equivalent: [`@GlobalScope.print_rich()`](https://docs.godotengine.org/en/stable/classes/class_@globalscope.html#class-globalscope-method-print-rich)_.
#[macro_export]
macro_rules! godot_print_rich {
    ($fmt:literal $(, $args:expr)* $(,)?) => {
        $crate::global::print_rich(&[
            $crate::builtin::Variant::from(
                format!($fmt $(, $args)*)
            )
        ])
    };
}
