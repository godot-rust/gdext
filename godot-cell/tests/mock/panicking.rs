/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! A mock implementation of our instance-binding pattern in pure rust.
//!
//! Used so we can run miri on this, which we cannot when we are running in itest against Godot.

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

/// Run each method both from the main thread and a newly created thread.
#[test]
fn calls_different_thread() {
    use std::thread;

    let instance_id = MyClass::init();

    // We're not running in parallel, so it will never fail to increment completely.
    for (f, _, expected_increment) in CALLS {
        let start = unsafe { get_int(instance_id) };
        unsafe {
            f(instance_id).unwrap();

            assert_id_is(instance_id, start + expected_increment);
        }
        let start = start + expected_increment;
        thread::spawn(move || unsafe { f(instance_id).unwrap() })
            .join()
            .unwrap();
        unsafe {
            assert_id_is(instance_id, start + expected_increment);
        }
    }
}

/// Call each method from different threads, allowing them to run in parallel.
///
/// This may cause borrow failures, we do a best-effort attempt at estimating the value then. We can detect
/// if the first call failed, so then we know the integer was incremented by 0. Otherwise, we at least know
/// the range of values that it can be incremented by.
#[test]
fn calls_parallel() {
    use std::thread;

    let instance_id = MyClass::init();
    let mut handles = Vec::new();

    for (f, min_increment, max_increment) in CALLS {
        let handle = thread::spawn(move || unsafe {
            f(instance_id).map_or((0, 0), |_| (*min_increment, *max_increment))
        });
        handles.push(handle);
    }

    let (min_expected, max_expected) = handles
        .into_iter()
        .map(|handle| handle.join().unwrap())
        .reduce(|(curr_min, curr_max), (min, max)| (curr_min + min, curr_max + max))
        .unwrap();

    unsafe {
        assert!(get_int(instance_id) >= min_expected);
        assert!(get_int(instance_id) <= max_expected);
    }
}

/// Call each method from different threads, allowing them to run in parallel.
///
/// This may cause borrow failures, we do a best-effort attempt at estimating the value then. We can detect
/// if the first call failed, so then we know the integer was incremented by 0. Otherwise, we at least know
/// the range of values that it can be incremented by.
///
/// Runs each method several times in a row. This should reduce the non-determinism that comes from
/// scheduling of threads.
#[test]
fn calls_parallel_many_serial() {
    use std::thread;

    let instance_id = MyClass::init();
    let mut handles = Vec::new();

    for (f, min_increment, max_increment) in CALLS {
        for _ in 0..10 {
            let handle = thread::spawn(move || unsafe {
                f(instance_id).map_or((0, 0), |_| (*min_increment, *max_increment))
            });
            handles.push(handle);
        }
    }

    let (min_expected, max_expected) = handles
        .into_iter()
        .map(|handle| handle.join().unwrap())
        .reduce(|(curr_min, curr_max), (min, max)| (curr_min + min, curr_max + max))
        .unwrap();

    unsafe {
        assert!(get_int(instance_id) >= min_expected);
        assert!(get_int(instance_id) <= max_expected);
    }
}

/// Call each method from different threads, allowing them to run in parallel.
///
/// This may cause borrow failures, we do a best-effort attempt at estimating the value then. We can detect
/// if the first call failed, so then we know the integer was incremented by 0. Otherwise, we at least know
/// the range of values that it can be incremented by.
///
/// Runs all the tests several times. This is different from [`calls_parallel_many_serial`] as that calls the
/// methods like AAA...BBB...CCC..., whereas this interleaves the methods like ABC...ABC...ABC...
#[test]
fn calls_parallel_many_parallel() {
    use std::thread;

    let instance_id = MyClass::init();
    let mut handles = Vec::new();

    for _ in 0..10 {
        for (f, min_increment, max_increment) in CALLS {
            let handle = thread::spawn(move || unsafe {
                f(instance_id).map_or((0, 0), |_| (*min_increment, *max_increment))
            });
            handles.push(handle);
        }
    }

    let (min_expected, max_expected) = handles
        .into_iter()
        .map(|handle| handle.join().unwrap())
        .reduce(|(curr_min, curr_max), (min, max)| (curr_min + min, curr_max + max))
        .unwrap();

    unsafe {
        assert!(get_int(instance_id) >= min_expected);
        assert!(get_int(instance_id) <= max_expected);
    }
}
