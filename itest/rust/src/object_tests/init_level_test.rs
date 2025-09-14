/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::sync::atomic::{AtomicBool, Ordering};

use godot::init::InitLevel;
use godot::obj::NewAlloc;
use godot::register::{godot_api, GodotClass};
use godot::sys::Global;

use crate::framework::{expect_panic, itest, runs_release, suppress_godot_print};

static OBJECT_CALL_HAS_RUN: AtomicBool = AtomicBool::new(false);
static LEVELS_SEEN: Global<Vec<InitLevel>> = Global::default();

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

// Ensure that the above function has actually run and succeeded.
#[itest]
fn init_level_all_initialized() {
    assert!(
        OBJECT_CALL_HAS_RUN.load(Ordering::Relaxed),
        "Object call function did not run during Core init level"
    );
}

// Ensure that we saw all the init levels expected.
#[itest]
fn init_level_observed_all() {
    let levels_seen = LEVELS_SEEN.lock().clone();

    assert_eq!(levels_seen[0], InitLevel::Core);
    assert_eq!(levels_seen[1], InitLevel::Servers);
    assert_eq!(levels_seen[2], InitLevel::Scene);

    // In Debug/Editor builds, Editor level is loaded; otherwise not.
    let level_3 = levels_seen.get(3);
    if runs_release() {
        assert_eq!(level_3, None);
    } else {
        assert_eq!(level_3, Some(&InitLevel::Editor));
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Level-specific callbacks

pub fn on_level_init(level: InitLevel) {
    LEVELS_SEEN.lock().push(level);

    match level {
        InitLevel::Core => on_init_core(),
        InitLevel::Servers => on_init_servers(),
        InitLevel::Scene => on_init_scene(),
        InitLevel::Editor => on_init_editor(),
    }
}

// Runs during core init level to ensure we can access core singletons.
fn on_init_core() {
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

fn on_init_servers() {
    // Nothing yet.
}

fn on_init_scene() {
    // Known limitation that singletons only become available later:
    // https://github.com/godotengine/godot-cpp/issues/1180#issuecomment-3074351805
    expect_panic("Singletons not loaded during Scene init level", || {
        suppress_godot_print(|| {
            let _ = godot::classes::RenderingServer::singleton();
        });
    });
}

pub fn on_init_editor() {
    // Nothing yet.
}
