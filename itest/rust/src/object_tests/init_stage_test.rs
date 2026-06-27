/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::panic;
use std::sync::Once;

use godot::builtin::{Rid, Variant};
use godot::classes::{Engine, IObject, Node, Object, Os, RenderingServer, Time};
use godot::init::{InitStage, is_class_available, is_singleton_available};
use godot::obj::{Base, GodotClass, NewAlloc, Singleton};
use godot::register::{GodotClass, godot_api};
use godot::sys::{GdextBuild, Global};

use crate::engine_tests::check_classdb_full_api;
use crate::framework::{expect_panic_or_ub, itest, runs_release, suppress_godot_print};

static STAGES_SEEN: Global<Vec<InitStage>> = Global::default();
static STAGES_PANICKED: Global<Vec<InitStage>> = Global::default();

// `tool` because this is internal test scaffolding that exercises class construction during init levels, which itest also runs in editor mode
// (`-e --headless`). Without `tool`, Godot would substitute a placeholder for runtime classes in the editor.
#[derive(GodotClass)]
#[class(base = Object, init, tool)]
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

// Ensure that no init stages panicked.
#[itest]
fn init_level_no_panics() {
    let panicked_stages = STAGES_PANICKED.lock().clone();

    assert!(
        panicked_stages.is_empty(),
        "Init stages panicked: {:?}",
        panicked_stages
    );
}

// Asserts that T's singleton availability matches `present`. When present, also verifies that cached and uncached `singleton()` calls are
// consistent; when absent, verifies `singleton()` panics rather than handing out a dangling/null pointer (regression test, see
// https://github.com/godot-rust/gdext/pull/1638).
fn assert_singleton_present<T: Singleton + GodotClass>(present: bool) {
    use godot::init::__invalidate_singleton_caches;

    // First test the dedicated availability API.
    assert_eq!(is_singleton_available::<T>(), present);

    if present {
        // Must not panic; cached and uncached fetches must resolve to the same live instance.
        __invalidate_singleton_caches();
        let uncached = T::singleton().instance_id();
        let cached = T::singleton().instance_id();
        assert_eq!(uncached, cached);

        __invalidate_singleton_caches();
        let uncached_again = T::singleton().instance_id();
        assert_eq!(cached, uncached_again);
    } else {
        // Probing a missing singleton makes Godot print an error; suppress it when possible. Suppression itself goes through the Engine
        // singleton, which on Godot < 4.4 is not registered before the Scene level -- skip suppression in that window.
        let probe = || {
            expect_panic_or_ub("singleton unavailable", || {
                let _ = T::singleton();
            });
        };

        if is_singleton_available::<Engine>() {
            suppress_godot_print(probe);
        } else {
            probe();
        }
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Stage-specific callbacks

pub fn on_stage_init(stage: InitStage) {
    STAGES_SEEN.lock().push(stage);

    // For every level, check whether ClassDB API is available -- see https://github.com/godot-rust/gdext/pull/1474.
    if GdextBuild::since_api("4.7") || stage >= InitStage::Scene {
        check_classdb_full_api();
    }

    let stage_fn = match stage {
        InitStage::Core => on_init_core as fn(),
        InitStage::Servers => on_init_servers,
        InitStage::Scene => on_init_scene,
        InitStage::Editor => on_init_editor,
        #[cfg(since_api = "4.5")]
        InitStage::MainLoop => on_init_main_loop,
        _ => return, // Needed due to #[non_exhaustive].
    };

    // Catch panics to track which stages fail.
    let result = panic::catch_unwind(panic::AssertUnwindSafe(stage_fn));

    if let Err(panic_payload) = result {
        STAGES_PANICKED.lock().push(stage);
        // Re-panic to preserve original behavior.
        panic::resume_unwind(panic_payload);
    }
}

// Runs during core init level to ensure we can access core singletons.
#[cfg(since_api = "4.4")] // Singletons aren't available in older versions.
fn on_init_core() {
    assert!(is_class_available::<Object>()); // Core
    assert!(!is_class_available::<Node>()); // Scene
    assert!(!is_class_available::<RenderingServer>()); // Servers

    // Core singletons (Engine/Os/Time) are reachable at Core level; RenderingServer is not.
    // Each call also checks cached/uncached consistency (if present) or that access panics (if absent).
    assert_singleton_present::<Engine>(true);
    assert_singleton_present::<Os>(true);
    assert_singleton_present::<Time>(true);
    assert_singleton_present::<RenderingServer>(false);

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

    // Functional: the cached pointers are usable for real method calls, not just identity-stable.
    assert!(Engine::singleton().get_physics_ticks_per_second() > 0);
    assert!(!Os::singleton().get_name().is_empty());
    let time = Time::singleton();
    assert!(time.get_ticks_usec() <= time.get_ticks_usec());
}

#[cfg(before_api = "4.4")]
fn on_init_core() {
    // Engine::singleton() is not available before InitLevel::Scene on Godot < 4.4, so we cannot call
    // Engine::is_editor_hint() here. is_editor_or_unknown() returns None at this point.
    //
    // Engine classes (DeclEngine) are safe: their new_alloc() does not go through is_editor_or_unknown().
    let obj = Object::new_alloc();
    obj.free();

    // User classes (DeclUser) call default_instance() -> is_editor_or_unknown().unwrap_or(false),
    // which returns false when unknown, taking the direct creation path instead of ClassDB.instantiate().
    // This means no placeholder substitution, but that is acceptable at early init before Scene level.
    SomeObject::test();
}

fn on_init_servers() {
    // RenderingServer class becomes available at Servers level, but the singleton instance is
    // only registered later (see comment in on_init_scene).
    assert!(is_class_available::<RenderingServer>());
    assert_singleton_present::<RenderingServer>(false);

    // Scene-level classes still not available.
    assert!(!is_class_available::<Node>());
}

fn on_init_scene() {
    // Scene-level classes are available now.
    assert!(is_class_available::<Node>());

    // Known limitation that singletons only become available later:
    // https://github.com/godotengine/godot-cpp/issues/1180#issuecomment-3074351805
    assert_singleton_present::<RenderingServer>(false);
}

pub fn on_init_editor() {
    // Nothing yet.
}

// `tool` for the same reason as `SomeObject` above: this singleton is registered during MainLoop init and accessed via `bind()` in editor
// mode (see #1404).
#[derive(GodotClass)]
#[class(base=Object, tool)]
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
    // By MainLoop, the RenderingServer singleton is registered. Verify availability + both cache paths.
    assert!(is_class_available::<RenderingServer>());
    assert_singleton_present::<RenderingServer>(true);

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
    // Clear panics at the start of the deinit sequence, so we can collect again during the sequence.
    // First stage can be either MainLoop (>=4.5) or Scene/Editor (<4.5, depending on where itest runs), so easier to use static.
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        STAGES_PANICKED.lock().clear();
    });

    let stage_fn: Option<fn()> = match stage {
        #[cfg(since_api = "4.5")]
        InitStage::MainLoop => Some(on_deinit_main_loop),
        InitStage::Core => Some(on_deinit_core),
        _ => None, // Nothing for other stages yet.
    };

    if let Some(stage_fn) = stage_fn {
        // Catch panics to track which stages fail.
        let result = panic::catch_unwind(panic::AssertUnwindSafe(stage_fn));

        if let Err(_panic_payload) = result {
            STAGES_PANICKED.lock().push(stage);
            // Don't re-panic during deinit - continue to other stages.
        }
    }

    // Core is last deinit stage -- if anything panicked, report and exit immediately (at this point, it's difficult to communicate to Godot).
    if stage == InitStage::Core {
        let panicked_stages = STAGES_PANICKED.lock();
        if !panicked_stages.is_empty() {
            godot::global::godot_error!("godot-rust einit stages panicked: {:?}", *panicked_stages);
            std::process::exit(177);
        }
    }
}

