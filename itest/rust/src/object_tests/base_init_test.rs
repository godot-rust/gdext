/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
use std::f32::consts::FRAC_PI_3;

use godot::classes::ClassDb;
use godot::prelude::*;
use godot::task::TaskHandle;

use crate::framework::{expect_panic, itest, next_frame};
use crate::object_tests::base_test::{Based, RefcBased};

#[itest]
fn base_init_propagation() {
    let obj = Gd::<Based>::from_init_fn(|base| {
        // Test both temporary + local-variable syntax.
        base.to_init_gd().set_rotation(FRAC_PI_3);

        let mut gd = base.to_init_gd();
        gd.set_position(Vector2::new(100.0, 200.0));

        Based { base, i: 456 }
    });

    // Check that values are propagated to derived object.
    let guard = obj.bind();
    assert_eq!(guard.i, 456);
    assert_eq!(guard.base().get_rotation(), FRAC_PI_3);
    assert_eq!(guard.base().get_position(), Vector2::new(100.0, 200.0));
    drop(guard);

    obj.free();
}

// This isn't recommended, but test what happens if someone clones and stores the Gd<T>.
#[itest]
fn base_init_extracted_gd() {
    let mut extractor = None;

    let obj = Gd::<Based>::from_init_fn(|base| {
        extractor = Some(base.to_init_gd());

        Based { base, i: 456 }
    });

    let extracted = extractor.expect("extraction failed");
    assert_eq!(extracted.instance_id(), obj.instance_id());
    assert_eq!(extracted, obj.clone().upcast());

    // Destroy through the extracted Gd<T>.
    extracted.free();
    assert!(
        !obj.is_instance_valid(),
        "object should be invalid after base ptr is freed"
    );
}

// Checks bad practice of rug-pulling the base pointer.
#[itest]
fn base_init_freed_gd() {
    let mut free_executed = false;

    expect_panic("base object is destroyed", || {
        let _obj = Gd::<Based>::from_init_fn(|base| {
            let obj = base.to_init_gd();
            obj.free(); // Causes the problem, but doesn't panic yet.
            free_executed = true;

            Based { base, i: 456 }
        });
    });

    assert!(
        free_executed,
        "free() itself doesn't panic, but following construction does"
    );
}

// Verifies that there are no panics, memory leaks or UB in basic case.
#[itest]
fn base_init_refcounted_simple() {
    let _obj = Gd::from_init_fn(|base| {
        drop(base.to_init_gd());

        RefcBased { base }
    });
}

// Tests that the auto-decrement of surplus references also works when instantiated through the engine.
#[itest(async)]
fn base_init_refcounted_from_engine() -> TaskHandle {
    let db = ClassDb::singleton();
    let obj = db.instantiate("RefcBased").to::<Gd<RefcBased>>();

    assert_eq!(obj.get_reference_count(), 2);
    next_frame(move || assert_eq!(obj.get_reference_count(), 1, "eventual dec-ref happens"))
}

#[itest(async)]
fn base_init_refcounted_from_rust() -> TaskHandle {
    let obj = RefcBased::new_gd();

    assert_eq!(obj.get_reference_count(), 2);
    next_frame(move || assert_eq!(obj.get_reference_count(), 1, "eventual dec-ref happens"))
}

#[itest(async)]
fn base_init_refcounted_complex() -> TaskHandle {
    // Instantiate with multiple Gd<T> references.
    let id_simple = verify_complex_init(RefcBased::split_simple());
    let id_intermixed = verify_complex_init(RefcBased::split_intermixed());

    next_frame(move || {
        assert!(!id_simple.lookup_validity(), "object destroyed eventually");
        assert!(
            !id_intermixed.lookup_validity(),
            "object destroyed eventually"
        );
    })
}

fn verify_complex_init((obj, base): (Gd<RefcBased>, Gd<RefCounted>)) -> InstanceId {
    let id = obj.instance_id();

    assert_eq!(obj.instance_id(), base.instance_id());
    assert_eq!(base.get_reference_count(), 3);
    assert_eq!(obj.get_reference_count(), 3);

    drop(base);
    assert_eq!(obj.get_reference_count(), 2);
    assert_eq!(obj.get_reference_count(), 2);
    drop(obj);

    // Not dead yet.
    assert!(id.lookup_validity(), "object retained (dec-ref deferred)");
    id
}

#[cfg(debug_assertions)]
#[itest]
fn base_init_outside_init() {
    let mut obj = Based::new_alloc();

    expect_panic("to_init_gd() outside init() function", || {
        let guard = obj.bind_mut();
        let _gd = guard.base.to_init_gd(); // Panics in Debug builds.
    });

    obj.free();
}

#[cfg(debug_assertions)]
#[itest]
fn base_init_to_gd() {
    expect_panic("WithBaseField::to_gd() inside init() function", || {
        let _obj = Gd::<Based>::from_init_fn(|base| {
            let temp_obj = Based { base, i: 999 };

            // Call to self.to_gd() during initialization should panic in Debug builds.
            let _gd = godot::obj::WithBaseField::to_gd(&temp_obj);

            temp_obj
        });
    });
}
