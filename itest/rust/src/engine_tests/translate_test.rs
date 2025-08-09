/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use godot::builtin::Vector2;
use godot::tools::{tr, tr_n};

use crate::framework::itest;

#[itest]
fn tr_macro_format() {
    // Make sure expressions are parsed correctly, and use positional label to use argument again.
    let no_context_list_positional = tr!(
        "List: first: {}, second: {}, first again: {0}",
        255,
        "hello!",
    );
    assert_eq!(
        no_context_list_positional.to_string(),
        "List: first: 255, second: hello!, first again: 255"
    );

    // Create a Vector2 to test named field-access formatting.
    let vector2 = Vector2 { x: 1.25, y: 1.5 };

    // Test named (with context).
    let context_named = tr!(
        false; "Named: x: {x}, y: {y}",
        x = vector2.x,
        y = vector2.y,
    );
    assert_eq!(context_named.to_string(), "Named: x: 1.25, y: 1.5");
}

#[itest]
fn tr_n_macro_format_plural() {
    // Test that the first string is chosen and formatted correctly when n == 1.
    let mut n = 1;
    let hello = tr_n!(n; "Hello singular {}!", "Hello plural {}s!", "world");
    assert_eq!(hello.to_string(), "Hello singular world!");

    // Test that the plural string is chosen and formatted correctly with the same invocation when n != 1.
    n = 3;
    let hello = tr_n!(n; "Hello singular {}!", "Hello plural {}s!", "world");
    assert_eq!(hello.to_string(), "Hello plural worlds!");
}
