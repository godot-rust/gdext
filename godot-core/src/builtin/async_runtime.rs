/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::cell::RefCell;
use std::future::Future;
use std::marker::PhantomData;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll, Wake, Waker};
use std::thread::{self, ThreadId};

use crate::builtin::{Callable, Variant};
use crate::classes::Os;
use crate::meta::ToGodot;

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Public interface

pub fn godot_task(future: impl Future<Output = ()> + 'static) -> TaskHandle {
    let os = Os::singleton();

    // Spawning new tasks is only allowed on the main thread for now.
    // We can not accept Sync + Send futures since all object references (i.e. Gd<T>) are not thread-safe. So a future has to remain on the
    // same thread it was created on. Godots signals on the other hand can be emitted on any thread, so it can't be guaranteed on which thread
    // a future will be polled.
    // By limiting async tasks to the main thread we can redirect all signal callbacks back to the main thread via `call_deferred`.
    //
    // Once thread-safe futures are possible the restriction can be lifted.
    if os.get_thread_caller_id() != os.get_main_thread_id() {
        panic!("godot_task can only be used on the main thread!");
    }

    let (task_handle, waker) = ASYNC_RUNTIME.with_borrow_mut(move |rt| {
        let task_handle = rt.add_task(Box::pin(future));
        let godot_waker = Arc::new(GodotWaker::new(
            task_handle.index,
            task_handle.id,
            thread::current().id(),
        ));

        (task_handle, Waker::from(godot_waker))
    });

    waker.wake();
    task_handle
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Async Runtime

thread_local! {
    pub(crate) static ASYNC_RUNTIME: RefCell<AsyncRuntime> = RefCell::new(AsyncRuntime::new());
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
        self.value = FutureSlotState::Empty;
    }

    fn cancel(&mut self) {
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
                FutureSlotState::Empty | FutureSlotState::Gone => false,
                FutureSlotState::Pending(_) => task.id == self.id,
                FutureSlotState::Polling => panic!("Can not cancel future from inside it!"),
            };

            if !alive {
                return;
            }

            rt.cancel_task(self.index);
        })
    }

    pub fn is_pending(&self) -> bool {
        ASYNC_RUNTIME.with_borrow(|rt| {
            let slot = rt.tasks.get(self.index).expect("Slot at index must exist!");

            if slot.id != self.id {
                return false;
            }

            matches!(slot.value, FutureSlotState::Pending(_))
        })
    }
}

#[derive(Default)]
pub(crate) struct AsyncRuntime {
    tasks: Vec<FutureSlot<Pin<Box<dyn Future<Output = ()>>>>>,
    task_counter: u64,
}

impl AsyncRuntime {
    fn new() -> Self {
        Self {
            tasks: Vec::with_capacity(10),
            task_counter: 0,
        }
    }

    fn next_id(&mut self) -> u64 {
        let id = self.task_counter;
        self.task_counter += 1;
        id
    }

    fn add_task<F: Future<Output = ()> + 'static>(&mut self, future: F) -> TaskHandle {
        let id = self.next_id();
        let slot = self
            .tasks
            .iter_mut()
            .enumerate()
            .find(|(_, slot)| slot.is_empty());

        let boxed = Box::pin(future);

        let index = match slot {
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

    fn cancel_task(&mut self, index: usize) {
        self.tasks[index].cancel();
    }

    fn park_task(&mut self, index: usize, future: Pin<Box<dyn Future<Output = ()>>>) {
        self.tasks[index].park(future);
    }
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
    fn wake(self: std::sync::Arc<Self>) {
        let callable = Callable::from_local_fn("GodotWaker::wake", move |_args| {
            let current_thread = thread::current().id();

            if self.thread_id != current_thread {
                panic!("trying to poll future on a different thread!\nCurrent Thread: {:?}, Future Thread: {:?}", current_thread, self.thread_id);
            }

            let waker = Waker::from(self.clone());
            let mut ctx = Context::from_waker(&waker);

            // take future out of the runtime.
            let future = ASYNC_RUNTIME.with_borrow_mut(|rt| {
                match rt.get_task(self.runtime_index, self.task_id) {
                    FutureSlotState::Empty => {
                        panic!("Future no longer exists when waking it! This is a bug!");
                    },

                    FutureSlotState::Gone => {
                        None
                    }

                    FutureSlotState::Polling => {
                        panic!("The same GodotWaker has been called recursively, this is not expected!");
                    }

                    FutureSlotState::Pending(future) => Some(future),
                }
            });

            let Some(mut future) = future else {
                // future has been canceled while the waker was already triggered.
                return Ok(Variant::nil());
            };

            let result = future.as_mut().poll(&mut ctx);

            // update runtime.
            ASYNC_RUNTIME.with_borrow_mut(|rt| match result {
                Poll::Pending => rt.park_task(self.runtime_index, future),
                Poll::Ready(()) => rt.clear_task(self.runtime_index),
            });

            Ok(Variant::nil())
        });

        // shedule waker to poll the future on the end of the frame.
        callable.to_variant().call("call_deferred", &[]);
    }
}
