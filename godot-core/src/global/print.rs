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
            // Godot supports Unicode messages, not only ASCII. See `do_panic` test.

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
/// # See also
/// [`godot_print!`](macro.godot_print.html) and [`godot_error!`](macro.godot_error.html).
///
/// Related to the utility function [`global::push_warning()`](crate::global::push_warning).
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
/// # See also
/// [`godot_print!`](macro.godot_print.html) and [`godot_warn!`](macro.godot_warn.html).
/// For script errors (less relevant in Rust), use [`godot_script_error!`](macro.godot_script_error.html).
///
/// Related to the utility function [`global::push_error()`][crate::global::push_error].
///
/// _Godot equivalent: [`@GlobalScope.push_error()`](https://docs.godotengine.org/en/stable/classes/class_@globalscope.html#class-globalscope-method-push-error)_.
#[macro_export]
macro_rules! godot_error {
    ($fmt:literal $(, $args:expr)* $(,)?) => {
        $crate::inner_godot_msg!(print_error; $fmt $(, $args)*);
    };
}

/// Logs a script error to Godot's built-in debugger and to the OS terminal.
///
/// This is rarely needed in Rust; script errors are typically emitted by the GDScript parser.
///
/// # See also
/// [`godot_error!`](macro.godot_error.html) for a general error message.
///
///
#[macro_export]
macro_rules! godot_script_error {
    ($fmt:literal $(, $args:expr)* $(,)?) => {
        $crate::inner_godot_msg!(print_script_error; $fmt $(, $args)*);
    };
}

/// Prints to the Godot console.
///
/// Automatically appends a newline character at the end of the message.
///
/// Used exactly like standard [`println!`]:
/// ```no_run
/// use godot::global::godot_print;
///
/// let version = 4;
/// godot_print!("Hello, Godot {version}!");
/// ```
///
/// # See also
/// [`godot_print_rich!`](macro.godot_print_rich.html) for a slower alternative that supports BBCode, color and URL tags.
/// To print Godot errors and warnings, use [`godot_error!`](macro.godot_error.html) and [`godot_warn!`](macro.godot_warn.html), respectively.
///
/// This uses the underlying [`global::print()`][crate::global::print] function, which takes a variable-length slice of variants.
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
/// Slower than [`godot_print!`](macro.godot_print_rich.html).
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

/// Concatenates format-style arguments into a `GString`.
///
/// Works similar to Rust's standard [`format!`] macro but returns a Godot `GString`.
///
/// # Example
/// ```no_run
/// use godot::builtin::GString;
/// use godot::global::godot_str;
///
/// let name = "Player";
/// let score = 100;
/// let message: GString = godot_str!("The {name} scored {score} points!");
/// ```
///
/// # See also
/// This macro uses the underlying [`global::str()`][crate::global::str] function, which takes a variable-length slice of variants.
///
/// _Godot equivalent: [`@GlobalScope.str()`](https://docs.godotengine.org/en/stable/classes/class_@globalscope.html#class-globalscope-method-str)_.
#[macro_export]
macro_rules! godot_str {
    ($fmt:literal $(, $args:expr)* $(,)?) => {
        $crate::global::str(&[
            $crate::builtin::Variant::from(
                format!($fmt $(, $args)*)
            )
        ])
    };
}
