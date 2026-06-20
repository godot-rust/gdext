/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use godot::classes::node::DuplicateFlags;
use godot::obj::EngineBitfield;

use crate::framework::itest;

const SIGNALS: DuplicateFlags = DuplicateFlags::SIGNALS; // 1
const GROUPS: DuplicateFlags = DuplicateFlags::GROUPS; // 2
const SCRIPTS: DuplicateFlags = DuplicateFlags::SCRIPTS; // 4

#[itest]
fn bitfield_ops_with() {
    let no_flags = DuplicateFlags::from_ord(0);

    assert_eq!(no_flags.with(GROUPS).ord(), 2);
    assert_eq!(GROUPS.with(no_flags).ord(), 2);
    assert_eq!(GROUPS.with(GROUPS).ord(), 2);

    assert_eq!(GROUPS.with(SIGNALS).ord(), 1 | 2);
    assert_eq!(GROUPS.with(GROUPS.with(SIGNALS)).ord(), 1 | 2);
    assert_eq!(GROUPS.with(GROUPS).with(SIGNALS).ord(), 1 | 2);

    assert_eq!(GROUPS.with(SIGNALS).with(SCRIPTS).ord(), 1 | 2 | 4);
}

#[itest]
fn bitfield_ops_without() {
    let no_flags = DuplicateFlags::from_ord(0);

    assert_eq!(no_flags.without(no_flags).ord(), 0);
    assert_eq!(GROUPS.without(GROUPS).ord(), 0);

    assert_eq!(GROUPS.without(SIGNALS).ord(), 2);
    assert_eq!(SIGNALS.with(GROUPS).without(SIGNALS).ord(), 2);
    assert_eq!(SIGNALS.with(GROUPS).without(SIGNALS.with(SCRIPTS)).ord(), 2);

    let all = GROUPS.with(SIGNALS).with(SCRIPTS);
    assert_eq!(all.without(SIGNALS).ord(), 2 | 4);
    assert_eq!(all.without(SIGNALS).without(SCRIPTS).ord(), 2);
}
