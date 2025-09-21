/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::collections::HashSet;

use godot::builtin::varray;
use godot::classes::input::CursorShape;
use godot::classes::mesh::PrimitiveType;
use godot::classes::window::LayoutDirection;
use godot::classes::{time, ArrayMesh};
use godot::global::{Key, Orientation};
use godot::obj::{EngineEnum, NewGd};

use crate::framework::itest;

#[itest]
fn enum_ords() {
    use godot::obj::EngineEnum;
    assert_eq!(CursorShape::ARROW.ord(), 0);
    assert_eq!(CursorShape::IBEAM.ord(), 1);
    assert_eq!(CursorShape::POINTING_HAND.ord(), 2);
    assert_eq!(CursorShape::CROSS.ord(), 3);
    assert_eq!(CursorShape::WAIT.ord(), 4);
    assert_eq!(CursorShape::BUSY.ord(), 5);
    assert_eq!(CursorShape::DRAG.ord(), 6);
    assert_eq!(CursorShape::CAN_DROP.ord(), 7);
    assert_eq!(CursorShape::FORBIDDEN.ord(), 8);
    assert_eq!(CursorShape::VSIZE.ord(), 9);
    assert_eq!(CursorShape::HSIZE.ord(), 10);
    assert_eq!(CursorShape::BDIAGSIZE.ord(), 11);
    assert_eq!(CursorShape::FDIAGSIZE.ord(), 12);
    assert_eq!(CursorShape::MOVE.ord(), 13);
    assert_eq!(CursorShape::VSPLIT.ord(), 14);
    assert_eq!(CursorShape::HSPLIT.ord(), 15);
    assert_eq!(CursorShape::HELP.ord(), 16);
}

#[itest]
fn enum_equality() {
    // TODO: find 2 overlapping ords in same enum

    // assert_eq!(
    //     file_access::CompressionMode::COMPRESSION_DEFLATE,
    //     file_access::CompressionMode::COMPRESSION_DEFLATE
    // );
}

#[itest]
fn enum_hash() {
    let mut months = HashSet::new();
    months.insert(time::Month::JANUARY);
    months.insert(time::Month::FEBRUARY);
    months.insert(time::Month::MARCH);
    months.insert(time::Month::APRIL);
    months.insert(time::Month::MAY);
    months.insert(time::Month::JUNE);
    months.insert(time::Month::JULY);
    months.insert(time::Month::AUGUST);
    months.insert(time::Month::SEPTEMBER);
    months.insert(time::Month::OCTOBER);
    months.insert(time::Month::NOVEMBER);
    months.insert(time::Month::DECEMBER);

    assert_eq!(months.len(), 12, "hash collisions in constants");
}

// Testing https://github.com/godot-rust/gdext/issues/335
// This fails upon calling the function, we don't actually need to make a good call.
#[itest]
fn add_surface_from_arrays() {
    let mut mesh = ArrayMesh::new_gd();
    mesh.add_surface_from_arrays(PrimitiveType::TRIANGLES, &varray![]);
}

#[itest]
fn enum_as_str() {
    use godot::obj::EngineEnum;
    assert_eq!(Orientation::VERTICAL.as_str(), "VERTICAL");
    assert_eq!(Orientation::HORIZONTAL.as_str(), "HORIZONTAL");

    assert_eq!(Key::NONE.as_str(), "NONE");
    assert_eq!(Key::SPECIAL.as_str(), "SPECIAL");
    assert_eq!(Key::ESCAPE.as_str(), "ESCAPE");
    assert_eq!(Key::TAB.as_str(), "TAB");
    assert_eq!(Key::A.as_str(), "A");

    #[cfg(since_api = "4.4")] // Deprecated in Godot, LOCALE is now alias for APPLICATION_LOCALE.
    assert_eq!(LayoutDirection::LOCALE.as_str(), "APPLICATION_LOCALE");
}

#[itest]
fn enum_godot_name() {
    use godot::obj::EngineEnum;
    assert_eq!(
        godot_name(Orientation::VERTICAL),
        Orientation::VERTICAL.as_str()
    );
    assert_eq!(
        godot_name(Orientation::HORIZONTAL),
        Orientation::HORIZONTAL.as_str()
    );

    assert_eq!(godot_name(Key::NONE), "KEY_NONE");
    assert_eq!(godot_name(Key::SPECIAL), "KEY_SPECIAL");
    assert_eq!(godot_name(Key::ESCAPE), "KEY_ESCAPE");
    assert_eq!(godot_name(Key::TAB), "KEY_TAB");
    assert_eq!(godot_name(Key::A), "KEY_A");

    // Unknown enumerators (might come from the future).
    assert_eq!(godot_name(Key::from_ord(1234)), "");
}

fn godot_name<T: EngineEnum + Eq + PartialEq + 'static>(value: T) -> &'static str {
    T::all_constants()
        .iter()
        .find(|c| c.value() == value)
        .map(|c| c.godot_name())
        .unwrap_or("") // Previous behavior.
}
