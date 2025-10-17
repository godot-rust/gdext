/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

// More tests on native structures are in native_structure_full_codegen_tests.rs.

use godot::builtin::Rid;
use godot::classes::native::{AudioFrame, Glyph, ObjectId};

use crate::framework::itest;

// Simple function to make these up with one field differing.
pub fn sample_glyph(start: i32) -> Glyph {
    Glyph {
        start,
        end: 8,
        count: 9,
        repeat: 10,
        flags: 33,
        x_off: 999.0,
        y_off: -999.0,
        advance: 132.0,
        font_rid: Rid::new(1024),
        font_size: 1025,
        index: 1026,
        span_index: -1,
    }
}

#[itest]
fn native_structure_codegen() {
    // Test construction of a few simple types.
    let _ = AudioFrame {
        left: 0.0,
        right: 0.0,
    };
    let _ = Glyph {
        start: 0,
        end: 0,
        count: 0,
        repeat: 0,
        flags: 0,
        x_off: 0.0,
        y_off: 0.0,
        advance: 0.0,
        font_rid: Rid::new(0),
        font_size: 0,
        index: 0,
        span_index: -1,
    };
}

#[itest]
fn native_structure_partialeq() {
    // Test basic equality between two identically-constructed
    // (but distinct) native structures.
    assert_eq!(sample_glyph(5), sample_glyph(5));
    assert_ne!(sample_glyph(1), sample_glyph(2));
}

#[itest]
fn native_structure_debug() {
    // Test debug output, both pretty-printed and not.
    let object_id = ObjectId { id: 256 };
    assert_eq!(
        format!("{object_id:?}"),
        String::from("ObjectId { id: 256 }")
    );
    assert_eq!(
        format!("{object_id:#?}"),
        String::from("ObjectId {\n    id: 256,\n}")
    );
}
