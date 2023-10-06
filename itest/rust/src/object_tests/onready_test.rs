/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::framework::{expect_panic, itest};

use godot::obj::OnReady;

#[itest]
fn lateinit_deref() {
    let mut l = OnReady::<i32>::new(|| 42);
    l.init();

    // DerefMut
    let mut_ref: &mut i32 = &mut *l;
    assert_eq!(*mut_ref, 42);

    // Deref
    let l = l;
    let shared_ref: &i32 = &*l;
    assert_eq!(*shared_ref, 42);
}

#[itest]
fn lateinit_deref_on_uninit() {
    expect_panic("Deref on uninit fails", || {
        let l = OnReady::<i32>::new(|| 42);
        let _ref: &i32 = &*l;
    });

    expect_panic("DerefMut on uninit fails", || {
        let mut l = OnReady::<i32>::new(|| 42);
        let _ref: &mut i32 = &mut *l;
    });
}

#[itest]
fn lateinit_multi_init() {
    expect_panic("init() on already initialized container fails", || {
        let mut l = OnReady::<i32>::new(|| 42);
        l.init();
        l.init();
    });
}
