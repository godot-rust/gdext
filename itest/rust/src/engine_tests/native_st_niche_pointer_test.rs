/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

// Tests for `AudioFrame` and `CaretInfo` require more classes as support, and are only enabled in full codegen mode.
// More tests on native structures are in native_structure_tests.rs.
#![cfg(feature = "codegen-full")]

use std::ptr;

use godot::builtin::{vslice, Rect2, Rid, VarDictionary};
use godot::classes::native::{CaretInfo, Glyph, ObjectId, PhysicsServer2DExtensionShapeResult};
use godot::classes::text_server::Direction;
use godot::classes::{IRefCounted, Node3D, RefCounted};
use godot::meta::RawPtr;
use godot::obj::{Base, NewAlloc, NewGd};
use godot::register::{godot_api, GodotClass};

use super::native_structures_test::sample_glyph;
use crate::framework::itest;

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Tests around #[func] passing, based on CaretInfo and Glyph.

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
    fn pass_native_struct(&self, caret_info: RawPtr<*const CaretInfo>) -> VarDictionary {
        let CaretInfo {
            leading_caret,
            trailing_caret,
            leading_direction,
            trailing_direction,
        } = unsafe { &*caret_info.ptr() };

        let mut result = VarDictionary::new();

        result.set("leading_caret", *leading_caret);
        result.set("trailing_caret", *trailing_caret);
        result.set("leading_direction", *leading_direction);
        result.set("trailing_direction", *trailing_direction);

        result
    }

    #[func]
    fn native_struct_array_ret(&self) -> RawPtr<*const Glyph> {
        unsafe { RawPtr::new(self.glyphs.as_ptr()) }
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

#[itest]
fn native_structure_parameter() {
    let caret = CaretInfo {
        leading_caret: Rect2::from_components(0.0, 0.0, 0.0, 0.0),
        trailing_caret: Rect2::from_components(1.0, 1.0, 1.0, 1.0),
        leading_direction: Direction::AUTO,
        trailing_direction: Direction::LTR,
    };

    let raw_ptr: RawPtr<*const CaretInfo> = unsafe { RawPtr::new(ptr::addr_of!(caret)) };
    let mut object = NativeStructTests::new_gd();
    let result: VarDictionary = object.call("pass_native_struct", vslice![raw_ptr]).to();

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
    let result_ptr: RawPtr<*const Glyph> = object.call("native_struct_array_ret", &[]).to();
    let result = unsafe { std::slice::from_raw_parts(result_ptr.ptr(), 2) };

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
fn native_structure_object_pointers() {
    let mut result = PhysicsServer2DExtensionShapeResult {
        rid: Rid::new(12),
        collider_id: ObjectId { id: 0 },
        // SAFETY: PhysicsServer2DExtensionShapeResult accepts null pointers for raw_collider_ptr.
        raw_collider_ptr: unsafe { RawPtr::null() },
        shape: 0,
    };

    let retrieved = result.get_collider();
    assert_eq!(retrieved, None);

    let object = Node3D::new_alloc();
    unsafe { result.set_collider(object.clone()) };
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
        // SAFETY: PhysicsServer2DExtensionShapeResult accepts null pointers for raw_collider_ptr.
        raw_collider_ptr: unsafe { RawPtr::null() },
        shape: 0,
    };

    // RefCounted, increment ref-count.
    let object = RefCounted::new_gd();
    let id = object.instance_id();
    unsafe { result.set_collider(object.clone()) };
    assert_eq!(result.collider_id.id, id.to_i64() as u64);

    drop(object); // Test if Godot keeps ref-count.

    let retrieved = result
        .get_collider()
        .expect("Ref-counted objects don't drop if ref-count is incremented");
    assert_eq!(retrieved.instance_id(), id);

    // Manually decrement refcount (method unexposed).
    //Gd::<RefCounted>::from_instance_id(id).call("unreference", &[]);

    // RefCounted, do NOT increment ref-count.
    let object = RefCounted::new_gd();
    let id = object.instance_id();
    unsafe { result.set_collider(object.clone()) };
    assert_eq!(result.collider_id.id, id.to_i64() as u64);

    drop(object); // Test if Godot keeps ref-count.

    let retrieved = result.get_collider();
    assert!(
        retrieved.is_none(),
        "Ref-counted objects drop if ref-count is not incremented"
    );
}
