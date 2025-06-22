/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

// WeakRef is so rarely needed, makes no sense to include in minimal set.
#![cfg(feature = "codegen-full")]

use crate::framework::{expect_panic, itest};
use godot::classes::{Node, RefCounted, WeakRef};
use godot::global::weakref;
use godot::meta::ToGodot;
use godot::obj::{Gd, NewAlloc, NewGd};

#[itest]
fn weakref_default() {
    let weak_instance = WeakRef::new_gd();
    let weak_ref_v = weak_instance.get_ref();
    assert!(weak_ref_v.is_nil());
}

#[itest]
fn weakref_manual() {
    let manual = Node::new_alloc();

    let weak_instance_v = weakref(&manual.to_variant());
    let weak_instance = weak_instance_v.to::<Gd<WeakRef>>();

    let weak_ref_v = weak_instance.get_ref();
    let weak_ref = weak_ref_v.to::<Gd<Node>>();

    assert_eq!(weak_ref, manual);
    manual.free();

    // Now dead.
    let weak_ref_v = weak_instance.get_ref();
    assert!(weak_ref_v.is_nil());
}

#[itest]
fn weakref_refcounted() {
    let refc = RefCounted::new_gd();

    let weak_instance_v = weakref(&refc.to_variant());
    let weak_instance = weak_instance_v.to::<Gd<WeakRef>>();

    let weak_ref_v = weak_instance.get_ref();
    let weak_ref = weak_ref_v.to::<Gd<RefCounted>>();

    assert_eq!(weak_ref, refc);
    assert_eq!(refc.get_reference_count(), 3);
    drop(weak_ref);
    drop(weak_ref_v);
    drop(refc);

    // Now dead.
    let weak_ref_v = weak_instance.get_ref();
    assert!(weak_ref_v.is_nil());
}
