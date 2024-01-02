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
use std::pin::Pin;
use std::sync::{atomic::AtomicUsize, Mutex, OnceLock};

use godot_cell::{GdCell, InaccessibleGuard};

struct InstanceBinding(*mut ());

unsafe impl Sync for InstanceBinding {}
unsafe impl Send for InstanceBinding {}

static INSTANCE_BINDINGS: OnceLock<Mutex<HashMap<usize, InstanceBinding>>> = OnceLock::new();

struct InstanceStorage<T> {
    cell: Pin<Box<GdCell<T>>>,
}

fn binding() -> &'static Mutex<HashMap<usize, InstanceBinding>> {
    INSTANCE_BINDINGS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn register_instance<T>(instance: T) -> usize {
    static COUNTER: AtomicUsize = AtomicUsize::new(0);

    let key = COUNTER.fetch_add(1, std::sync::atomic::Ordering::AcqRel);

    let binding = binding();

    let mut guard = binding.lock().unwrap();

    assert!(!guard.contains_key(&key));

    let cell = GdCell::new(instance);
    let storage = Box::new(InstanceStorage { cell });
    let ptr = Box::into_raw(storage) as *mut ();

    guard.insert(key, InstanceBinding(ptr));
    key
}
/*
unsafe fn free_instance<T>(key: usize) {
    let binding = binding();
    let mut guard = binding.lock().unwrap();

    let InstanceBinding(ptr) = guard.remove(&key).unwrap();

    let ptr: *mut InstanceStorage<T> = ptr as *mut _;

    let storage = unsafe { Box::from_raw(ptr) };
}
*/
unsafe fn get_instance<'a, T>(key: usize) -> &'a InstanceStorage<T> {
    let binding = binding();
    let guard = binding.lock().unwrap();

    let instance = guard.get(&key).unwrap();

    let ptr: *mut InstanceStorage<T> = instance.0 as *mut _;

    &*ptr
}

unsafe fn call_immut_method<T>(key: usize, method: fn(&T)) -> Result<(), Box<dyn Error>> {
    let storage = get_instance::<T>(key);

    let instance = storage.cell.as_ref().borrow()?;
    method(&*instance);

    Ok(())
}

unsafe fn call_mut_method<T>(key: usize, method: fn(&mut T)) -> Result<(), Box<dyn Error>> {
    let storage = get_instance::<T>(key);

    let mut instance = storage.cell.as_ref().borrow_mut()?;
    method(&mut *instance);

    Ok(())
}

struct Base<T> {
    instance_id: usize,
    _p: PhantomData<T>,
}

impl<T> Base<T> {
    fn cell<'a, 'b: 'a>(&'a self) -> Pin<&'b GdCell<T>> {
        let storage = unsafe { get_instance::<T>(self.instance_id) };
        storage.cell.as_ref()
    }
}

struct BaseGuard<'a, T> {
    instance_id: usize,
    inaccessible_guard: Option<InaccessibleGuard<'a, T>>,
}

impl<'a, T> BaseGuard<'a, T> {
    fn new(instance_id: usize, inaccessible_guard: InaccessibleGuard<'a, T>) -> Self {
        Self {
            instance_id,
            inaccessible_guard: Some(inaccessible_guard),
        }
    }

    fn call_immut(&self, f: fn(&T)) -> Result<(), Box<dyn Error>> {
        unsafe { call_immut_method(self.instance_id, f) }
    }

    fn call_mut(&self, f: fn(&mut T)) -> Result<(), Box<dyn Error>> {
        unsafe { call_mut_method(self.instance_id, f) }
    }
}

impl<'a, T> Drop for BaseGuard<'a, T> {
    fn drop(&mut self) {
        // Block while waiting for the guard to be droppable.
        // This is only needed to make multi-threaded code work nicely, the default drop-impl for
        // `InaccessibleGuard` may panic and get poisoned in multi-threaded code, which is non-ideal but
        // still safe behavior.
        let mut guard_opt = Some(std::mem::ManuallyDrop::new(
            self.inaccessible_guard.take().unwrap(),
        ));

        while let Some(guard) = guard_opt.take() {
            if let Err(new_guard) = std::mem::ManuallyDrop::into_inner(guard).try_drop() {
                guard_opt = Some(new_guard);
                std::hint::spin_loop()
            }
        }
    }
}

struct MyClass {
    base: Base<MyClass>,
    int: i64,
}

