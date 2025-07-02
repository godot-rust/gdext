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

#[cfg(feature = "tokio")]
use tokio::runtime::Handle;

use crate::builtin::{Callable, Variant};
use crate::private::handle_panic;

// *** Added: Support async Future with return values ***

use crate::classes::RefCounted;
use crate::meta::ToGodot;
use crate::obj::{Gd, NewGd};

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
    // In single-threaded mode, spawning is only allowed on the main thread.
    // We can not accept Sync + Send futures since all object references (i.e. Gd<T>) are not thread-safe. So a future has to remain on the
    // same thread it was created on. Godots signals on the other hand can be emitted on any thread, so it can't be guaranteed on which thread
    // a future will be polled.
    // By limiting async tasks to the main thread we can redirect all signal callbacks back to the main thread via `call_deferred`.
    //
    // In multi-threaded mode with experimental-threads, the restriction is lifted.
    #[cfg(all(not(wasm_nothreads), not(feature = "experimental-threads")))]
    assert!(
        crate::init::is_main_thread(),
        "spawn() can only be used on the main thread in single-threaded mode"
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

/// Spawn an async task that returns a value.
///
/// Unlike [`spawn`], this function returns a [`Gd<RefCounted>`] that can be
/// directly awaited in GDScript. When the async task completes, the object emits
/// a `finished` signal with the result.
///
/// # Example
/// ```rust
/// use godot_core::task::spawn_with_result;
///
/// let async_task = spawn_with_result(async {
///     // Some async computation that returns a value
///     42
/// });
///
/// // In GDScript:
/// // var result = await Signal(async_task, "finished")
/// ```
pub fn spawn_with_result<F, R>(future: F) -> Gd<RefCounted>
where
    F: Future<Output = R> + Send + 'static,
    R: ToGodot + Send + Sync + 'static,
{
    // In single-threaded mode, spawning is only allowed on the main thread
    // In multi-threaded mode, we allow spawning from any thread
    #[cfg(all(not(wasm_nothreads), not(feature = "experimental-threads")))]
    assert!(
        crate::init::is_main_thread(),
        "spawn_with_result() can only be used on the main thread in single-threaded mode"
    );
    // Create a RefCounted object that will emit the completion signal
    let mut signal_emitter = RefCounted::new_gd();

    // Add a user-defined signal that takes a Variant parameter
    signal_emitter.add_user_signal("finished");

    let emitter_clone = signal_emitter.clone();

    let godot_waker = ASYNC_RUNTIME.with_runtime_mut(|rt| {
        // Create a wrapper that will emit the signal when complete
        let result_future = SignalEmittingFuture {
            inner: future,
            signal_emitter: emitter_clone,
        };

        // Spawn the signal-emitting future using standard spawn mechanism
        let task_handle = rt.add_task(Box::pin(result_future));

        // Create waker to trigger initial poll
        Arc::new(GodotWaker::new(
            task_handle.index,
            task_handle.id,
            thread::current().id(),
        ))
    });

    // Trigger initial poll
    poll_future(godot_waker);

    signal_emitter
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
    #[cfg(feature = "tokio")]
    _tokio_handle: Option<Handle>,
}

/// Wrapper for futures that stores results as Variants in external storage
/// Wrapper for futures that emits a signal when the future completes
struct SignalEmittingFuture<F, R>
where
    F: Future<Output = R>,
    R: ToGodot + Send + Sync + 'static,
{
    inner: F,
    signal_emitter: Gd<RefCounted>,
}

impl<F, R> Future for SignalEmittingFuture<F, R>
where
    F: Future<Output = R>,
    R: ToGodot + Send + Sync + 'static,
{
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        // SAFETY: We're only projecting to fields that are safe to pin project
        let this = unsafe { self.get_unchecked_mut() };
        let inner_pin = unsafe { Pin::new_unchecked(&mut this.inner) };

        match inner_pin.poll(cx) {
            Poll::Ready(result) => {
                // Convert the result to Variant and emit the completion signal
                let variant_result = result.to_variant();

                // Use call_deferred to ensure signal emission happens on the main thread
                let mut signal_emitter = this.signal_emitter.clone();
                let variant_result_clone = variant_result.clone();
                let callable = Callable::from_local_fn("emit_finished_signal", move |_args| {
                    signal_emitter.emit_signal("finished", &[variant_result_clone.clone()]);
                    Ok(Variant::nil())
                });

                callable.call_deferred(&[]);
                Poll::Ready(())
            }
            Poll::Pending => Poll::Pending,
        }
    }
}

