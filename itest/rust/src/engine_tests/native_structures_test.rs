/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::framework::itest;

use godot::builtin::{Rect2, Rid, Variant};
use godot::classes::native::{
    AudioFrame, CaretInfo, Glyph, ObjectId, PhysicsServer2DExtensionShapeResult,
};
use godot::classes::text_server::Direction;
use godot::classes::{ITextServerExtension, Node3D, RefCounted, TextServer, TextServerExtension};
use godot::obj::{Base, Gd, NewAlloc, NewGd};
use godot::register::{godot_api, GodotClass};

use std::cell::Cell;

#[derive(GodotClass)]
#[class(base=TextServerExtension)]
pub struct TestTextServer {
    glyphs: [Glyph; 2],
    cell: Cell<Option<(Rid, i64)>>,
}

// Simple function to make these up with one field differing.
fn sample_glyph(start: i32) -> Glyph {
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
    }
}

#[godot_api]
impl ITextServerExtension for TestTextServer {
    fn init(_base: Base<TextServerExtension>) -> Self {
        TestTextServer {
            glyphs: [sample_glyph(99), sample_glyph(700)],
            cell: Cell::new(None),
        }
    }

    unsafe fn shaped_text_get_carets(&self, shaped: Rid, position: i64, caret: *mut CaretInfo) {
        // Record the arguments we were called with.
        self.cell.set(Some((shaped, position)));
        // Now put something in the out param.
        *caret = CaretInfo {
            leading_caret: Rect2::from_components(0.0, 0.0, 0.0, 0.0),
            trailing_caret: Rect2::from_components(1.0, 1.0, 1.0, 1.0),
            leading_direction: Direction::AUTO,
            trailing_direction: Direction::LTR,
        };
    }

    fn shaped_text_get_glyph_count(&self, _shaped: Rid) -> i64 {
        self.glyphs.len() as i64
    }

    unsafe fn shaped_text_get_glyphs(&self, _shaped: Rid) -> *const Glyph {
        self.glyphs.as_ptr()
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
    };
}

#[itest]
fn native_structure_out_parameter() {
    // Instantiate a TextServerExtension and then have Godot call a
    // function which uses an 'out' pointer parameter.
    let mut ext = TestTextServer::new_gd();
    let result = ext
        .clone()
        .upcast::<TextServer>()
        .shaped_text_get_carets(Rid::new(100), 200);

    // Check that we called the virtual function.
    let cell = ext.bind_mut().cell.take();
    assert_eq!(cell, Some((Rid::new(100), 200)));

    // Check the result dictionary (Godot made it out of our 'out'
    // param).
    assert_eq!(
        result.get("leading_rect"),
        Some(Variant::from(Rect2::from_components(0.0, 0.0, 0.0, 0.0)))
    );
    assert_eq!(
        result.get("trailing_rect"),
        Some(Variant::from(Rect2::from_components(1.0, 1.0, 1.0, 1.0)))
    );
    assert_eq!(
        result.get("leading_direction"),
        Some(Variant::from(Direction::AUTO))
    );
    assert_eq!(
        result.get("trailing_direction"),
        Some(Variant::from(Direction::LTR))
    );
}

#[itest]
fn native_structure_pointer_to_array_parameter() {
    // Instantiate a TextServerExtension.
    let ext = TestTextServer::new_gd();
    let result = ext
        .clone()
        .upcast::<TextServer>()
        .shaped_text_get_glyphs(Rid::new(100));

    // Check the result array.
    assert_eq!(result.len(), 2);
    assert_eq!(result.at(0).get("start"), Some(Variant::from(99)));
    assert_eq!(result.at(1).get("start"), Some(Variant::from(700)));
}

#[itest]
fn native_structure_clone() {
    // Instantiate CaretInfo directly.
    let caret1 = CaretInfo {
        leading_caret: Rect2::from_components(0.0, 0.0, 0.0, 0.0),
        trailing_caret: Rect2::from_components(1.0, 1.0, 1.0, 1.0),
        leading_direction: Direction::AUTO,
        trailing_direction: Direction::LTR,
    };

    // Clone a new CaretInfo.
    let caret2 = caret1.clone();

    // Test field-wise equality
    // between the original constructor arguments and the clone.
    assert_eq!(
        caret2.leading_caret,
        Rect2::from_components(0.0, 0.0, 0.0, 0.0)
    );
    assert_eq!(
        caret2.trailing_caret,
        Rect2::from_components(1.0, 1.0, 1.0, 1.0)
    );
    assert_eq!(caret2.leading_direction, Direction::AUTO);
    assert_eq!(caret2.trailing_direction, Direction::LTR);
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
        format!("{:?}", object_id),
        String::from("ObjectId { id: 256 }")
    );
    assert_eq!(
        format!("{:#?}", object_id),
        String::from("ObjectId {\n    id: 256,\n}")
    );
}

#[itest]
fn native_structure_object_pointers() {
    // Object.
    let object = Node3D::new_alloc();

    let mut result = PhysicsServer2DExtensionShapeResult {
        rid: Rid::new(12),
        collider_id: ObjectId { id: 0 },
        raw_collider_ptr: std::ptr::null_mut(),
        shape: 0,
    };

    result.set_collider(object.clone().upcast(), true);
    assert_eq!(result.collider_id.id, object.instance_id().to_i64() as u64);

    let retrieved = result.collider();
    assert_eq!(retrieved, Some(object.clone().upcast()));

    object.free();
    assert_eq!(result.collider(), None);

    // RefCounted, increment ref-count.
    let object = RefCounted::new_gd();
    let id = object.instance_id();
    result.set_collider(object.clone().upcast(), true);
    assert_eq!(result.collider_id.id, id.to_i64() as u64);

    drop(object); // Test if Godot keeps ref-count.

    let retrieved = result
        .collider()
        .expect("Ref-counted objects don't drop if ref-count is incremented");
    assert_eq!(retrieved.instance_id(), id);

    // Manually decrement refcount (method unexposed).
    Gd::<RefCounted>::from_instance_id(id).call("unreference".into(), &[]);

    // RefCounted, do NOT increment ref-count.
    let object = RefCounted::new_gd();
    let id = object.instance_id();
    result.set_collider(object.clone().upcast(), false);
    assert_eq!(result.collider_id.id, id.to_i64() as u64);

    drop(object); // Test if Godot keeps ref-count.

    let retrieved = result.collider();
    assert!(
        retrieved.is_none(),
        "Ref-counted objects drop if ref-count is not incremented"
    );
}
