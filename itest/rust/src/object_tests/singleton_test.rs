/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use godot::builtin::GString;
use godot::classes::{Input, Os};
use godot::obj::{Gd, Singleton};

use crate::framework::itest;

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
    let key = GString::from("MY_TEST_ENV");
    let value = GString::from("SOME_VALUE");

    // set_environment is const, for some reason
    os.set_environment(&key, &value);

    let read_value = os.get_environment(&key);
    assert_eq!(read_value, value);
}
