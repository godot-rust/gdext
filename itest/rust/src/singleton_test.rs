/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::itest;
use godot::builtin::GodotString;
use godot::engine::{Input, OS};
use godot::obj::Gd;

pub fn run() -> bool {
    let mut ok = true;
    ok &= singleton_is_unique();
    ok &= singleton_from_instance_id();
    ok &= singleton_is_operational();
    ok
}

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
    let a: Gd<OS> = OS::singleton();
    let id = a.instance_id();

    let b: Gd<OS> = Gd::from_instance_id(id);

    assert_eq!(a.get_executable_path(), b.get_executable_path());
}

#[itest]
fn singleton_is_operational() {
    let os: Gd<OS> = OS::singleton();
    let key = GodotString::from("MY_TEST_ENV");
    let value = GodotString::from("SOME_VALUE");

    // set_environment is const, for some reason
    os.set_environment(key.clone(), value.clone());

    let read_value = os.get_environment(key);
    assert_eq!(read_value, value);
}
