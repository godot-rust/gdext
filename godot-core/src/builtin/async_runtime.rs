/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::cell::RefCell;
use std::future::Future;
use std::marker::PhantomData;
use std::panic::AssertUnwindSafe;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll, Wake, Waker};
use std::thread::{self, ThreadId};

use crate::builtin::{Callable, Variant};
use crate::private::handle_panic;

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Public interface

pub fn godot_task(future: impl Future<Output = ()> + 'static) -> TaskHandle {
    // Spawning new tasks is only allowed on the main thread for now.
    // We can not accept Sync + Send futures since all object references (i.e. Gd<T>) are not thread-safe. So a future has to remain on the
    // same thread it was created on. Godots signals on the other hand can be emitted on any thread, so it can't be guaranteed on which thread
    // a future will be polled.
    // By limiting async tasks to the main thread we can redirect all signal callbacks back to the main thread via `call_deferred`.
    //
    // Once thread-safe futures are possible the restriction can be lifted.
    assert!(
        crate::init::is_main_thread(),
        "godot_task can only be used on the main thread!"
    );

    let (task_handle, godot_waker) = ASYNC_RUNTIME.with_borrow_mut(move |rt| {
        let task_handle = rt.add_task(Box::pin(future));
        let godot_waker = Arc::new(GodotWaker::new(
            task_handle.index,
            task_handle.id,
            thread::current().id(),
        ));

        (task_handle, godot_waker)
    });

    poll_future(godot_waker);
    task_handle
}

pub struct TaskHandle {
    index: usize,
    id: u64,
    _pd: PhantomData<*const ()>,
}

impl TaskHandle {
    fn new(index: usize, id: u64) -> Self {
        Self {
            index,
            id,
            _pd: PhantomData,
        }
    }

    pub fn cancel(self) {
        ASYNC_RUNTIME.with_borrow_mut(|rt| {
            let Some(task) = rt.tasks.get(self.index) else {
                return;
            };

            let alive = match task.value {
                FutureSlotState::Empty => {
                    panic!("Future slot is empty when canceling it! This is a bug!")
                }
                FutureSlotState::Gone => false,
                FutureSlotState::Pending(_) => task.id == self.id,
                FutureSlotState::Polling => panic!("Can not cancel future from inside it!"),
            };

            if !alive {
                return;
            }

            rt.clear_task(self.index);
        })
    }

    pub fn is_pending(&self) -> bool {
        ASYNC_RUNTIME.with_borrow(|rt| {
            let slot = rt.tasks.get(self.index).expect("Slot at index must exist!");

            if slot.id != self.id {
                return false;
            }

            matches!(
                slot.value,
                FutureSlotState::Pending(_) | FutureSlotState::Polling
            )
        })
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Async Runtime

thread_local! {
    static ASYNC_RUNTIME: RefCell<AsyncRuntime> = RefCell::new(AsyncRuntime::new());
}

/// Will be called during engine shudown.
///
/// We have to drop all the remaining Futures during engine shutdown. This avoids them being dropped at process termination where they would
/// try to access engine resources, which leads to SEGFAULTs.
pub(crate) fn cleanup() {
    ASYNC_RUNTIME.take();
}

#[cfg(feature = "trace")]
pub fn is_godot_task_panicked(task_handle: TaskHandle) -> bool {
    ASYNC_RUNTIME.with_borrow(|rt| rt.panicked_tasks.contains(&task_handle.id))
}

#[derive(Default)]
enum FutureSlotState<T> {
    /// Slot is currently empty.
    #[default]
    Empty,
    /// Slot was previously occupied but the future has been canceled or the slot reused.
    Gone,
    /// Slot contains a pending future.
    Pending(T),
    /// Slot contains a future which is currently being polled.
    Polling,
}

struct FutureSlot<T> {
    value: FutureSlotState<T>,
    id: u64,
}

impl<T> FutureSlot<T> {
    fn pending(id: u64, value: T) -> Self {
        Self {
            value: FutureSlotState::Pending(value),
            id,
        }
    }

    fn is_empty(&self) -> bool {
        matches!(self.value, FutureSlotState::Empty | FutureSlotState::Gone)
    }

    fn clear(&mut self) {
        self.value = FutureSlotState::Gone;
    }

    fn take(&mut self, id: u64) -> FutureSlotState<T> {
        match self.value {
            FutureSlotState::Empty => FutureSlotState::Empty,
            FutureSlotState::Polling => FutureSlotState::Polling,
            FutureSlotState::Gone => FutureSlotState::Gone,
            FutureSlotState::Pending(_) if self.id != id => FutureSlotState::Gone,
            FutureSlotState::Pending(_) => {
                std::mem::replace(&mut self.value, FutureSlotState::Polling)
            }
        }
    }

    fn park(&mut self, value: T) {
        match self.value {
            FutureSlotState::Empty | FutureSlotState::Gone => {
                panic!("Future slot is currently unoccupied, future can not be parked here!");
            }
            FutureSlotState::Pending(_) => {
                panic!("Future slot is already occupied by a different future!")
            }
            FutureSlotState::Polling => {
                self.value = FutureSlotState::Pending(value);
            }
        }
    }
}

#[derive(Default)]
struct AsyncRuntime {
    tasks: Vec<FutureSlot<Pin<Box<dyn Future<Output = ()>>>>>,
    task_counter: u64,
    #[cfg(feature = "trace")]
    panicked_tasks: std::collections::HashSet<u64>,
}

impl AsyncRuntime {
    fn new() -> Self {
        Self {
            tasks: Vec::with_capacity(10),
            task_counter: 0,
            #[cfg(feature = "trace")]
            panicked_tasks: std::collections::HashSet::default(),
        }
    }

    fn next_id(&mut self) -> u64 {
        let id = self.task_counter;
        self.task_counter += 1;
        id
    }

    fn add_task<F: Future<Output = ()> + 'static>(&mut self, future: F) -> TaskHandle {
        let id = self.next_id();
        let index_slot = self
            .tasks
            .iter_mut()
            .enumerate()
            .find(|(_, slot)| slot.is_empty());

        let boxed = Box::pin(future);

        let index = match index_slot {
            Some((index, slot)) => {
                *slot = FutureSlot::pending(id, boxed);
                index
            }
            None => {
                self.tasks.push(FutureSlot::pending(id, boxed));
                self.tasks.len() - 1
            }
        };

        TaskHandle::new(index, id)
    }

    fn get_task(
        &mut self,
        index: usize,
        id: u64,
    ) -> FutureSlotState<Pin<Box<dyn Future<Output = ()> + 'static>>> {
        let slot = self.tasks.get_mut(index);

        slot.map(|inner| inner.take(id)).unwrap_or_default()
    }

