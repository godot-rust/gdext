/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::sync::atomic::{AtomicBool, Ordering};

use godot::obj::NewAlloc;
use godot::register::{godot_api, GodotClass};

use crate::framework::itest;

static OBJECT_CALL_HAS_RUN: AtomicBool = AtomicBool::new(false);

#[derive(GodotClass)]
#[class(base = Object, init)]
struct SomeObject {}

#[godot_api]
impl SomeObject {
    #[func]
    pub fn set_has_run_true(&self) {
        OBJECT_CALL_HAS_RUN.store(true, Ordering::Release);
    }

    pub fn test() {
        assert!(!OBJECT_CALL_HAS_RUN.load(Ordering::Acquire));
        let mut some_object = SomeObject::new_alloc();
        // Need to go through Godot here as otherwise we bypass the failure.
        some_object.call("set_has_run_true", &[]);
        some_object.free();
    }
}

// Runs during core init level to ensure we can access core singletons.
pub fn test_early_core_singletons() {
    // Ensure we can create and use an Object-derived class during Core init level.
    SomeObject::test();

    // Check the early core singletons we can access here.
    #[cfg(feature = "codegen-full")]
    {
        let project_settings = godot::classes::ProjectSettings::singleton();
        assert_eq!(
            project_settings.get("application/config/name").get_type(),
            godot::builtin::VariantType::STRING
        );
    }

    let engine = godot::classes::Engine::singleton();
    assert!(engine.get_physics_ticks_per_second() > 0);

    let os = godot::classes::Os::singleton();
    assert!(!os.get_name().is_empty());

    let time = godot::classes::Time::singleton();
    assert!(time.get_ticks_usec() <= time.get_ticks_usec());
}

// Runs during scene init level to ensure we can access general singletons in the Scene init call for the extension as a whole.
pub fn test_general_singletons() {
    let mut rendering = godot::classes::RenderingServer::singleton();
    assert!(rendering.get_test_cube() != godot::builtin::Rid::Invalid);
}

// Ensure that the above function actually ran.
#[itest]
fn class_run_during_servers_init() {
    assert!(OBJECT_CALL_HAS_RUN.load(Ordering::Acquire));
}
