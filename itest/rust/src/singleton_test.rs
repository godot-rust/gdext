/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::itest;
use godot_core::api::OS;
use godot_core::obj::Gd;

pub fn run() -> bool {
    let mut ok = true;
    ok &= singleton_is_unique();
    ok &= singleton_from_instance_id();
    ok &= singleton_is_operational();
    ok
}

#[itest]
fn singleton_is_unique() {
    let a: Gd<OS> = OS::singleton();
    let id_a = a.instance_id();

    let b: Gd<OS> = OS::singleton();
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
    let pid = os.get_process_id();
    let running = os.is_process_running(pid);
    assert!(running, "own process is running");
}
