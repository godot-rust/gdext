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
/// Requires a fallback branch, even if all direct known classes are handled. The reason for this is that there may be other subclasses which
/// are not statically known by godot-rust (e.g. from a script or GDExtension). The fallback branch can either be `_` (discard object), or
/// `_(variable)` to access the original object inside the fallback arm.
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
/// let simple_dispatch: i32 = match_class!(event, {
///    InputEventMouseButton(btn) => 1,
///    InputEventMouseMotion(motion) => 2,
///    InputEventAction(action) => 3,
///    _ => 0, // Fallback.
/// });
///
/// // More diverse dispatch patterns are also supported.
/// let fancy_dispatch: i32 = match_class!(some_input(), {
///     InputEventMouseButton(btn) => 1,
///
///     // Block syntax for multiple statements:
///     InputEventMouseMotion(motion) => {
///         godot_print!("motion");
///         2
///     },
///
///     // Qualified types supported:
///     godot::classes::InputEventAction(action) => 3,
///
///     // Fallback with variable -- retrieves original Gd<InputEvent>.
///     // Equivalent to pattern `InputEvent(original)`.
///     _(original) => 0,
/// });
///
/// // event_type is now 0, 1, 2, or 3
/// ```
///
/// # Limitations
/// The expression block is currently wrapped by a closure, so you cannot use control-flow statements like `?`, `return`, `continue`, `break`.
#[macro_export]
// Note: annoyingly shows full implementation in docs. For workarounds, either move impl to a helper macro, or use something like
// https://crates.io/crates/clean-macro-docs.
macro_rules! match_class {
    ($subject:expr, {
        $(
            $($class:ident)::+($var:ident) => $body:expr
        ),+,
        _($fallback_var:ident) => $fallback:expr
        $(,)?
    }) => {
        (|| {
            let mut __evt = $subject;
            $(
                __evt = match __evt.try_cast::<$($class)::*>() {
                    Ok($var) => return $body,
                    Err(e)    => e,
                };
            )+
            let $fallback_var = __evt;
            $fallback
        })()
    };

    ($subject:expr, {
        $(
            $($class:ident)::+($var:ident) => $body:expr
        ),+,
        _ => $fallback:expr
        $(,)?
    }) => {
        (|| {
            let mut __evt = $subject;
            $(
                __evt = match __evt.try_cast::<$($class)::*>() {
                    Ok($var) => return $body,
                    Err(e)    => e,
                };
            )+
            $fallback
        })()
    };
}