impl MyClass {
    fn init() -> usize {
        let this = Self {
            base: Base {
                instance_id: 0,
                _p: PhantomData,
            },
            int: 0,
        };
        let key = register_instance(this);

        let instance = unsafe { get_instance::<Self>(key) };
        instance
            .cell
            .as_ref()
            .borrow_mut()
            .unwrap()
            .base
            .instance_id = key;
        key
    }

    fn immut_method(&self) {
        println!("immut #1: int is {}", self.int);
    }

    fn mut_method(&mut self) {
        println!("mut #1: int is {}", self.int);
        self.int += 1;
        println!("mut #2: int is now {}", self.int);
    }

    fn mut_method_calls_immut(&mut self) {
        println!("mut_calls_immut #1: int is {}", self.int);
        self.int += 1;
        println!("mut_calls_immut #2: int is now {}", self.int);
        _ = self.base().call_immut(Self::immut_method);
        println!("mut_calls_immut #3: int is now {}", self.int);
    }

    fn mut_method_calls_mut(&mut self) {
        println!("mut_calls_mut #1: int is {}", self.int);
        self.int += 1;
        println!("mut_calls_mut #2: int is now {}", self.int);
        _ = self.base().call_mut(Self::mut_method);
        println!("mut_calls_mut #3: int is now {}", self.int);
    }

    fn mut_method_calls_twice(&mut self) {
        println!("mut_calls_twice #1: int is {}", self.int);
        self.int += 1;
        println!("mut_calls_twice #2: int is now {}", self.int);
        _ = self.base().call_mut(Self::mut_method_calls_immut);
        println!("mut_calls_twice #3: int is now {}", self.int);
    }

    fn mut_method_calls_twice_mut(&mut self) {
        println!("mut_calls_twice_mut #1: int is {}", self.int);
        self.int += 1;
        println!("mut_calls_twice_mut #2: int is now {}", self.int);
        _ = self.base().call_mut(Self::mut_method_calls_mut);
        println!("mut_calls_twice_mut #3: int is now {}", self.int);
    }

    fn immut_calls_immut_directly(&self) {
        println!("immut_calls_directly #1: int is {}", self.int);
        unsafe { call_immut_method(self.base.instance_id, Self::immut_method).unwrap() }
    }

    fn base(&mut self) -> BaseGuard<'_, Self> {
        let cell = self.base.cell();
        BaseGuard::new(self.base.instance_id, cell.make_inaccessible(self).unwrap())
    }
}

#[test]
fn call_works() {
    let instance_id = MyClass::init();

    unsafe { call_immut_method(instance_id, MyClass::immut_method).unwrap() };
}

/// `instance_id` must be the key of a `MyClass`.
unsafe fn get_int(instance_id: usize) -> i64 {
    let storage = unsafe { get_instance::<MyClass>(instance_id) };
    let bind = storage.cell.as_ref().borrow().unwrap();
    bind.int
}

/// `instance_id` must be the key of a `MyClass`.
unsafe fn assert_id_is(instance_id: usize, target: i64) {
    let storage = unsafe { get_instance::<MyClass>(instance_id) };
    let bind = storage.cell.as_ref().borrow().unwrap();
    assert_eq!(bind.int, target);
}

type MethodCall = unsafe fn(usize) -> Result<(), Box<dyn Error>>;

/// A list of each calls to each method of `MyClass`. The numbers are the minimum and maximum increment
/// of the method call.
static CALLS: &[(MethodCall, i64, i64)] = &[
    (
        |id| unsafe { call_immut_method(id, MyClass::immut_method) },
        0,
        0,
    ),
    (
        |id| unsafe { call_mut_method(id, MyClass::mut_method) },
        1,
        1,
    ),
    (
        |id| unsafe { call_mut_method(id, MyClass::mut_method_calls_immut) },
        1,
        1,
    ),
    (
        |id| unsafe { call_mut_method(id, MyClass::mut_method_calls_mut) },
        1,
        2,
    ),
    (
        |id| unsafe { call_mut_method(id, MyClass::mut_method_calls_twice) },
        1,
        2,
    ),
    (
        |id| unsafe { call_mut_method(id, MyClass::mut_method_calls_twice_mut) },
        1,
        3,
    ),
    (
        |id| unsafe { call_immut_method(id, MyClass::immut_calls_immut_directly) },
        0,
        0,
    ),
];

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
/// if the first call failed, so then we know the integer was incremented by 0. Otherwise we at least know
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
/// if the first call failed, so then we know the integer was incremented by 0. Otherwise we at least know
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
/// if the first call failed, so then we know the integer was incremented by 0. Otherwise we at least know
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
