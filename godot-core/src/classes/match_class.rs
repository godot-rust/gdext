/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

/// Dispatches a class to different subclasses.
///
/// Similar to a `match` statement, but with downcasts. Earlier matches dominate, so keep more-derived classes first.
/// The current implementation checks [`Gd::try_cast()`][crate::obj::Gd::try_cast] linearly with the number of branches.
/// This may change in the future.
///
/// When none of the listed classes match, a _fallback branch_ acts as a catch-all and allows to retrieve the original `Gd` pointer.
/// If the type of the `match_class!` expression is `()`, you can omit the fallback branch. For all other types, it is required, even if all
/// direct subclasses are handled. The reason for this is that there may be other subclasses which are not statically known by godot-rust
/// (e.g. from a script or GDExtension).
///
/// The fallback branch can either be `_` (discard object), or `variable` to access the original object inside the fallback arm.
///
/// # Example
/// ```no_run
/// # use godot::prelude::*;
/// # use godot_core::classes::{InputEvent, InputEventAction};
/// # fn some_input() -> Gd<InputEvent> { unimplemented!() }
/// # // Hack to keep amount of SELECTED_CLASSES limited:
/// # type InputEventMouseButton = InputEventAction;
/// # type InputEventMouseMotion = InputEventAction;
/// // Basic syntax.
/// let event: Gd<InputEvent> = some_input();
///
/// let simple_dispatch: i32 = match_class! { event,
///    button @ InputEventMouseButton => 1,
///    motion @ InputEventMouseMotion => 2,
///    action @ InputEventAction => 3,
///    _ => 0, // Fallback.
/// };
///
/// // More diverse dispatch patterns are also supported.
/// let fancy_dispatch: i32 = match_class! { some_input(),
///     // Mutable bindings supported:
///     mut button @ InputEventMouseButton => 1,
///
///     // Block syntax for multiple statements:
///     motion @ InputEventMouseMotion => {
///         godot_print!("motion");
///         2
///     },
///
///     // Qualified types supported:
///     action @ godot::classes::InputEventAction => 3,
///
///     // Fallback with variable -- retrieves original Gd<InputEvent>.
///     original => 0,
///     // Can also be used with mut:
///     // mut original => 0,
///     // If the match arms have type (), we can also omit the fallback branch.
/// };
///
/// // event_type is now 0, 1, 2, or 3
/// ```
///
/// # Expression and control flow
/// The `match_class!` macro is an expression, as such it has a type. If that type is not `()`, you typically need to use the expression or
/// end it with a semicolon.
///
/// Control-flow statements like `?`, `return`, `continue`, `break` can be used within the match arms.
#[macro_export]
// Note: annoyingly shows full implementation in docs. For workarounds, either move impl to a helper macro, or use something like
// https://crates.io/crates/clean-macro-docs.
// Earlier syntax expected curly braces, i.e.:  ($subject:expr, { $($tt:tt)* }) => {{ ... }};
macro_rules! match_class {
    ($subject:expr, $($tt:tt)*) => {{
        let subject = $subject;
        $crate::match_class_muncher!(subject, $($tt)*)
    }};
}

#[doc(hidden)]
#[macro_export]
macro_rules! match_class_muncher {
    // mut variable @ Class => { ... }.
    ($subject:ident, mut $var:ident @ $Ty:ty => $block:expr, $($rest:tt)*) => {{
        match $subject.try_cast::<$Ty>() {
            Ok(mut $var) => $block,
            Err(__obj) => {
                $crate::match_class_muncher!(__obj, $($rest)*)
            }
        }
    }};

    // variable @ Class => { ... }.
    ($subject:ident, $var:ident @ $Ty:ty => $block:expr, $($rest:tt)*) => {{
        match $subject.try_cast::<$Ty>() {
            Ok($var) => $block,
            Err(__obj) => {
                $crate::match_class_muncher!(__obj, $($rest)*)
            }
        }
    }};

    // mut variable => { ... }.
    ($subject:ident, mut $var:ident => $block:expr $(,)?) => {{
        let mut $var = $subject;
        $block
    }};

    // variable => { ... }.
    ($subject:ident, $var:ident => $block:expr $(,)?) => {{
        let $var = $subject;
        $block
    }};

    // _ => { ... }
    // or nothing, if fallback is absent and overall expression being ().
    ($subject:ident, $(_ => $block:expr $(,)?)?) => {{
        $($block)?
    }};
}
