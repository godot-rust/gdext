/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! A mock implementation of our instance-binding pattern in pure rust.
//!
//! Used so we can run miri on this, which we cannot when we are running in itest against Godot.
//!
//! Currently, the panicking `GdCell` is suitable only for single-threaded use. Without `experimental-threads` enabled,
//! godot-rust will block access to bindings from any thread other than the main one.

use std::collections::HashMap;
use std::error::Error;
use std::marker::PhantomData;
use std::sync::atomic::AtomicUsize;
use std::sync::{Mutex, OnceLock};

use godot_cell::panicking::{GdCell, InaccessibleGuard};

super::setup_mock!(GdCell);

/// `instance_id` must be the key of a `MyClass`.
unsafe fn assert_id_is(instance_id: usize, target: i64) {
    let storage = unsafe { get_instance::<MyClass>(instance_id) };
    let bind = storage.cell.borrow().unwrap();
    assert_eq!(bind.int, target);
}

super::setup_test_class!();

#[test]
fn call_works() {
    let instance_id = MyClass::init();

    unsafe { call_immut_method(instance_id, MyClass::immut_method).unwrap() };
}

/// Run each test once ensuring the integer changes as expected.
#[test]
fn all_calls_work() {
    let instance_id = MyClass::init();

    unsafe {
        assert_id_is(instance_id, 0);
    }

    // We're not running in parallel, so it will never fail to increment completely.
    for (f, _, expected_increment) in CALLS {
        let start = unsafe { get_int(instance_id) };
        unsafe {
            f(instance_id).unwrap();
        }
        unsafe {
            assert_id_is(instance_id, start + *expected_increment);
        }
    }
}