    fn clear_task(&mut self, index: usize) {
        self.tasks[index].clear();
    }

    fn park_task(&mut self, index: usize, future: Pin<Box<dyn Future<Output = ()>>>) {
        self.tasks[index].park(future);
    }

    #[cfg(feature = "trace")]
    fn track_panic(&mut self, task_id: u64) {
        self.panicked_tasks.insert(task_id);
    }
}

fn poll_future(godot_waker: Arc<GodotWaker>) {
    let current_thread = thread::current().id();

    assert_eq!(
                godot_waker.thread_id,
                current_thread,
                "trying to poll future on a different thread!\n  Current thread: {:?}\n  Future thread: {:?}",
                current_thread,
                godot_waker.thread_id,
            );

    let waker = Waker::from(godot_waker.clone());
    let mut ctx = Context::from_waker(&waker);

    // Take future out of the runtime.
    let future = ASYNC_RUNTIME.with_borrow_mut(|rt| {
        match rt.get_task(godot_waker.runtime_index, godot_waker.task_id) {
            FutureSlotState::Empty => {
                panic!("Future slot is empty when waking it! This is a bug!");
            }

            FutureSlotState::Gone => None,

            FutureSlotState::Polling => {
                panic!("The same GodotWaker has been called recursively, this is not expected!");
            }

            FutureSlotState::Pending(future) => Some(future),
        }
    });

    let Some(future) = future else {
        // Future has been canceled while the waker was already triggered.
        return;
    };

    let error_context = || "Godot async task failed";
    let mut future = AssertUnwindSafe(future);

    let panic_result = handle_panic(error_context, move || {
        (future.as_mut().poll(&mut ctx), future)
    });

    let Ok((poll_result, future)) = panic_result else {
        ASYNC_RUNTIME.with_borrow_mut(|rt| {
            #[cfg(feature = "trace")]
            rt.track_panic(godot_waker.task_id);
            rt.clear_task(godot_waker.runtime_index);
        });

        return;
    };

    // Update the state of the Future in the runtime.
    ASYNC_RUNTIME.with_borrow_mut(|rt| match poll_result {
        // Future is still pending, so we park it again.
        Poll::Pending => rt.park_task(godot_waker.runtime_index, future.0),

        // Future has resolved, so we remove it from the runtime.
        Poll::Ready(()) => rt.clear_task(godot_waker.runtime_index),
    });
}

struct GodotWaker {
    runtime_index: usize,
    task_id: u64,
    thread_id: ThreadId,
}

impl GodotWaker {
    fn new(index: usize, task_id: u64, thread_id: ThreadId) -> Self {
        Self {
            runtime_index: index,
            thread_id,
            task_id,
        }
    }
}

impl Wake for GodotWaker {
    fn wake(self: Arc<Self>) {
        let mut waker = Some(self);
        let callable = Callable::from_local_fn("GodotWaker::wake", move |_args| {
            poll_future(waker.take().expect("Callable will never be called again"));
            Ok(Variant::nil())
        });

        // Schedule waker to poll the Future at the end of the frame.
        callable.call_deferred(&[]);
    }
}
