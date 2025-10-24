/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use godot::builtin::{Rid, Variant};
use godot::classes::{Engine, IObject, RenderingServer};
use godot::init::InitStage;
use godot::obj::{Base, GodotClass, NewAlloc, Singleton};
use godot::register::{godot_api, GodotClass};
use godot::sys::Global;

use crate::framework::{expect_panic, itest, runs_release, suppress_godot_print};

static STAGES_SEEN: Global<Vec<InitStage>> = Global::default();

#[derive(GodotClass)]
#[class(base = Object, init)]
struct SomeObject {}

#[godot_api]
impl SomeObject {
    #[func]
    pub fn method(&self) -> i32 {
        356
    }

    pub fn test() {
        let mut some_object = SomeObject::new_alloc();
        // Need to go through Godot here as otherwise we bypass the failure.
        let result = some_object.call("method", &[]);
        assert_eq!(result, Variant::from(356));

        some_object.free();
    }
}

// Ensure that we saw all the init levels expected.
#[itest]
fn init_level_observed_all() {
    let actual_stages = STAGES_SEEN.lock().clone();

    let mut expected_stages = vec![InitStage::Core, InitStage::Servers, InitStage::Scene];

    // In Debug/Editor builds, Editor level is loaded; otherwise not.
    if !runs_release() {
        expected_stages.push(InitStage::Editor);
    }

    // From Godot 4.5, MainLoop level is added.
    #[cfg(since_api = "4.5")]
    expected_stages.push(InitStage::MainLoop);

    assert_eq!(actual_stages, expected_stages);
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Stage-specific callbacks

pub fn on_stage_init(stage: InitStage) {
    STAGES_SEEN.lock().push(stage);

    match stage {
        InitStage::Core => on_init_core(),
        InitStage::Servers => on_init_servers(),
        InitStage::Scene => on_init_scene(),
        InitStage::Editor => on_init_editor(),
        #[cfg(since_api = "4.5")]
        InitStage::MainLoop => on_init_main_loop(),
        _ => { /* Needed due to #[non_exhaustive] */ }
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
    suppress_godot_print(|| {
        expect_panic("Singletons not loaded during Scene init level", || {
            let _ = godot::classes::RenderingServer::singleton();
        });
    });
}

pub fn on_init_editor() {
    // Nothing yet.
}

#[derive(GodotClass)]
#[class(base=Object)]
struct MainLoopCallbackSingleton {
    tex: Rid,
}

#[godot_api]
impl IObject for MainLoopCallbackSingleton {
    fn init(_: Base<Self::Base>) -> Self {
        Self {
            tex: RenderingServer::singleton().texture_2d_placeholder_create(),
        }
    }
}

#[cfg(since_api = "4.5")]
fn on_init_main_loop() {
    // RenderingServer should be accessible in MainLoop init and deinit.
    let singleton = MainLoopCallbackSingleton::new_alloc();
    assert!(singleton.bind().tex.is_valid());
    Engine::singleton().register_singleton(
        &MainLoopCallbackSingleton::class_id().to_string_name(),
        &singleton,
    );
}

#[cfg(not(since_api = "4.5"))]
fn on_init_main_loop() {
    // Nothing on older API versions.
}

pub fn on_stage_deinit(stage: InitStage) {
    match stage {
        #[cfg(since_api = "4.5")]
        InitStage::MainLoop => on_deinit_main_loop(),
        _ => {
            // Nothing for other stages yet.
        }
    }
}

#[cfg(since_api = "4.5")]
fn on_deinit_main_loop() {
    let singleton = Engine::singleton()
        .get_singleton(&MainLoopCallbackSingleton::class_id().to_string_name())
        .unwrap()
        .cast::<MainLoopCallbackSingleton>();
    Engine::singleton()
        .unregister_singleton(&MainLoopCallbackSingleton::class_id().to_string_name());
    let tex = singleton.bind().tex;
    assert!(tex.is_valid());
    RenderingServer::singleton().free_rid(tex);
    singleton.free();
}

#[cfg(not(since_api = "4.5"))]
fn on_deinit_main_loop() {
    // Nothing on older API versions.
}

#[cfg(since_api = "4.5")]
pub fn on_main_loop_frame() {
    // Nothing yet.
}

#[cfg(not(since_api = "4.5"))]
pub fn on_main_loop_frame() {
    // Nothing on older API versions.
}
