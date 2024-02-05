/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! Macros for translation.

pub use crate::{tr, tr_n};

/// A convenience macro for using the [`Object::tr()`](crate::engine::Object::tr()) and [`Object::tr_ex()`](crate::engine::Object::tr_ex())
///  methods.
///
/// Takes a format string literal, with optional arguments. Optionally, `context` for potentially ambiguous words can be
/// added before the format arguments, separated with a `;`.
///
/// Using named or positional parameters instead of `{}` may make it easier to use dynamic formatting once gdext supports it:
/// ```no_run
/// # #[macro_use] extern crate godot;
/// # use godot::builtin::Vector2i;
/// # let a = Vector2i { x: 0, y: 0 };
/// # let b = Vector2i { x: 0, y: 0 };
/// # let context = "context";
/// use godot::engine::translate::tr;
///
/// // Good.
/// tr!(context; "{a} is a {b}"); // inlined, with context
/// tr!("{0} is a {1}", a, b); // positional, without context
/// tr!("{c} is a {d}", c = a.x, d = b.y); // named (inlining not possible here)
///
/// // Not as good, much more fragile.
/// tr!("{} is a {}", a, b);
/// ```
/// The methods are called from the [`Engine`](crate::engine::Engine) singleton.
///
/// See also: [Translation contexts](https://docs.godotengine.org/en/stable/tutorials/i18n/internationalizing_games.html#translation-contexts)
/// in Godot.
#[macro_export]
macro_rules! tr {
    ($fmt:literal $(, $($args:tt)*)?) => {
        $crate::engine::Engine::singleton()
            .tr(format!($fmt $(, $($args)*)?).into())
    };

    ($context:expr; $fmt:literal $(, $($args:tt)*)?) => {
        $crate::engine::Engine::singleton()
            .tr_ex(format!($fmt $(, $($args)*)?).into())
            .context(format!("{}", $context).into())
            .done()
    };
}

/// A convenience macro for using the [`Object::tr_n()`](crate::engine::Object::tr_n()) and
/// [`Object::tr_n_ex()`](crate::engine::Object::tr_n_ex()) methods.
///
/// Allows for the use of format strings with arbitrary arguments. `n` is given prior to the format string, followed by `;`.
/// Optionally, `context` for potentially ambiguous words can be added with `,` after `n` and before `;`.
///
/// Using named or positional parameters instead of `{}` may make it easier to use dynamic formatting once gdext supports it:
/// ```no_run
/// # #[macro_use] extern crate godot;
/// # use godot::builtin::Vector2i;
/// # let a = Vector2i { x: 0, y: 0 };
/// # let b = Vector2i { x: 0, y: 0 };
/// # let context = "context";
/// # let n = 2;
/// use godot::engine::translate::tr_n;
///
/// // Good.
/// tr_n!(n, context; "{a} is a {b}", "{a}s are {b}s"); // inlined, with context
/// tr_n!(n; "{0} is a {1}", "{0}s are {1}s", a, b); // positional, without context
/// tr_n!(n; "{c} is a {d}", "{c}s are {d}s", c = a.x, d = b.y); // named (inlining not possible here)
///
/// // Not as good, much more fragile.
/// // Additionally, such syntax requires that BOTH format strings use ALL listed arguments.
/// tr_n!(n; "{} is a {}", "{}s are {}s", a, b);
/// ```
/// The methods are called from the [`Engine`](crate::engine::Engine) singleton.
///
/// See also: [Translation contexts](https://docs.godotengine.org/en/stable/tutorials/i18n/internationalizing_games.html#translation-contexts)
/// in Godot.
#[macro_export]
macro_rules! tr_n {
    ($n:expr; $singular:literal, $plural:literal $(, $($args:tt)*)?) => {
        $crate::engine::Engine::singleton()
            .tr_n(
                format!($singular$(, $($args)*)?).into(),
                format!($plural$(, $($args)*)?).into(),
                $n,
            )
    };

    ($n:expr, $context:expr; $singular:literal, $plural:literal $(, $($args:tt)*)?) => {
        $crate::engine::Engine::singleton()
            .tr_n_ex(
                format!($singular$(, $($args)*)?).into(),
                format!($plural$(, $($args)*)?).into(),
                $n,
            )
            .context(format!("{}", $context).into())
            .done()
    };
}
