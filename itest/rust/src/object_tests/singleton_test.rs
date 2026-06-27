/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use godot::builtin::GString;
use godot::classes::{Engine, Os, Time};
use godot::obj::{Gd, Singleton};
use godot::register::{GodotClass, godot_api};

use crate::framework::itest;

#[itest]
fn singleton_from_instance_id() {
    let a: Gd<Os> = Os::singleton();
    let id = a.instance_id();

    let b: Gd<Os> = Gd::from_instance_id(id);

    assert_eq!(a.get_executable_path(), b.get_executable_path());
}

#[itest]
fn singleton_is_operational() {
    let os: Gd<Os> = Os::singleton();
    let key = GString::from("MY_TEST_ENV");
    let value = GString::from("SOME_VALUE");

    // set_environment is const, for some reason
    os.set_environment(&key, &value);

    let read_value = os.get_environment(&key);
    assert_eq!(read_value, value);
}

#[itest]
fn user_singleton() {
    // Must be registered with the library and accessible at this point.
    let value = SomeUserSingleton::singleton().bind().some_method();
    assert_eq!(value, 42);
}

// Cached singleton pointers must return the same live instance, never stale or wrong. Same shape as `user_singleton_caching`:
// cached fetches resolve to the same live instance, also after a cache reset.
#[itest]
fn engine_singleton_caching() {
    let engine_id = Engine::singleton().instance_id();
    let os_id = Os::singleton().instance_id();
    let time_id = Time::singleton().instance_id();
    assert_eq!(Engine::singleton().instance_id(), engine_id);
    assert_eq!(Os::singleton().instance_id(), os_id);
    assert_eq!(Time::singleton().instance_id(), time_id);

    // Distinct singletons must resolve to distinct instances (cache must not mix up types).
    assert_ne!(engine_id, os_id);
    assert_ne!(engine_id, time_id);
    assert_ne!(os_id, time_id);

    godot::init::__invalidate_singleton_caches();
    assert_eq!(Engine::singleton().instance_id(), engine_id);
    assert_eq!(Os::singleton().instance_id(), os_id);
    assert_eq!(Time::singleton().instance_id(), time_id);

    // Functional sanity checks on the cached instances.
    assert!(Engine::singleton().get_physics_ticks_per_second() > 0);
    assert!(!Os::singleton().get_name().is_empty());
}

// User singletons (`#[class(singleton)]`) are cached just like engine singletons. Same shape as `engine_singleton_caching`.
#[itest]
fn user_singleton_caching() {
    let id = SomeUserSingleton::singleton().instance_id();
    assert_eq!(SomeUserSingleton::singleton().instance_id(), id);

    godot::init::__invalidate_singleton_caches();
    assert_eq!(SomeUserSingleton::singleton().instance_id(), id);

    // Functional: the cached pointer is usable, not just identity-stable.
    assert_eq!(SomeUserSingleton::singleton().bind().some_method(), 42);
}

#[derive(GodotClass)]
// `#[class(tool, base = Object)]` is implied by `#[class(singleton)]`.
#[class(init, singleton)]
struct SomeUserSingleton {}

#[godot_api]
impl SomeUserSingleton {
    #[func]
    fn some_method(&self) -> u32 {
        42
    }
}