#[cfg(since_api = "4.5")]
fn on_deinit_main_loop() {
    // RenderingServer singleton still available at MainLoop deinit; same level still loaded on the way down, so both cache paths must resolve.
    assert_singleton_present::<RenderingServer>(true);

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

fn on_deinit_core() {
    // At Core deinit, higher levels (Editor/Scene/Servers) have already been unloaded.
    // gdext_on_level_deinit runs *after* user code, so the available level is still Core.
    assert!(is_class_available::<Object>());
    assert!(!is_class_available::<Node>());
    assert!(!is_class_available::<RenderingServer>());

    // These singletons aren't available on those levels in older versions.
    #[cfg(since_api = "4.4")]
    {
        // Core singletons are still reachable at Core deinit.
        assert_singleton_present::<Engine>(true);
        assert_singleton_present::<Os>(true);
        assert_singleton_present::<Time>(true);

        // RenderingServer is already unloaded. Godot internally keeps a dangling map entry; godot-rust must panic instead of dereferencing it.
        assert_singleton_present::<RenderingServer>(false);

        // Functional: the still-loaded singleton is usable for a real method call.
        assert!(Engine::singleton().get_physics_ticks_per_second() > 0);
    }

    // Exit logic happens in on_stage_deinit.
}

#[cfg(since_api = "4.5")]
pub fn on_main_loop_frame() {
    // Nothing yet. Panics here are currently ignored.
}

#[cfg(not(since_api = "4.5"))]
pub fn on_main_loop_frame() {
    // Nothing on older API versions.
}
