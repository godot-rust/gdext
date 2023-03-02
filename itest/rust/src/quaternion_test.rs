/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::itest;
use godot::builtin::Quaternion;

#[itest]
fn quaternion_default() {
    let quat = Quaternion::default();

    assert_eq!(quat.x, 0.0);
    assert_eq!(quat.y, 0.0);
    assert_eq!(quat.z, 0.0);
    assert_eq!(quat.w, 1.0);
}

#[itest]
fn quaternion_from_xyzw() {
    let quat = Quaternion::new(0.2391, 0.099, 0.3696, 0.8924);

    assert_eq!(quat.x, 0.2391);
    assert_eq!(quat.y, 0.099);
    assert_eq!(quat.z, 0.3696);
    assert_eq!(quat.w, 0.8924);
}

// TODO more tests