// SAFETY: SignalEmittingFuture is Send if F and R are Send, which is required by our bounds
unsafe impl<F, R> Send for SignalEmittingFuture<F, R>
where
    F: Future<Output = R> + Send,
    R: ToGodot + Send + Sync + 'static,
{
}

impl AsyncRuntime {
    fn new() -> Self {
        #[cfg(feature = "tokio")]
        let tokio_handle = {
            // Use multi-threaded runtime when experimental-threads is enabled
            #[cfg(feature = "experimental-threads")]
            let mut builder = tokio::runtime::Builder::new_multi_thread();

            #[cfg(not(feature = "experimental-threads"))]
            let mut builder = tokio::runtime::Builder::new_current_thread();

            match builder.enable_all().build() {
                Ok(rt) => {
                    // Start the runtime in a separate thread to keep it running
                    let rt_handle = rt.handle().clone();
                    std::thread::spawn(move || {
                        rt.block_on(async {
                            // Keep the runtime alive indefinitely
                            loop {
                                tokio::time::sleep(std::time::Duration::from_secs(3600)).await;
                            }
                        })
                    });

                    Some(rt_handle)
                }
                Err(_e) => None,
            }
        };

        Self {
            tasks: Vec::new(),
            next_task_id: 0,
            #[cfg(feature = "trace")]
            panicked_tasks: std::collections::HashSet::new(),
            #[cfg(feature = "tokio")]
            _tokio_handle: tokio_handle,
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

        let index_slot = self.tasks.iter_mut().enumerate().find_map(|(index, slot)| {
            if slot.is_empty() {
                Some((index, slot))
            } else {
                None
            }
        });

        let boxed_future: Pin<Box<dyn Future<Output = ()> + 'static>> = Box::pin(future);

        let index = match index_slot {
            Some((index, slot)) => {
                *slot = FutureSlot::pending(id, boxed_future);
                index
            }
            None => {
                self.tasks.push(FutureSlot::pending(id, boxed_future));
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

    // Execute the poll operation within tokio context if available
    let panic_result = {
        #[cfg(feature = "tokio")]
        {
            ASYNC_RUNTIME.with_runtime(|rt| {
                if let Some(tokio_handle) = rt._tokio_handle.as_ref() {
                    let _guard = tokio_handle.enter();
                    handle_panic(error_context, move || {
                        (future.as_mut().poll(&mut ctx), future)
                    })
                } else {
                    handle_panic(error_context, move || {
                        (future.as_mut().poll(&mut ctx), future)
                    })
                }
            })
        }

        #[cfg(not(feature = "tokio"))]
        {
            handle_panic(error_context, move || {
                (future.as_mut().poll(&mut ctx), future)
            })
        }
    };

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
            F: for<'a> FnMut(&'a [&Variant]) -> Result<Variant, ()>,
        {
            f
        }

        #[cfg(not(feature = "experimental-threads"))]
        let create_callable = Callable::from_local_fn;

        #[cfg(feature = "experimental-threads")]
        let create_callable = Callable::from_sync_fn;

        let callable = create_callable(
            "GodotWaker::wake",
            callback_type_hint(move |_args| {
                poll_future(waker.take().expect("Callable will never be called again"));
                Ok(Variant::nil())
            }),
        );

        // Schedule waker to poll the Future at the end of the frame.
        callable.call_deferred(&[]);
    }
}
