/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::ptr;

use crate::framework::itest;

use godot::builtin::{Dictionary, Rect2, Rid};
use godot::classes::native::{
    AudioFrame, CaretInfo, Glyph, ObjectId, PhysicsServer2DExtensionShapeResult,
};
use godot::classes::text_server::Direction;
use godot::classes::{IRefCounted, Node3D, RefCounted};
use godot::meta::ToGodot;
use godot::obj::{Base, NewAlloc, NewGd};
use godot::register::{godot_api, GodotClass};

#[derive(GodotClass, Debug)]
#[class(base = RefCounted)]
struct NativeStructTests {
    glyphs: [Glyph; 2],
}

#[godot_api]
impl IRefCounted for NativeStructTests {
    fn init(_base: Base<Self::Base>) -> Self {
        Self {
            glyphs: [sample_glyph(99), sample_glyph(700)],
        }
    }
}

#[godot_api]
impl NativeStructTests {
    #[func]
    fn pass_native_struct(&self, caret_info: *const CaretInfo) -> Dictionary {
        let CaretInfo {
            leading_caret,
            trailing_caret,
            leading_direction,
            trailing_direction,
        } = unsafe { &*caret_info };

        let mut result = Dictionary::new();

        result.set("leading_caret", *leading_caret);
        result.set("trailing_caret", *trailing_caret);
        result.set("leading_direction", *leading_direction);
        result.set("trailing_direction", *trailing_direction);

        result
    }

    #[func]
    fn native_struct_array_ret(&self) -> *const Glyph {
        self.glyphs.as_ptr()
    }
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
fn native_structure_parameter() {
    let caret = CaretInfo {
        leading_caret: Rect2::from_components(0.0, 0.0, 0.0, 0.0),
        trailing_caret: Rect2::from_components(1.0, 1.0, 1.0, 1.0),
        leading_direction: Direction::AUTO,
        trailing_direction: Direction::LTR,
    };

    let ptr = ptr::addr_of!(caret);
    let mut object = NativeStructTests::new_gd();
    let result: Dictionary = object
        .call("pass_native_struct".into(), &[ptr.to_variant()])
        .to();

    assert_eq!(
        result.at("leading_caret").to::<Rect2>(),
        caret.leading_caret
    );
    assert_eq!(
        result.at("trailing_caret").to::<Rect2>(),
        caret.trailing_caret
    );
    assert_eq!(
        result.at("leading_direction").to::<Direction>(),
        caret.leading_direction
    );
    assert_eq!(
        result.at("trailing_direction").to::<Direction>(),
        caret.trailing_direction
    );
}

#[itest]
fn native_structure_pointer_to_array_parameter() {
    // Instantiate a custom class.
    let mut object = NativeStructTests::new_gd();
    let result_ptr: *const Glyph = object.call("native_struct_array_ret".into(), &[]).to();
    let result = unsafe { std::slice::from_raw_parts(result_ptr, 2) };

    // Check the result array.
    assert_eq!(result[0].start, 99);
    assert_eq!(result[1].start, 700);
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
    let mut result = PhysicsServer2DExtensionShapeResult {
        rid: Rid::new(12),
        collider_id: ObjectId { id: 0 },
        raw_collider_ptr: std::ptr::null_mut(),
        shape: 0,
    };

    let retrieved = result.get_collider();
    assert_eq!(retrieved, None);

    let object = Node3D::new_alloc();
    result.set_collider(object.clone());
    assert_eq!(result.collider_id.id, object.instance_id().to_i64() as u64);

    let retrieved = result.get_collider();
    assert_eq!(retrieved, Some(object.clone().upcast()));

    object.free();
    assert_eq!(result.get_collider(), None);
}

#[itest(skip)] // Not yet implemented.
fn native_structure_refcounted_pointers() {
    let mut result = PhysicsServer2DExtensionShapeResult {
        rid: Rid::new(12),
        collider_id: ObjectId { id: 0 },
        raw_collider_ptr: std::ptr::null_mut(),
        shape: 0,
    };

    // RefCounted, increment ref-count.
    let object = RefCounted::new_gd();
    let id = object.instance_id();
    result.set_collider(object.clone());
    assert_eq!(result.collider_id.id, id.to_i64() as u64);

    drop(object); // Test if Godot keeps ref-count.

    let retrieved = result
        .get_collider()
        .expect("Ref-counted objects don't drop if ref-count is incremented");
    assert_eq!(retrieved.instance_id(), id);

    // Manually decrement refcount (method unexposed).
    //Gd::<RefCounted>::from_instance_id(id).call("unreference".into(), &[]);

    // RefCounted, do NOT increment ref-count.
    let object = RefCounted::new_gd();
    let id = object.instance_id();
    result.set_collider(object.clone());
    assert_eq!(result.collider_id.id, id.to_i64() as u64);

    drop(object); // Test if Godot keeps ref-count.

    let retrieved = result.get_collider();
    assert!(
        retrieved.is_none(),
        "Ref-counted objects drop if ref-count is not incremented"
    );
}
