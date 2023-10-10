/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::framework::itest;
use godot::bind::godot_api;
use godot::builtin::GodotString;
use godot::engine::{Engine, Input, Os};
use godot::obj::Gd;
use godot::prelude::GodotClass;

#[itest]
fn singleton_is_unique() {
    let a: Gd<Input> = Input::singleton();
    let id_a = a.instance_id();

    let b: Gd<Input> = Input::singleton();
    let id_b = b.instance_id();

    assert_eq!(id_a, id_b, "Singletons have same instance ID");
}

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
    let key = GodotString::from("MY_TEST_ENV");
    let value = GodotString::from("SOME_VALUE");

    // set_environment is const, for some reason
    os.set_environment(key.clone(), value.clone());

    let read_value = os.get_environment(key);
    assert_eq!(read_value, value);
}

#[itest(focus)]
fn singleton_manually_registered_is_not_destroyed() {
    let mut engine = Engine::singleton();

    let singleton = Gd::<RustSingleton>::new_default();
    engine.register_singleton("RustSingleton".into(), singleton.upcast());

    // If this is destroyed, a potential use-after-free could occur, as Godot still holds it.
    let singleton_back = engine.get_singleton("RustSingleton".into()).unwrap();
    drop(singleton_back);

    assert!(
        singleton.is_instance_valid(),
        "singletons must not be destroyed"
    );
}

#[derive(GodotClass)]
#[class(init, base=RefCounted)] // RefCounted is crucial here.
struct RustSingleton {}

#[godot_api]
impl RustSingleton {}

impl Drop for RustSingleton {
    fn drop(&mut self) {
        println!("Line {}: dropped!", line!());
    }
}
