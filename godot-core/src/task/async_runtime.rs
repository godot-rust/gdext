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
use std::thread::{self, LocalKey, ThreadId};

use crate::builtin::{Callable, Variant};
use crate::private::handle_panic;

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Public interface

/// Create a new async background task.
///
/// This function allows creating a new async task in which Godot signals can be awaited, like it is possible in GDScript. The
/// [`TaskHandle`] that is returned provides synchronous introspection into the current state of the task.
///
/// Signals can be converted to futures in the following ways:
///
/// | Signal type | Simple future                | Fallible future (handles freed object) |
/// |-------------|------------------------------|----------------------------------------|
/// | Untyped     | [`Signal::to_future()`]      | [`Signal::to_fallible_future()`]       |
/// | Typed       | [`TypedSignal::to_future()`] | [`TypedSignal::to_fallible_future()`]  |
///
/// [`Signal::to_future()`]: crate::builtin::Signal::to_future
/// [`Signal::to_fallible_future()`]: crate::builtin::Signal::to_fallible_future
/// [`TypedSignal::to_future()`]: crate::registry::signal::TypedSignal::to_future
/// [`TypedSignal::to_fallible_future()`]: crate::registry::signal::TypedSignal::to_fallible_future
///
/// # Panics
/// If called from any other thread than the main thread.
///
/// # Examples
/// With typed signals:
///
/// ```no_run
/// # use godot::prelude::*;
/// #[derive(GodotClass)]
/// #[class(init)]
/// struct Building {
///    base: Base<RefCounted>,
/// }
///
/// #[godot_api]
/// impl Building {
///    #[signal]
///    fn constructed(seconds: u32);
/// }
///
/// let house = Building::new_gd();
/// godot::task::spawn(async move {
///     println!("Wait for construction...");
///
///     // Emitted arguments can be fetched in tuple form.
///     // If the signal has no parameters, you can skip `let` and just await the future.
///     let (seconds,) = house.signals().constructed().to_future().await;
///
///     println!("Construction complete after {seconds}s.");
/// });
/// ```
///
/// With untyped signals:
/// ```no_run
/// # use godot::builtin::Signal;
/// # use godot::classes::Node;
/// # use godot::obj::NewAlloc;
/// let node = Node::new_alloc();
/// let signal = Signal::from_object_signal(&node, "signal");
///
/// godot::task::spawn(async move {
///     println!("Starting task...");
///
///     // Explicit generic arguments needed, here `()`:
///     signal.to_future::<()>().await;
///
///     println!("Node has changed: {}", node.get_name());
/// });
/// ```
#[doc(alias = "async")]
pub fn spawn(future: impl Future<Output = ()> + 'static) -> TaskHandle {
    // Spawning new tasks is only allowed on the main thread for now.
    // We can not accept Sync + Send futures since all object references (i.e. Gd<T>) are not thread-safe. So a future has to remain on the
    // same thread it was created on. Godots signals on the other hand can be emitted on any thread, so it can't be guaranteed on which thread
    // a future will be polled.
    // By limiting async tasks to the main thread we can redirect all signal callbacks back to the main thread via `call_deferred`.
    //
    // Once thread-safe futures are possible the restriction can be lifted.
    assert!(
        crate::init::is_main_thread(),
        "godot_task() can only be used on the main thread"
    );

    let (task_handle, godot_waker) = ASYNC_RUNTIME.with_runtime_mut(move |rt| {
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

/// Handle for an active background task.
///
/// This handle provides introspection into the current state of the task, as well as providing a way to cancel it.
///
/// The associated task will **not** be canceled if this handle is dropped.
pub struct TaskHandle {
    index: usize,
    id: u64,
    _no_send_sync: PhantomData<*const ()>,
}

impl TaskHandle {
    fn new(index: usize, id: u64) -> Self {
        Self {
            index,
            id,
            _no_send_sync: PhantomData,
        }
    }

    /// Cancels the task if it is still pending and does nothing if it is already completed.
    pub fn cancel(self) {
        ASYNC_RUNTIME.with_runtime_mut(|rt| {
            let Some(task) = rt.tasks.get(self.index) else {
                // Getting the task from the runtime might return None if the runtime has already been deinitialized. In this case, we just
                // ignore the cancel request, as the entire runtime has already been canceled.
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

    /// Synchronously checks if the task is still pending or has already completed.
    pub fn is_pending(&self) -> bool {
        ASYNC_RUNTIME.with_runtime(|rt| {
            let slot = rt
                .tasks
                .get(self.index)
                .unwrap_or_else(|| unreachable!("missing future slot at index {}", self.index));

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

const ASYNC_RUNTIME_DEINIT_PANIC_MESSAGE: &str = "The async runtime is being accessed after it has been deinitialized. This should not be possible and is most likely a bug.";

thread_local! {
    /// The thread local is only initialized the first time it's used. This means the async runtime won't be allocated until a task is
    /// spawned.
    static ASYNC_RUNTIME: RefCell<Option<AsyncRuntime>> = RefCell::new(Some(AsyncRuntime::new()));
}

/// Will be called during engine shutdown.
///
/// We have to drop all the remaining Futures during engine shutdown. This avoids them being dropped at process termination where they would
/// try to access engine resources, which leads to SEGFAULTs.
pub(crate) fn cleanup() {
    ASYNC_RUNTIME.set(None);
}

#[cfg(feature = "trace")]
pub fn has_godot_task_panicked(task_handle: TaskHandle) -> bool {
    ASYNC_RUNTIME.with_runtime(|rt| rt.panicked_tasks.contains(&task_handle.id))
}

/// The current state of a future inside the async runtime.
enum FutureSlotState<T> {
    /// Slot is currently empty.
    Empty,
    /// Slot was previously occupied but the future has been canceled or the slot reused.
    Gone,
    /// Slot contains a pending future.
    Pending(T),
    /// Slot contains a future which is currently being polled.
    Polling,
}

/// Wrapper around a future that is being stored in the async runtime.
///
/// This wrapper contains additional metadata for the async runtime.
struct FutureSlot<T> {
    value: FutureSlotState<T>,
    id: u64,
}

impl<T> FutureSlot<T> {
    /// Create a new slot with a pending future.
    fn pending(id: u64, value: T) -> Self {
        Self {
            value: FutureSlotState::Pending(value),
            id,
        }
    }

    /// Checks if the future slot is either still empty or has become unoccupied due to a future completing.
    fn is_empty(&self) -> bool {
        matches!(self.value, FutureSlotState::Empty | FutureSlotState::Gone)
    }

    /// Drop the future from this slot.
    ///
    /// This transitions the slot into the [`FutureSlotState::Gone`] state.
    fn clear(&mut self) {
        self.value = FutureSlotState::Gone;
    }

    /// Attempts to extract the future with the given ID from the slot.
    ///
    /// Puts the slot into [`FutureSlotState::Polling`] state after taking the future out. It is expected that the future is either parked
    /// again or the slot is cleared.
    /// In cases were the slot state is not [`FutureSlotState::Pending`], a copy of the state is returned but the slot remains untouched.
    fn take_for_polling(&mut self, id: u64) -> FutureSlotState<T> {
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

    /// Parks the future in this slot again.
    ///
    /// # Panics
    /// - If the slot is not in state [`FutureSlotState::Polling`].
    fn park(&mut self, value: T) {
        match self.value {
            FutureSlotState::Empty | FutureSlotState::Gone => {
                panic!("cannot park future in slot which is unoccupied")
            }
            FutureSlotState::Pending(_) => {
                panic!(
                    "cannot park future in slot, which is already occupied by a different future"
                )
            }
            FutureSlotState::Polling => {
                self.value = FutureSlotState::Pending(value);
            }
        }
    }
}

/// The storage for the pending tasks of the async runtime.
#[derive(Default)]
struct AsyncRuntime {
    tasks: Vec<FutureSlot<Pin<Box<dyn Future<Output = ()>>>>>,
    next_task_id: u64,
    #[cfg(feature = "trace")]
    panicked_tasks: std::collections::HashSet<u64>,
}

impl AsyncRuntime {
    fn new() -> Self {
        Self {
            // We only create a new async runtime inside a thread_local, which has lazy initialization on first use.
            tasks: Vec::with_capacity(16),
            next_task_id: 0,
            #[cfg(feature = "trace")]
            panicked_tasks: std::collections::HashSet::default(),
        }
    }

    /// Get the next task ID.
    fn next_id(&mut self) -> u64 {
        let id = self.next_task_id;
        self.next_task_id += 1;
        id
    }

    /// Store a new async task in the runtime.
    ///
    /// First, a linear search is performed to locate an already existing but currently unoccupied slot in the task buffer. If there is no
    /// free slot, a new slot is added which may grow the underlying [`Vec`].
    ///
    /// The future storage always starts out with a capacity of 10 tasks.
    fn add_task<F: Future<Output = ()> + 'static>(&mut self, future: F) -> TaskHandle {
        let id = self.next_id();
        let index_slot = self
            .tasks
            // If we find an available slot, we will assign the new future to it.
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

    /// Extract a pending task from the storage.
    ///
    /// Attempts to extract a future with the given ID from the specified index and leaves the slot in state [`FutureSlotState::Polling`].
    /// In cases were the slot state is not [`FutureSlotState::Pending`], a copy of the state is returned but the slot remains untouched.
    fn take_task_for_polling(
        &mut self,
        index: usize,
        id: u64,
    ) -> FutureSlotState<Pin<Box<dyn Future<Output = ()> + 'static>>> {
        let slot = self.tasks.get_mut(index);
        slot.map(|inner| inner.take_for_polling(id))
            .unwrap_or(FutureSlotState::Empty)
    }

    /// Remove a future from the storage and free up its slot.
    ///
    /// The slot is left in the [`FutureSlotState::Gone`] state.
    fn clear_task(&mut self, index: usize) {
        self.tasks[index].clear();
    }

    /// Move a future back into its slot.
    ///
    /// # Panic
    /// - If the underlying slot is not in the [`FutureSlotState::Polling`] state.
    fn park_task(&mut self, index: usize, future: Pin<Box<dyn Future<Output = ()>>>) {
        self.tasks[index].park(future);
    }

    /// Track that a future caused a panic.
    ///
    /// This is only available for itest.
    #[cfg(feature = "trace")]
    fn track_panic(&mut self, task_id: u64) {
        self.panicked_tasks.insert(task_id);
    }
}

trait WithRuntime {
    fn with_runtime<R>(&'static self, f: impl FnOnce(&AsyncRuntime) -> R) -> R;
    fn with_runtime_mut<R>(&'static self, f: impl FnOnce(&mut AsyncRuntime) -> R) -> R;
}

impl WithRuntime for LocalKey<RefCell<Option<AsyncRuntime>>> {
    fn with_runtime<R>(&'static self, f: impl FnOnce(&AsyncRuntime) -> R) -> R {
        self.with_borrow(|rt| {
            let rt_ref = rt.as_ref().expect(ASYNC_RUNTIME_DEINIT_PANIC_MESSAGE);

            f(rt_ref)
        })
    }

    fn with_runtime_mut<R>(&'static self, f: impl FnOnce(&mut AsyncRuntime) -> R) -> R {
        self.with_borrow_mut(|rt| {
            let rt_ref = rt.as_mut().expect(ASYNC_RUNTIME_DEINIT_PANIC_MESSAGE);

            f(rt_ref)
        })
    }
}

/// Use a godot waker to poll it's associated future.
///
/// # Panics
/// - If called from a thread other than the main-thread.
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

    // Move future out of the runtime while we are polling it to avoid holding a mutable reference for the entire runtime.
    let future = ASYNC_RUNTIME.with_runtime_mut(|rt| {
        match rt.take_task_for_polling(godot_waker.runtime_index, godot_waker.task_id) {
            FutureSlotState::Empty => {
                panic!("Future slot is empty when waking it! This is a bug!");
            }

            FutureSlotState::Gone => None,

            FutureSlotState::Polling => {
                unreachable!("the same GodotWaker has been called recursively");
            }

            FutureSlotState::Pending(future) => Some(future),
        }
    });

    let Some(future) = future else {
        // Future has been canceled while the waker was already triggered.
        return;
    };

    let error_context = || "Godot async task failed".to_string();

    // If Future::poll() panics, the future is immediately dropped and cannot be accessed again,
    // thus any state that may not have been unwind-safe cannot be observed later.
    let mut future = AssertUnwindSafe(future);

    let panic_result = handle_panic(error_context, move || {
        (future.as_mut().poll(&mut ctx), future)
    });

    let Ok((poll_result, future)) = panic_result else {
        // Polling the future caused a panic. The task state has to be cleaned up and we want track the panic if the trace feature is enabled.
        ASYNC_RUNTIME.with_runtime_mut(|rt| {
            #[cfg(feature = "trace")]
            rt.track_panic(godot_waker.task_id);
            rt.clear_task(godot_waker.runtime_index);
        });

        return;
    };

    // Update the state of the Future in the runtime.
    ASYNC_RUNTIME.with_runtime_mut(|rt| match poll_result {
        // Future is still pending, so we park it again.
        Poll::Pending => rt.park_task(godot_waker.runtime_index, future.0),

        // Future has resolved, so we remove it from the runtime.
        Poll::Ready(()) => rt.clear_task(godot_waker.runtime_index),
    });
}

/// Implementation of a [`Waker`] to poll futures with the engine.
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

// Uses a deferred callable to poll the associated future, i.e. at the end of the current frame.
impl Wake for GodotWaker {
    fn wake(self: Arc<Self>) {
        let mut waker = Some(self);

        /// Enforce the passed closure is generic over its lifetime. The compiler gets confused about the livetime of the argument otherwise.
        /// This appears to be a common issue: https://github.com/rust-lang/rust/issues/89976
        fn callback_type_hint<F>(f: F) -> F
        where
            F: for<'a> FnMut(&'a [&Variant]) -> Variant,
        {
            f
        }

        #[cfg(not(feature = "experimental-threads"))]
        let create_callable = Callable::from_fn;

        #[cfg(feature = "experimental-threads")]
        let create_callable = Callable::from_sync_fn;

        let callable = create_callable(
            "GodotWaker::wake",
            callback_type_hint(move |_args| {
                poll_future(waker.take().expect("Callable will never be called again"));
                Variant::nil()
            }),
        );

        // Schedule waker to poll the Future at the end of the frame.
        callable.call_deferred(&[]);
    }
}
