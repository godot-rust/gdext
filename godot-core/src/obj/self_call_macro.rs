/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

/// Calls a single `&mut self`/engine method on `self`'s base, hoisting the arguments before releasing `self`.
///
/// This solves a common borrow-checker trap: `self.base_mut().set_velocity(self.velocity)` fails to compile,
/// because the `base_mut()` temporary mutably borrows `self` while the argument list still reads from it.
/// Combining the two is safe -- the arguments are evaluated to owned values before `self` is released -- but
/// rustc's two-phase borrows don't extend across a user-defined method call to see that. This macro performs
/// the hoist for you:
///
/// ```no_run
/// # use godot::prelude::*;
/// #[derive(GodotClass)]
/// #[class(init, base = Node3D)]
/// struct MyClass {
///     base: Base<Node3D>,
///     velocity: Vector3,
/// }
///
/// impl MyClass {
///     fn f(&mut self) {
///         self_call!(self.set_position(self.velocity));
///     }
/// }
/// ```
///
/// expands roughly to:
///
/// ```ignore
/// {
///     let arg0 = self.velocity;
///     self.base_mut().set_position(arg0)
/// }
/// ```
///
/// # Limitations
/// - The receiver is always `self`; there is no variant for releasing an arbitrary `Gd<T>`, since that case
///   has no aliasing conflict to hoist around (the `Gd<T>` and the arguments are different variables, so
///   plain `gd.bind_mut().method(args)` already works).
/// - Only a single method call is supported, no chaining (e.g. `self.get_node("x").queue_free()`).
/// - Arguments are hoisted by value: pass owned/`Copy` expressions, or clone data you need to keep. Hoisting
///   a reference into a local (e.g. `&self.name`) still borrows `self` across the `base_mut()` call, and will
///   fail to compile for the same reason plain code would -- correctly so, since code that Godot re-enters
///   during the call may mutate the field that reference would still be pointing to.
/// - There is no block form for multiple statements: hoisting only works because it happens once, right
///   before a single release point. A multi-statement version would need every statement's arguments hoisted
///   upfront, which breaks as soon as a later statement's arguments depend on an earlier call's result. Use
///   [`reentrant()`](crate::obj::WithBaseField::reentrant) or [`base_mut()`](crate::obj::WithBaseField::base_mut)
///   directly for that case.
/// - Supports up to 10 arguments, which covers the entire generated Godot API.
#[macro_export]
macro_rules! self_call {
    ($self:tt.$method:ident()) => {
        $crate::obj::WithBaseField::base_mut($self).$method()
    };
    ($self:tt.$method:ident($a0:expr $(,)?)) => {{
        let __a0 = $a0;
        $crate::obj::WithBaseField::base_mut($self).$method(__a0)
    }};
    ($self:tt.$method:ident($a0:expr, $a1:expr $(,)?)) => {{
        let __a0 = $a0;
        let __a1 = $a1;
        $crate::obj::WithBaseField::base_mut($self).$method(__a0, __a1)
    }};
    ($self:tt.$method:ident($a0:expr, $a1:expr, $a2:expr $(,)?)) => {{
        let __a0 = $a0;
        let __a1 = $a1;
        let __a2 = $a2;
        $crate::obj::WithBaseField::base_mut($self).$method(__a0, __a1, __a2)
    }};
    ($self:tt.$method:ident($a0:expr, $a1:expr, $a2:expr, $a3:expr $(,)?)) => {{
        let __a0 = $a0;
        let __a1 = $a1;
        let __a2 = $a2;
        let __a3 = $a3;
        $crate::obj::WithBaseField::base_mut($self).$method(__a0, __a1, __a2, __a3)
    }};
    ($self:tt.$method:ident($a0:expr, $a1:expr, $a2:expr, $a3:expr, $a4:expr $(,)?)) => {{
        let __a0 = $a0;
        let __a1 = $a1;
        let __a2 = $a2;
        let __a3 = $a3;
        let __a4 = $a4;
        $crate::obj::WithBaseField::base_mut($self).$method(__a0, __a1, __a2, __a3, __a4)
    }};
    ($self:tt.$method:ident($a0:expr, $a1:expr, $a2:expr, $a3:expr, $a4:expr, $a5:expr $(,)?)) => {{
        let __a0 = $a0;
        let __a1 = $a1;
        let __a2 = $a2;
        let __a3 = $a3;
        let __a4 = $a4;
        let __a5 = $a5;
        $crate::obj::WithBaseField::base_mut($self).$method(__a0, __a1, __a2, __a3, __a4, __a5)
    }};
    ($self:tt.$method:ident($a0:expr, $a1:expr, $a2:expr, $a3:expr, $a4:expr, $a5:expr, $a6:expr $(,)?)) => {{
        let __a0 = $a0;
        let __a1 = $a1;
        let __a2 = $a2;
        let __a3 = $a3;
        let __a4 = $a4;
        let __a5 = $a5;
        let __a6 = $a6;
        $crate::obj::WithBaseField::base_mut($self)
            .$method(__a0, __a1, __a2, __a3, __a4, __a5, __a6)
    }};
    ($self:tt.$method:ident($a0:expr, $a1:expr, $a2:expr, $a3:expr, $a4:expr, $a5:expr, $a6:expr, $a7:expr $(,)?)) => {{
        let __a0 = $a0;
        let __a1 = $a1;
        let __a2 = $a2;
        let __a3 = $a3;
        let __a4 = $a4;
        let __a5 = $a5;
        let __a6 = $a6;
        let __a7 = $a7;
        $crate::obj::WithBaseField::base_mut($self)
            .$method(__a0, __a1, __a2, __a3, __a4, __a5, __a6, __a7)
    }};
    ($self:tt.$method:ident($a0:expr, $a1:expr, $a2:expr, $a3:expr, $a4:expr, $a5:expr, $a6:expr, $a7:expr, $a8:expr $(,)?)) => {{
        let __a0 = $a0;
        let __a1 = $a1;
        let __a2 = $a2;
        let __a3 = $a3;
        let __a4 = $a4;
        let __a5 = $a5;
        let __a6 = $a6;
        let __a7 = $a7;
        let __a8 = $a8;
        $crate::obj::WithBaseField::base_mut($self)
            .$method(__a0, __a1, __a2, __a3, __a4, __a5, __a6, __a7, __a8)
    }};
    ($self:tt.$method:ident($a0:expr, $a1:expr, $a2:expr, $a3:expr, $a4:expr, $a5:expr, $a6:expr, $a7:expr, $a8:expr, $a9:expr $(,)?)) => {{
        let __a0 = $a0;
        let __a1 = $a1;
        let __a2 = $a2;
        let __a3 = $a3;
        let __a4 = $a4;
        let __a5 = $a5;
        let __a6 = $a6;
        let __a7 = $a7;
        let __a8 = $a8;
        let __a9 = $a9;
        $crate::obj::WithBaseField::base_mut($self)
            .$method(__a0, __a1, __a2, __a3, __a4, __a5, __a6, __a7, __a8, __a9)
    }};
}
