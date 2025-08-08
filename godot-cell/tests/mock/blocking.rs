/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! A mock implementation of our instance-binding pattern in pure rust for the blocking variant of GdCell.
//!
//! Used so we can run miri on this, which we cannot when we are running in itest against Godot.

use std::collections::HashMap;
use std::error::Error;
use std::marker::PhantomData;
use std::sync::atomic::AtomicUsize;
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

use godot_cell::blocking::{GdCell, InaccessibleGuard};

super::setup_mock!(GdCell);

// ----------------------------------------------------------------------------------------------------------------------------------------------

super::setup_test_class!();

impl MyClass {
    fn immut_calls_mut_directly(&self) {
        unsafe { call_mut_method(self.base.instance_id, Self::mut_method).unwrap() }
    }

    fn immut_with_sleep(&self) {
        std::thread::sleep(Duration::from_millis(100));
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

/// Call each method from different threads, allowing them to run in parallel.
///
/// This should not cause borrow failures and should not lead to deadlocks.
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
        assert!(get_int(instance_id) == max_expected);
    }
}

/// Call each method from different threads, allowing them to run in parallel.
///
/// This should not cause borrow failures and should not lead to deadlocks.
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
        assert!(get_int(instance_id) == max_expected);
    }
}

/// Call each method from different threads, allowing them to run in parallel.
///
/// This should not cause borrow failures and should not lead to deadlocks.
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
        assert!(get_int(instance_id) == max_expected);
    }
}

/// Reborrow the same cell on multiple threads.
///
/// This verifies that threads don't hang if a reborrow occours inside them, i.e. immutable borrow followed
/// by a mutable borrow.
///
/// Threads should only block if:
/// a) Thread A holds mutable reference AND thread B holds no references.
/// b) One or more threads hold shared references AND thread A holds no references
#[test]
fn non_blocking_reborrow() {
    use std::thread;
    let instance_id = MyClass::init();

    let thread_a = thread::spawn(move || unsafe {
        // Acquire an immutable reference and then try to also acquire a mutable reference.
        // This should panic.
        call_immut_method(instance_id, MyClass::immut_calls_mut_directly).unwrap();
    });

    let thread_b = thread::spawn(move || unsafe {
        // Do the same thing as the other thread. This should not block, but panic. Shared and mutable references in the same thread
        // are not possible.
        call_immut_method(instance_id, MyClass::immut_calls_mut_directly).unwrap();
    });

    let panic_a = thread_a.join().err();

    assert_eq!(panic_a.unwrap().downcast_ref::<String>().unwrap(), "called `Result::unwrap()` on an `Err` value: Custom(\"cannot borrow mutable while shared borrow exists\")");

    let panic_b = thread_b.join().err();

    assert_eq!(panic_b.unwrap().downcast_ref::<String>().unwrap(), "called `Result::unwrap()` on an `Err` value: Custom(\"cannot borrow mutable while shared borrow exists\")");
}

/// Mutable borrow on main thread with shared borrow on others.
///
/// This verifies that the thread which initialized the `GdCell` does not panic when it attempts to mutably borrow while there is already a
/// shared borrow on an other thread.
#[test]
fn no_mut_panic_on_main() {
    use std::thread;
    let instance_id = MyClass::init();

    let thread_a = thread::spawn(move || unsafe {
        // Acquire an immutable reference and sleep for a while.
        call_immut_method(instance_id, MyClass::immut_with_sleep).unwrap();
    });

    thread::sleep(Duration::from_millis(50));

    let main_result = unsafe { call_mut_method(instance_id, MyClass::mut_method) };

    main_result.expect("The main thread should not panic!");
    thread_a.join().expect("Thread_a should not panic!");
}
