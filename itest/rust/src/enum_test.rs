/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::itest;
use godot::builtin::varray;
use godot::engine::input::CursorShape;
use godot::engine::mesh::PrimitiveType;
use godot::engine::{time, ArrayMesh};
use std::collections::HashSet;

#[itest]
fn enum_ords_correct() {
    use godot::obj::EngineEnum;
    assert_eq!(CursorShape::CURSOR_ARROW.ord(), 0);
    assert_eq!(CursorShape::CURSOR_IBEAM.ord(), 1);
    assert_eq!(CursorShape::CURSOR_POINTING_HAND.ord(), 2);
    assert_eq!(CursorShape::CURSOR_CROSS.ord(), 3);
    assert_eq!(CursorShape::CURSOR_WAIT.ord(), 4);
    assert_eq!(CursorShape::CURSOR_BUSY.ord(), 5);
    assert_eq!(CursorShape::CURSOR_DRAG.ord(), 6);
    assert_eq!(CursorShape::CURSOR_CAN_DROP.ord(), 7);
    assert_eq!(CursorShape::CURSOR_FORBIDDEN.ord(), 8);
    assert_eq!(CursorShape::CURSOR_VSIZE.ord(), 9);
    assert_eq!(CursorShape::CURSOR_HSIZE.ord(), 10);
    assert_eq!(CursorShape::CURSOR_BDIAGSIZE.ord(), 11);
    assert_eq!(CursorShape::CURSOR_FDIAGSIZE.ord(), 12);
    assert_eq!(CursorShape::CURSOR_MOVE.ord(), 13);
    assert_eq!(CursorShape::CURSOR_VSPLIT.ord(), 14);
    assert_eq!(CursorShape::CURSOR_HSPLIT.ord(), 15);
    assert_eq!(CursorShape::CURSOR_HELP.ord(), 16);
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
    months.insert(time::Month::MONTH_JANUARY);
    months.insert(time::Month::MONTH_FEBRUARY);
    months.insert(time::Month::MONTH_MARCH);
    months.insert(time::Month::MONTH_APRIL);
    months.insert(time::Month::MONTH_MAY);
    months.insert(time::Month::MONTH_JUNE);
    months.insert(time::Month::MONTH_JULY);
    months.insert(time::Month::MONTH_AUGUST);
    months.insert(time::Month::MONTH_SEPTEMBER);
    months.insert(time::Month::MONTH_OCTOBER);
    months.insert(time::Month::MONTH_NOVEMBER);
    months.insert(time::Month::MONTH_DECEMBER);

    assert_eq!(months.len(), 12);
}

// Testing https://github.com/godot-rust/gdext/issues/335
// This fails upon calling the function, we dont actually need to make a good call.
#[itest]
fn add_surface_from_arrays() {
    let mut mesh = ArrayMesh::new();
    mesh.add_surface_from_arrays(PrimitiveType::PRIMITIVE_TRIANGLES, varray![]);
}
