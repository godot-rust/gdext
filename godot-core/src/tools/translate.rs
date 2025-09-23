/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! Macros for translation.

pub use crate::{tr, tr_n};

/// A convenience macro for using the [`Object::tr()`](crate::classes::Object::tr()) and [`Object::tr_ex()`](crate::classes::Object::tr_ex())
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
/// use godot::tools::tr;
///
/// // Good.
/// tr!(context; "{a} is a {b}"); // inlined, with context
/// tr!("{0} is a {1}", a, b); // positional, without context
/// tr!("{c} is a {d}", c = a.x, d = b.y); // named (inlining not possible here)
///
/// // Not as good, much more fragile.
/// tr!("{} is a {}", a, b);
/// ```
/// The methods are called from the [`Engine`](crate::classes::Engine) singleton.
///
/// See also: [Translation contexts](https://docs.godotengine.org/en/stable/tutorials/i18n/internationalizing_games.html#translation-contexts)
/// in Godot.
#[macro_export]
macro_rules! tr {
    ($fmt:literal $(, $($args:tt)*)?) => {{
        let msg = format!($fmt $(, $($args)*)?);

        <$crate::classes::Engine as $crate::obj::Singleton>::singleton().tr(&msg)
    }};

    ($context:expr; $fmt:literal $(, $($args:tt)*)?) => {{
        let msg = format!($fmt $(, $($args)*)?);
        let context = format!("{}", $context);

        <$crate::classes::Engine as $crate::obj::Singleton>::singleton()
            .tr_ex(&msg)
            .context(&context)
            .done()
    }};
}

/// A convenience macro for using the [`Object::tr_n()`](crate::classes::Object::tr_n()) and
/// [`Object::tr_n_ex()`](crate::classes::Object::tr_n_ex()) methods.
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
/// use godot::tools::tr_n;
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
/// The methods are called from the [`Engine`](crate::classes::Engine) singleton.
///
/// See also: [Translation contexts](https://docs.godotengine.org/en/stable/tutorials/i18n/internationalizing_games.html#translation-contexts)
/// in Godot.
#[macro_export]
macro_rules! tr_n {
    ($n:expr; $singular:literal, $plural:literal $(, $($args:tt)*)?) => {
        <$crate::classes::Engine as $crate::obj::Singleton>::singleton()
            .tr_n(
                &format!($singular$(, $($args)*)?),
                &format!($plural$(, $($args)*)?),
                $n,
            )
    };

    ($n:expr, $context:expr; $singular:literal, $plural:literal $(, $($args:tt)*)?) => {
        <$crate::classes::Engine as $crate::obj::Singleton>::singleton()
            .tr_n_ex(
                &format!($singular$(, $($args)*)?),
                &format!($plural$(, $($args)*)?),
                $n,
            )
            .context(&format!("{}", $context))
            .done()
    };
}
