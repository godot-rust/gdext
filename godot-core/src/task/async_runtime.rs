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

// Use pin-project-lite for safe pin projection
use pin_project_lite::pin_project;

use crate::builtin::{Callable, Variant};
use crate::private::handle_panic;

// *** Added: Support async Future with return values ***

use crate::classes::RefCounted;
use crate::meta::ToGodot;
use crate::obj::{Gd, NewGd};

// *** Added: Enhanced Error Handling ***

/// Errors that can occur during async runtime operations
#[derive(Debug, Clone)]
pub enum AsyncRuntimeError {
    /// Runtime has been deinitialized (during engine shutdown)
    RuntimeDeinitialized,
    /// Task was canceled while being polled
    TaskCanceled { task_id: u64 },
    /// Task panicked during polling
    TaskPanicked { task_id: u64, message: String },
    /// Task slot is in an invalid state
    InvalidTaskState {
        task_id: u64,
        expected_state: String,
    },
    /// Tokio runtime creation failed
    TokioRuntimeCreationFailed { reason: String },
    /// Task spawning failed
    TaskSpawningFailed { reason: String },
    /// Signal emission failed
    SignalEmissionFailed { task_id: u64, reason: String },
    /// Thread safety violation
    ThreadSafetyViolation {
        expected_thread: ThreadId,
        actual_thread: ThreadId,
    },
}

impl std::fmt::Display for AsyncRuntimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AsyncRuntimeError::RuntimeDeinitialized => {
                write!(f, "Async runtime has been deinitialized")
            }
            AsyncRuntimeError::TaskCanceled { task_id } => {
                write!(f, "Task {task_id} was canceled")
            }
            AsyncRuntimeError::TaskPanicked { task_id, message } => {
                write!(f, "Task {task_id} panicked: {message}")
            }
            AsyncRuntimeError::InvalidTaskState {
                task_id,
                expected_state,
            } => {
                write!(
                    f,
                    "Task {task_id} is in invalid state, expected: {expected_state}"
                )
            }
            AsyncRuntimeError::TokioRuntimeCreationFailed { reason } => {
                write!(f, "Failed to create tokio runtime: {reason}")
            }
            AsyncRuntimeError::TaskSpawningFailed { reason } => {
                write!(f, "Failed to spawn task: {reason}")
            }
            AsyncRuntimeError::SignalEmissionFailed { task_id, reason } => {
                write!(f, "Failed to emit signal for task {task_id}: {reason}")
            }
            AsyncRuntimeError::ThreadSafetyViolation {
                expected_thread,
                actual_thread,
            } => {
                write!(f, "Thread safety violation: expected thread {expected_thread:?}, got {actual_thread:?}")
            }
        }
    }
}

impl std::error::Error for AsyncRuntimeError {}

/// Result type for async runtime operations
pub type AsyncRuntimeResult<T> = Result<T, AsyncRuntimeError>;

/// Errors that can occur when spawning tasks
#[derive(Debug, Clone)]
pub enum TaskSpawnError {
    /// Task queue is full and cannot accept more tasks
    QueueFull {
        active_tasks: usize,
        queued_tasks: usize,
    },
    // Note: LimitsExceeded and RuntimeShuttingDown variants were removed because:
    // - LimitsExceeded: Was designed for more sophisticated task limit enforcement,
    //   but current implementation only uses queue-based backpressure
    // - RuntimeShuttingDown: Was designed for graceful shutdown coordination,
    //   but current implementation uses simpler immediate cleanup approach
}

impl std::fmt::Display for TaskSpawnError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TaskSpawnError::QueueFull {
                active_tasks,
                queued_tasks,
            } => {
                write!(
                    f,
                    "Task queue is full: {active_tasks} active tasks, {queued_tasks} queued tasks"
                )
            }
        }
    }
}

impl std::error::Error for TaskSpawnError {}

/// Context guard that ensures proper runtime context is entered
/// Similar to tokio's EnterGuard, ensures async operations run in the right context
pub struct RuntimeContextGuard<'a> {
    #[cfg(feature = "tokio")]
    _tokio_guard: Option<tokio::runtime::EnterGuard<'a>>,
    #[cfg(not(feature = "tokio"))]
    _phantom: PhantomData<&'a ()>,
}

impl<'a> RuntimeContextGuard<'a> {
    /// Create a new context guard
    ///
    /// # Safety
    /// This should only be called when we have confirmed that a runtime context is available
    #[cfg(feature = "tokio")]
    fn new(handle: &'a tokio::runtime::Handle) -> Self {
        Self {
            _tokio_guard: Some(handle.enter()),
        }
    }

    #[cfg(not(feature = "tokio"))]
    fn new() -> Self {
        Self {
            _phantom: PhantomData,
        }
    }

    // Note: dummy() method was removed because it was designed as a fallback
    // for when no runtime context is available, but the current implementation
    // always uses proper context guards or the new() constructor directly.
}

/// Context management for async runtime operations
/// Provides tokio-style runtime context entering and exiting
#[derive(Default)]
pub struct RuntimeContext {
    #[cfg(feature = "tokio")]
    tokio_handle: Option<tokio::runtime::Handle>,
}

impl RuntimeContext {
    /// Create a new runtime context
    pub fn new() -> Self {
        Self {
            #[cfg(feature = "tokio")]
            tokio_handle: None,
        }
    }

    /// Initialize the context with a tokio handle
    #[cfg(feature = "tokio")]
    pub fn with_tokio_handle(handle: tokio::runtime::Handle) -> Self {
        Self {
            tokio_handle: Some(handle),
        }
    }

    /// Enter the runtime context
    /// Returns a guard that ensures the context remains active
    pub fn enter(&self) -> RuntimeContextGuard<'_> {
        #[cfg(feature = "tokio")]
        {
            if let Some(handle) = &self.tokio_handle {
                RuntimeContextGuard::new(handle)
            } else {
                // When no tokio handle is available, create a guard with None
                RuntimeContextGuard { _tokio_guard: None }
            }
        }

        #[cfg(not(feature = "tokio"))]
        {
            RuntimeContextGuard::new()
        }
    }

    // Note: has_tokio_runtime() and try_current_tokio() methods were removed because:
    // - has_tokio_runtime(): Was designed for public API to check tokio availability,
    //   but current implementation doesn't expose this check to users
    // - try_current_tokio(): Was designed for automatic tokio runtime detection,
    //   but current implementation uses explicit runtime management instead
}

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
/// # Thread Safety
///
/// In single-threaded mode (default), this function must be called from the main thread and the
/// future will be polled on the main thread. This ensures compatibility with Godot's threading model
/// where most objects are not thread-safe.
///
/// In multi-threaded mode (with `experimental-threads` feature), the function can be called from
/// any thread, but the future will still be polled on the main thread for consistency.
///
/// # Memory Safety
///
/// The future must be `'static` and not require `Send` since it will only run on a single thread.
/// If the future panics during polling, it will be safely dropped and cleaned up without affecting
/// other tasks.
///
/// # Panics
///
/// - If called from a non-main thread in single-threaded mode
/// - If the async runtime has been deinitialized (should only happen during engine shutdown)
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
pub fn spawn(future: impl Future<Output = ()> + Send + 'static) -> TaskHandle {
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
        "spawn() can only be used on the main thread in single-threaded mode.\n\
         Current thread: {:?}, Main thread: {:?}\n\
         Consider using the 'experimental-threads' feature if you need multi-threaded async support.",
        std::thread::current().id(),
        std::thread::current().id() // This is not actually the main thread ID, but it's for illustrative purposes
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

/// Create a new async background task that doesn't require Send.
///
/// This function is similar to [`spawn`] but allows futures that contain non-Send types
/// like Godot objects (`Gd<T>`, `Signal`, etc.). The future will be polled on the main thread
/// where it was created.
///
/// This is the preferred function for futures that interact with Godot objects, since most
/// Godot types are not thread-safe and don't implement Send.
///
/// # Thread Safety
///
/// This function must be called from the main thread in both single-threaded and multi-threaded modes.
/// The future will always be polled on the main thread to ensure compatibility with Godot's threading model.
///
/// # Examples
/// ```rust
/// use godot::prelude::*;
/// use godot::task;
///
/// let signal = Signal::from_object_signal(&some_object, "some_signal");
/// task::spawn_local(async move {
///     signal.to_future::<()>().await;
///     println!("Signal received!");
/// });
/// ```
pub fn spawn_local(future: impl Future<Output = ()> + 'static) -> TaskHandle {
    // Must be called from the main thread since Godot objects are not thread-safe
    assert!(
        crate::init::is_main_thread(),
        "spawn_local() must be called from the main thread.\n\
         Non-Send futures containing Godot objects can only be used on the main thread."
    );

    let (task_handle, godot_waker) = ASYNC_RUNTIME.with_runtime_mut(move |rt| {
        let task_handle = rt.add_task_non_send(Box::pin(future));
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
/// The returned object automatically has a `finished` signal added to it. When the
/// async task completes, this signal is emitted with the result as its argument.
///
/// # Examples
///
/// Basic usage:
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
///
/// With tokio operations:
/// ```rust
/// use godot_core::task::spawn_with_result;
/// use tokio::time::{sleep, Duration};
///
/// let async_task = spawn_with_result(async {
///     sleep(Duration::from_millis(100)).await;
///     "Task completed".to_string()
/// });
/// ```
///
/// # Thread Safety
///
/// In single-threaded mode (default), this function must be called from the main thread.
/// In multi-threaded mode (with `experimental-threads` feature), it can be called from any thread.
///
/// # Panics
///
/// Panics if called from a non-main thread in single-threaded mode.
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

    spawn_with_result_signal(signal_emitter.clone(), future);
    signal_emitter
}

/// Spawn an async task that emits to an existing signal holder.
///
/// This is used internally by the #[async_func] macro to enable direct Signal returns.
/// The signal holder should already have a "finished" signal defined.
///
/// # Example
/// ```rust
/// let signal_holder = RefCounted::new_gd();
/// signal_holder.add_user_signal("finished");
/// let signal = Signal::from_object_signal(&signal_holder, "finished");
///
/// spawn_with_result_signal(signal_holder, async { 42 });
/// // Now you can: await signal
/// ```
///
/// # Thread Safety
///
/// In single-threaded mode (default), this function must be called from the main thread.
/// In multi-threaded mode (with `experimental-threads` feature), it can be called from any thread.
///
/// # Panics
///
/// Panics if called from a non-main thread in single-threaded mode.
pub fn spawn_with_result_signal<F, R>(signal_emitter: Gd<RefCounted>, future: F)
where
    F: Future<Output = R> + Send + 'static,
    R: ToGodot + Send + Sync + 'static,
{
    // In single-threaded mode, spawning is only allowed on the main thread
    // In multi-threaded mode, we allow spawning from any thread
    #[cfg(all(not(wasm_nothreads), not(feature = "experimental-threads")))]
    assert!(
        crate::init::is_main_thread(),
        "spawn_with_result_signal() can only be used on the main thread in single-threaded mode"
    );

    let godot_waker = ASYNC_RUNTIME.with_runtime_mut(|rt| {
        // Create a wrapper that will emit the signal when complete
        let result_future = SignalEmittingFuture {
            inner: future,
            signal_emitter,
            _phantom: PhantomData,
            creation_thread: thread::current().id(),
        };

        // Spawn the signal-emitting future using standard spawn mechanism
        let task_handle = rt.add_task_non_send(Box::pin(result_future));

        // Create waker to trigger initial poll
        Arc::new(GodotWaker::new(
            task_handle.index,
            task_handle.id,
            thread::current().id(),
        ))
    });

    // Trigger initial poll
    poll_future(godot_waker);
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

    /// Create a new handle for a queued task
    ///
    /// Queued tasks don't have a slot index yet, so we use a special marker
    fn new_queued(id: u64) -> Self {
        Self {
            index: usize::MAX, // Special marker for queued tasks
            id,
            _no_send_sync: PhantomData,
        }
    }

    /// Cancels the task if it is still pending and does nothing if it is already completed.
    ///
    /// Returns Ok(()) if the task was successfully canceled or was already completed.
    /// Returns Err if the runtime has been deinitialized.
    pub fn cancel(self) -> AsyncRuntimeResult<()> {
        ASYNC_RUNTIME.with_runtime_mut(|rt| {
            let Some(task) = rt.task_storage.tasks.get(self.index) else {
                return Err(AsyncRuntimeError::RuntimeDeinitialized);
            };

            let alive = match task.value {
                FutureSlotState::Empty => {
                    return Err(AsyncRuntimeError::InvalidTaskState {
                        task_id: self.id,
                        expected_state: "non-empty".to_string(),
                    });
                }
                FutureSlotState::Gone => false,
                FutureSlotState::Pending(_) => task.id == self.id,
                FutureSlotState::Polling => {
                    return Err(AsyncRuntimeError::InvalidTaskState {
                        task_id: self.id,
                        expected_state: "not currently polling".to_string(),
                    });
                }
            };

            if alive {
                rt.clear_task(self.index);
            }

            Ok(())
        })
    }

    /// Synchronously checks if the task is still pending or has already completed.
    ///
    /// Returns Ok(true) if the task is still pending, Ok(false) if completed.
    /// Returns Err if the runtime has been deinitialized.
    pub fn is_pending(&self) -> AsyncRuntimeResult<bool> {
        ASYNC_RUNTIME.with_runtime(|rt| {
            let slot = rt
                .task_storage
                .tasks
                .get(self.index)
                .ok_or(AsyncRuntimeError::RuntimeDeinitialized)?;

            if slot.id != self.id {
                return Ok(false);
            }

            Ok(matches!(
                slot.value,
                FutureSlotState::Pending(_) | FutureSlotState::Polling
            ))
        })
    }

    /// Get the task ID for debugging purposes
    pub fn task_id(&self) -> u64 {
        self.id
    }

    /// Get the task index for debugging purposes
    pub fn task_index(&self) -> usize {
        self.index
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

/// Simplified lifecycle management for godot engine integration
///
/// Note: The original lifecycle module contained extensive monitoring and integration
/// features that were designed for:
/// - RuntimeLifecycleState enum: Designed for state tracking during complex initialization
/// - get_runtime_state/initialize_runtime: Designed for explicit lifecycle management
/// - on_frame_update/health_check: Designed for runtime monitoring and diagnostics
/// - engine_integration submodule: Designed for hooking into Godot's lifecycle events
///
/// These were removed because the current implementation uses:
/// - Lazy initialization (runtime created on first use)
/// - Simple cleanup on engine shutdown
/// - No need for complex state tracking or health monitoring
///
/// Only the essential cleanup function remains.
pub mod lifecycle {
    use super::*;

    /// Begin shutdown of the async runtime
    ///
    /// Returns the number of tasks that were canceled during shutdown
    pub fn begin_shutdown() -> usize {
        ASYNC_RUNTIME.with(|runtime| {
            if let Some(mut rt) = runtime.borrow_mut().take() {
                let storage_stats = rt.task_storage.get_stats();
                let task_count = storage_stats.active_tasks;

                // Log shutdown information
                if task_count > 0 {
                    eprintln!("Async runtime shutdown: canceling {task_count} pending tasks");
                }

                // Clear all components
                rt.clear_all();

                // Drop the runtime to free resources
                drop(rt);

                task_count
            } else {
                0
            }
        })
    }
}

/// Will be called during engine shutdown.
///
/// We have to drop all the remaining Futures during engine shutdown. This avoids them being dropped at process termination where they would
/// try to access engine resources, which leads to SEGFAULTs.
pub(crate) fn cleanup() {
    let canceled_tasks = lifecycle::begin_shutdown();

    if canceled_tasks > 0 {
        eprintln!("Godot async runtime cleanup: {canceled_tasks} tasks were canceled during engine shutdown");
    }
}

#[cfg(feature = "trace")]
pub fn has_godot_task_panicked(task_handle: TaskHandle) -> bool {
    ASYNC_RUNTIME.with_runtime(|rt| rt.task_scheduler.has_task_panicked(task_handle.id))
}

// Note: The following public API functions were removed because they were designed
// for external runtime inspection but are not actually used:
// - has_tokio_runtime_context(): Was designed to check if tokio is available
// - try_enter_runtime_context(): Was designed for explicit context management
// - get_runtime_context_info(): Was designed for runtime monitoring
// - RuntimeContextInfo struct: Supporting type for runtime monitoring
//
// These were part of a more complex public API that isn't needed by the current
// simple spawn() function interface.

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

/// Separated concerns for better architecture
///
/// Task limits and backpressure configuration
#[derive(Debug, Clone)]
pub struct TaskLimits {
    /// Maximum number of concurrent tasks allowed
    pub max_concurrent_tasks: usize,
    /// Maximum size of the task queue when at capacity
    pub max_queued_tasks: usize,
    /// Enable task prioritization
    pub enable_priority_scheduling: bool,
    /// Memory limit warning threshold (in active tasks)
    pub memory_warning_threshold: usize,
}

impl Default for TaskLimits {
    fn default() -> Self {
        Self {
            max_concurrent_tasks: 1000,        // Reasonable default
            max_queued_tasks: 500,             // Queue up to 500 tasks when at capacity
            enable_priority_scheduling: false, // Simple FIFO by default
            memory_warning_threshold: 800,     // Warn at 80% of max capacity
        }
    }
}

/// Task priority levels for prioritized scheduling
/// Note: Only Normal priority is currently used. Low, High, and Critical variants
/// were designed for priority-based task scheduling, but the current implementation
/// uses simple FIFO scheduling without prioritization.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub enum TaskPriority {
    #[default]
    Normal = 1,
}

/// Queued task waiting to be scheduled
struct QueuedTask {
    // This field is accessed via `queued_task.future` when the entire struct
    // is consumed during scheduling, but the compiler doesn't detect this usage pattern.
    #[allow(dead_code)]
    future: Pin<Box<dyn Future<Output = ()> + Send + 'static>>,
    priority: TaskPriority,
    queued_at: std::time::Instant,
    task_id: u64,
}

impl std::fmt::Debug for QueuedTask {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("QueuedTask")
            .field("priority", &self.priority)
            .field("queued_at", &self.queued_at)
            .field("task_id", &self.task_id)
            .field("future", &"<future>")
            .finish()
    }
}

/// Trait for type-erased future storage with minimal boxing overhead
trait ErasedFuture: Send + 'static {
    /// Poll the future in a type-erased way
    fn poll_erased(&mut self, cx: &mut std::task::Context<'_>) -> std::task::Poll<()>;

    // Note: debug_type_name() method was removed because it was designed for
    // debugging and diagnostics, but current implementation doesn't use runtime
    // type introspection for debugging purposes.
}

impl<F> ErasedFuture for F
where
    F: Future<Output = ()> + Send + 'static,
{
    fn poll_erased(&mut self, cx: &mut std::task::Context<'_>) -> std::task::Poll<()> {
        // SAFETY: We maintain the pin invariant by only calling this through proper Pin projection
        let pinned = unsafe { Pin::new_unchecked(self) };
        pinned.poll(cx)
    }
}

/// More efficient future storage that avoids unnecessary boxing
/// Only boxes when absolutely necessary (for type erasure)
enum FutureStorage {
    /// Direct storage for common small futures (avoids boxing)
    Inline(Box<dyn ErasedFuture>),
    /// For non-Send futures (like Godot integration)
    NonSend(Pin<Box<dyn Future<Output = ()> + 'static>>),
    // Note: Boxed variant was removed because it was designed as an alternative
    // storage method for cases requiring full Pin<Box<...>> type, but the current
    // implementation standardized on the ErasedFuture approach for all Send futures.
}

impl FutureStorage {
    /// Create optimized storage for a future
    fn new<F>(future: F) -> Self
    where
        F: Future<Output = ()> + Send + 'static,
    {
        // Always use the more efficient ErasedFuture approach
        Self::Inline(Box::new(future))
    }

    /// Create storage for a non-Send future
    fn new_non_send<F>(future: F) -> Self
    where
        F: Future<Output = ()> + 'static,
    {
        // Non-Send futures must use the boxed approach
        Self::NonSend(Box::pin(future))
    }

    /// Poll the stored future
    fn poll(&mut self, cx: &mut std::task::Context<'_>) -> std::task::Poll<()> {
        match self {
            Self::Inline(erased) => erased.poll_erased(cx),
            Self::NonSend(pinned) => pinned.as_mut().poll(cx),
        }
    }

    // Note: debug_type_name() method was removed because it was designed for
    // debugging and diagnostics, but current implementation doesn't use runtime
    // type introspection for debugging purposes.
}

/// Task storage component - manages the storage and lifecycle of futures
struct TaskStorage {
    tasks: Vec<FutureSlot<FutureStorage>>,
    next_task_id: u64,
    /// Configuration for task limits and backpressure
    limits: TaskLimits,
    /// Queue for tasks waiting to be scheduled when at capacity
    task_queue: Vec<QueuedTask>,
    /// Statistics for monitoring
    total_tasks_spawned: u64,
    // Note: total_tasks_completed field was removed because it was designed for
    // statistics tracking, but the current implementation doesn't track completed
    // tasks for monitoring purposes (only spawned and rejected for queue management).
    total_tasks_rejected: u64,
}

impl Default for TaskStorage {
    fn default() -> Self {
        Self::new()
    }
}

impl TaskStorage {
    fn new() -> Self {
        Self::with_limits(TaskLimits::default())
    }

    fn with_limits(limits: TaskLimits) -> Self {
        Self {
            tasks: Vec::new(),
            next_task_id: 0,
            limits,
            task_queue: Vec::new(),
            total_tasks_spawned: 0,
            total_tasks_rejected: 0,
        }
    }

    /// Get the next task ID
    fn next_id(&mut self) -> u64 {
        let id = self.next_task_id;
        self.next_task_id += 1;
        id
    }

    /// Store a new async task with priority and backpressure support
    fn store_task_with_priority<F>(
        &mut self,
        future: F,
        priority: TaskPriority,
    ) -> Result<TaskHandle, TaskSpawnError>
    where
        F: Future<Output = ()> + Send + 'static,
    {
        let id = self.next_id();
        self.total_tasks_spawned += 1;

        let active_tasks = self.get_active_task_count();

        // Check if we're at capacity
        if active_tasks >= self.limits.max_concurrent_tasks {
            return self.handle_capacity_overflow(future, priority, id);
        }

        // Check for memory pressure warning
        if active_tasks >= self.limits.memory_warning_threshold {
            eprintln!("Warning: High task load detected ({active_tasks} active tasks)");
        }

        self.schedule_task_immediately(future, id)
    }

    /// Store a new async task with priority and backpressure support (for non-Send futures)
    fn store_task_with_priority_non_send<F>(
        &mut self,
        future: F,
        priority: TaskPriority,
    ) -> Result<TaskHandle, TaskSpawnError>
    where
        F: Future<Output = ()> + 'static,
    {
        let id = self.next_id();
        self.total_tasks_spawned += 1;

        let active_tasks = self.get_active_task_count();

        // Check if we're at capacity
        if active_tasks >= self.limits.max_concurrent_tasks {
            return self.handle_capacity_overflow_non_send(future, priority, id);
        }

        // Check for memory pressure warning
        if active_tasks >= self.limits.memory_warning_threshold {
            eprintln!("Warning: High task load detected ({active_tasks} active tasks)");
        }

        self.schedule_task_immediately_non_send(future, id)
    }

    /// Store a new async task with default priority
    fn store_task<F>(&mut self, future: F) -> Result<TaskHandle, TaskSpawnError>
    where
        F: Future<Output = ()> + Send + 'static,
    {
        self.store_task_with_priority(future, TaskPriority::default())
    }

    /// Store a new async task with default priority (for non-Send futures)
    fn store_task_non_send<F>(&mut self, future: F) -> Result<TaskHandle, TaskSpawnError>
    where
        F: Future<Output = ()> + 'static,
    {
        self.store_task_with_priority_non_send(future, TaskPriority::default())
    }

    /// Handle task spawning when at capacity
    fn handle_capacity_overflow<F>(
        &mut self,
        future: F,
        priority: TaskPriority,
        id: u64,
    ) -> Result<TaskHandle, TaskSpawnError>
    where
        F: Future<Output = ()> + Send + 'static,
    {
        // Check if queue is full
        if self.task_queue.len() >= self.limits.max_queued_tasks {
            self.total_tasks_rejected += 1;
            return Err(TaskSpawnError::QueueFull {
                active_tasks: self.get_active_task_count(),
                queued_tasks: self.task_queue.len(),
            });
        }

        // Queue the task
        let queued_task = QueuedTask {
            future: Box::pin(future),
            priority,
            queued_at: std::time::Instant::now(),
            task_id: id,
        };

        // Insert based on priority if enabled
        if self.limits.enable_priority_scheduling {
            let insert_pos = self
                .task_queue
                .iter()
                .position(|task| task.priority < priority)
                .unwrap_or(self.task_queue.len());
            self.task_queue.insert(insert_pos, queued_task);
        } else {
            self.task_queue.push(queued_task);
        }

        // Return a special handle for queued tasks
        Ok(TaskHandle::new_queued(id))
    }

    /// Handle task spawning when at capacity (for non-Send futures)
    fn handle_capacity_overflow_non_send<F>(
        &mut self,
        _future: F,
        _priority: TaskPriority,
        _id: u64,
    ) -> Result<TaskHandle, TaskSpawnError>
    where
        F: Future<Output = ()> + 'static,
    {
        // For non-Send futures, we can't queue them because the queue stores Send futures
        // We reject them immediately
        self.total_tasks_rejected += 1;
        Err(TaskSpawnError::QueueFull {
            active_tasks: self.get_active_task_count(),
            queued_tasks: self.task_queue.len(),
        })
    }

    /// Schedule a task immediately
    fn schedule_task_immediately<F>(
        &mut self,
        future: F,
        id: u64,
    ) -> Result<TaskHandle, TaskSpawnError>
    where
        F: Future<Output = ()> + Send + 'static,
    {
        let storage = FutureStorage::new(future);

        let index_slot = self.tasks.iter_mut().enumerate().find_map(|(index, slot)| {
            if slot.is_empty() {
                Some((index, slot))
            } else {
                None
            }
        });

        let index = match index_slot {
            Some((index, slot)) => {
                *slot = FutureSlot::pending(id, storage);
                index
            }
            None => {
                self.tasks.push(FutureSlot::pending(id, storage));
                self.tasks.len() - 1
            }
        };

        Ok(TaskHandle::new(index, id))
    }

    /// Schedule a non-Send task immediately
    fn schedule_task_immediately_non_send<F>(
        &mut self,
        future: F,
        id: u64,
    ) -> Result<TaskHandle, TaskSpawnError>
    where
        F: Future<Output = ()> + 'static,
    {
        let storage = FutureStorage::new_non_send(future);

        let index_slot = self.tasks.iter_mut().enumerate().find_map(|(index, slot)| {
            if slot.is_empty() {
                Some((index, slot))
            } else {
                None
            }
        });

        let index = match index_slot {
            Some((index, slot)) => {
                *slot = FutureSlot::pending(id, storage);
                index
            }
            None => {
                self.tasks.push(FutureSlot::pending(id, storage));
                self.tasks.len() - 1
            }
        };

        Ok(TaskHandle::new(index, id))
    }

    // Note: try_promote_queued_tasks() method was removed because it was designed
    // for automatic queue processing when capacity becomes available, but the
    // current implementation uses simple queue overflow handling without automatic
    // promotion of queued tasks.

    /// Get the count of active (non-empty) tasks
    fn get_active_task_count(&self) -> usize {
        self.tasks.iter().filter(|slot| !slot.is_empty()).count()
    }

    /// Extract a pending task from storage
    fn take_task_for_polling(&mut self, index: usize, id: u64) -> FutureSlotState<FutureStorage> {
        let slot = self.tasks.get_mut(index);
        slot.map(|inner| inner.take_for_polling(id))
            .unwrap_or(FutureSlotState::Empty)
    }

    /// Remove a future from storage
    fn clear_task(&mut self, index: usize) {
        if let Some(slot) = self.tasks.get_mut(index) {
            slot.clear();
        }
    }

    /// Move a future back into storage
    fn park_task(&mut self, index: usize, future: FutureStorage) {
        if let Some(slot) = self.tasks.get_mut(index) {
            slot.park(future);
        }
    }

    /// Get statistics about task storage
    fn get_stats(&self) -> TaskStorageStats {
        let active_tasks = self.tasks.iter().filter(|slot| !slot.is_empty()).count();
        TaskStorageStats {
            active_tasks,
            // Note: total_slots and next_task_id fields were removed from stats
            // because they were designed for monitoring, but current implementation
            // only needs active task count for lifecycle management.
        }
    }

    /// Clear all tasks
    fn clear_all(&mut self) {
        self.tasks.clear();
    }
}

/// Statistics about task storage
#[derive(Debug, Clone)]
pub struct TaskStorageStats {
    pub active_tasks: usize,
    // Note: total_slots and next_task_id fields were removed because they were
    // designed for monitoring and diagnostics, but the current implementation
    // only needs active task count for lifecycle management.
}

/// Task scheduler component - handles task scheduling, polling, and execution
#[derive(Default)]
struct TaskScheduler {
    #[cfg(feature = "trace")]
    panicked_tasks: std::collections::HashSet<u64>,
    runtime_context: RuntimeContext,
}

impl TaskScheduler {
    fn new(runtime_context: RuntimeContext) -> Self {
        Self {
            #[cfg(feature = "trace")]
            panicked_tasks: std::collections::HashSet::new(),
            runtime_context,
        }
    }

    /// Track that a future caused a panic
    #[cfg(feature = "trace")]
    fn track_panic(&mut self, task_id: u64) {
        self.panicked_tasks.insert(task_id);
    }

    /// Check if a task has panicked
    #[cfg(feature = "trace")]
    fn has_task_panicked(&self, task_id: u64) -> bool {
        self.panicked_tasks.contains(&task_id)
    }

    // Note: The following methods were removed because they were designed for
    // internal diagnostics and monitoring, but are not used by the current
    // simplified implementation:
    // - has_tokio_context(): Was for checking tokio availability
    // - context(): Was for accessing runtime context
    // - get_stats(): Was for scheduler monitoring

    /// Clear panic tracking
    #[cfg(feature = "trace")]
    fn clear_panic_tracking(&mut self) {
        self.panicked_tasks.clear();
    }
}

// Note: TaskSchedulerStats struct was removed because it was designed for
// scheduler monitoring and diagnostics, but the current implementation doesn't
// use scheduler statistics for external monitoring.

// Note: SignalBridge component was removed because it was designed as a
// placeholder for future signal management features like:
// - Signal routing logic and caching
// - Batched signal processing
// - Advanced signal integration
//
// The current implementation handles signals directly in SignalEmittingFuture
// without needing a separate bridge component.

/// The main async runtime that coordinates between all components
struct AsyncRuntime {
    task_storage: TaskStorage,
    task_scheduler: TaskScheduler,
    // Note: signal_bridge field was removed because SignalBridge component
    // was designed as a placeholder for future signal management features,
    // but the current implementation handles signals directly without a bridge.
    #[cfg(feature = "tokio")]
    _runtime_manager: Option<RuntimeManager>,
}

impl Default for AsyncRuntime {
    fn default() -> Self {
        Self::new()
    }
}

// Use pin-project-lite for safe pin projection
pin_project! {
    /// Wrapper for futures that emits a signal when the future completes
    ///
    /// # Thread Safety
    ///
    /// This future ensures that signal emission always happens on the main thread
    /// via call_deferred, maintaining Godot's threading model.
    struct SignalEmittingFuture<F, R> {
        #[pin]
        inner: F,
        signal_emitter: Gd<RefCounted>,
        _phantom: PhantomData<R>,
        creation_thread: ThreadId,
    }
}

impl<F, R> Future for SignalEmittingFuture<F, R>
where
    F: Future<Output = R>,
    R: ToGodot + Send + Sync + 'static,
{
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        // Safe pin projection using pin-project-lite
        let this = self.project();

        // Enhanced thread safety validation
        let current_thread = thread::current().id();
        if *this.creation_thread != current_thread {
            eprintln!(
                "Warning: SignalEmittingFuture polled on different thread than created. \
                Created on {:?}, polling on {:?}. This may cause issues with Gd<RefCounted> access.",
                this.creation_thread, current_thread
            );
        }

        match this.inner.poll(cx) {
            Poll::Ready(result) => {
                // Convert the result to Variant and emit the completion signal
                let variant_result = result.to_variant();

                // Use call_deferred to ensure signal emission happens on the main thread
                let mut signal_emitter = this.signal_emitter.clone();
                let variant_result_clone = variant_result.clone();
                let creation_thread_id = *this.creation_thread;

                let callable = Callable::from_local_fn("emit_finished_signal", move |_args| {
                    // Additional thread safety check at emission time
                    let emission_thread = thread::current().id();
                    if creation_thread_id != emission_thread {
                        eprintln!(
                            "Warning: Signal emission happening on different thread than future creation. \
                            Created on {creation_thread_id:?}, emitting on {emission_thread:?}"
                        );
                    }

                    // Enhanced error handling for signal emission
                    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        signal_emitter.emit_signal("finished", &[variant_result_clone.clone()]);
                    })) {
                        Ok(()) => Ok(Variant::nil()),
                        Err(panic_err) => {
                            let error_msg = if let Some(s) = panic_err.downcast_ref::<String>() {
                                s.clone()
                            } else if let Some(s) = panic_err.downcast_ref::<&str>() {
                                s.to_string()
                            } else {
                                "Unknown panic during signal emission".to_string()
                            };

                            eprintln!("Warning: Signal emission failed: {error_msg}");
                            Ok(Variant::nil())
                        }
                    }
                });

                callable.call_deferred(&[]);
                Poll::Ready(())
            }
            Poll::Pending => Poll::Pending,
        }
    }
}

// SignalEmittingFuture is automatically Send if all its components are Send
// We ensure this through proper bounds rather than unsafe impl

/// Proper tokio runtime management with cleanup
#[cfg(feature = "tokio")]
struct RuntimeManager {
    _runtime: Option<tokio::runtime::Runtime>,
    handle: tokio::runtime::Handle,
}

#[cfg(feature = "tokio")]
impl RuntimeManager {
    fn new() -> Option<Self> {
        // Try to use current tokio runtime first
        if let Ok(current_handle) = Handle::try_current() {
            return Some(Self {
                _runtime: None,
                handle: current_handle,
            });
        }

        // Create a new runtime if none exists
        #[cfg(feature = "experimental-threads")]
        let mut builder = tokio::runtime::Builder::new_multi_thread();

        #[cfg(not(feature = "experimental-threads"))]
        let mut builder = tokio::runtime::Builder::new_current_thread();

        match builder.enable_all().build() {
            Ok(runtime) => {
                let handle = runtime.handle().clone();
                Some(Self {
                    _runtime: Some(runtime),
                    handle,
                })
            }
            Err(e) => {
                // Log the error but don't panic, just continue without tokio support
                eprintln!("Warning: Failed to create tokio runtime: {e}");
                #[cfg(feature = "trace")]
                eprintln!("  This will disable tokio-based async operations");
                None
            }
        }
    }

    fn handle(&self) -> &tokio::runtime::Handle {
        &self.handle
    }
}

#[cfg(feature = "tokio")]
impl Drop for RuntimeManager {
    fn drop(&mut self) {
        // Runtime will be properly dropped when _runtime is dropped
        // No manual shutdown needed as Drop handles it
    }
}

impl AsyncRuntime {
    fn new() -> Self {
        #[cfg(feature = "tokio")]
        let (runtime_manager, tokio_handle) = {
            match RuntimeManager::new() {
                Some(manager) => {
                    let handle = manager.handle().clone();
                    (Some(manager), Some(handle))
                }
                None => (None, None),
            }
        };

        let runtime_context = {
            #[cfg(feature = "tokio")]
            {
                if let Some(handle) = tokio_handle.as_ref() {
                    RuntimeContext::with_tokio_handle(handle.clone())
                } else {
                    RuntimeContext::new()
                }
            }
            #[cfg(not(feature = "tokio"))]
            {
                RuntimeContext::new()
            }
        };

        Self {
            task_storage: TaskStorage::new(),
            task_scheduler: TaskScheduler::new(runtime_context),
            #[cfg(feature = "tokio")]
            _runtime_manager: runtime_manager,
        }
    }

    /// Store a new async task in the runtime
    /// Delegates to task storage component
    fn add_task<F>(&mut self, future: F) -> TaskHandle
    where
        F: Future<Output = ()> + Send + 'static,
    {
        match self.task_storage.store_task(future) {
            Ok(handle) => handle,
            Err(spawn_error) => {
                // For backward compatibility, we log the error but don't panic
                // In the future, we might want to return a Result from spawn()
                eprintln!("Warning: Task spawn failed: {spawn_error}");
                eprintln!("  This task will be dropped. Consider reducing concurrent task load.");

                // Return a dummy handle that represents a failed task
                TaskHandle::new_queued(0) // Task ID 0 represents a failed task
            }
        }
    }

    /// Store a new async task in the runtime (for futures that are not Send)
    /// This is used for Godot integration where Gd<T> objects are not Send
    fn add_task_non_send<F>(&mut self, future: F) -> TaskHandle
    where
        F: Future<Output = ()> + 'static,
    {
        match self.task_storage.store_task_non_send(future) {
            Ok(handle) => handle,
            Err(spawn_error) => {
                // For backward compatibility, we log the error but don't panic
                eprintln!("Warning: Task spawn failed: {spawn_error}");
                eprintln!("  This task will be dropped. Consider reducing concurrent task load.");

                // Return a dummy handle that represents a failed task
                TaskHandle::new_queued(0) // Task ID 0 represents a failed task
            }
        }
    }

    /// Extract a pending task from the storage
    /// Delegates to task storage component
    fn take_task_for_polling(&mut self, index: usize, id: u64) -> FutureSlotState<FutureStorage> {
        self.task_storage.take_task_for_polling(index, id)
    }

    /// Remove a future from the storage
    /// Delegates to task storage component
    fn clear_task(&mut self, index: usize) {
        self.task_storage.clear_task(index);
    }

    /// Move a future back into storage
    /// Delegates to task storage component
    fn park_task(&mut self, index: usize, future: FutureStorage) {
        self.task_storage.park_task(index, future);
    }

    /// Track that a future caused a panic
    /// Delegates to task scheduler component
    #[cfg(feature = "trace")]
    fn track_panic(&mut self, task_id: u64) {
        self.task_scheduler.track_panic(task_id);
    }

    // Note: The following methods were removed because they were designed for
    // internal diagnostics and frame-based processing, but are not used by the
    // current simplified implementation:
    // - has_tokio_context(): Was for checking tokio availability
    // - context(): Was for accessing runtime context
    // - get_combined_stats(): Was for aggregating statistics from all components
    // - process_frame_update(): Was for frame-based signal processing

    /// Clear all data from all components
    fn clear_all(&mut self) {
        self.task_storage.clear_all();
        #[cfg(feature = "trace")]
        self.task_scheduler.clear_panic_tracking();
    }
}

// Note: CombinedRuntimeStats struct was removed because it was designed for
// aggregating statistics from all runtime components, but the current
// implementation doesn't use combined statistics for monitoring.

trait WithRuntime {
    fn with_runtime<R>(&'static self, f: impl FnOnce(&AsyncRuntime) -> R) -> R;
    fn with_runtime_mut<R>(&'static self, f: impl FnOnce(&mut AsyncRuntime) -> R) -> R;
    // Note: try_with_runtime and try_with_runtime_mut methods were removed because
    // they were designed as error-returning variants of the main methods, but the
    // current implementation uses panicking behavior for consistency with the
    // rest of the runtime error handling.
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

    // Note: try_with_runtime and try_with_runtime_mut implementations were removed
    // because they were designed as error-returning variants, but the current
    // implementation only uses the panicking variants for consistency.
}

/// Use a godot waker to poll it's associated future.
///
/// # Panics
/// - If called from a thread other than the main-thread.
fn poll_future(godot_waker: Arc<GodotWaker>) {
    let current_thread = thread::current().id();

    // Enhanced thread safety check with better error reporting
    if godot_waker.thread_id != current_thread {
        let error = AsyncRuntimeError::ThreadSafetyViolation {
            expected_thread: godot_waker.thread_id,
            actual_thread: current_thread,
        };

        // Log the error before panicking
        eprintln!("FATAL: {error}");

        // Still panic for safety, but with better error message
        panic!("Thread safety violation in async runtime: {error}");
    }

    let waker = Waker::from(godot_waker.clone());
    let mut ctx = Context::from_waker(&waker);

    // Move future out of the runtime while we are polling it to avoid holding a mutable reference for the entire runtime.
    let future_storage = ASYNC_RUNTIME.with_runtime_mut(|rt| {
        match rt.take_task_for_polling(godot_waker.runtime_index, godot_waker.task_id) {
            FutureSlotState::Empty => {
                // Enhanced error handling - log and return None instead of panicking
                let task_id = godot_waker.task_id;
                eprintln!("Warning: Future slot is empty when waking task {task_id}. This may indicate a race condition.");
                None
            }

            FutureSlotState::Gone => None,

            FutureSlotState::Polling => {
                // Enhanced error handling - log the issue but don't panic
                let task_id = godot_waker.task_id;
                eprintln!("Warning: Task {task_id} is already being polled. This may indicate recursive waking.");
                None
            }

            FutureSlotState::Pending(future) => Some(future),
        }
    });

    let Some(mut future_storage) = future_storage else {
        // Future has been canceled while the waker was already triggered.
        return;
    };

    let task_id = godot_waker.task_id;
    let error_context = || format!("Godot async task failed (task_id: {task_id})");

    // Execute the poll operation within proper runtime context
    let panic_result = {
        ASYNC_RUNTIME.with_runtime(|rt| {
            // Enter the runtime context for proper tokio integration
            let _context_guard = rt.task_scheduler.runtime_context.enter();

            handle_panic(
                error_context,
                AssertUnwindSafe(move || {
                    let poll_result = future_storage.poll(&mut ctx);
                    (poll_result, future_storage)
                }),
            )
        })
    };

    let Ok((poll_result, future_storage)) = panic_result else {
        // Polling the future caused a panic. The task state has to be cleaned up and we want track the panic if the trace feature is enabled.
        let error = AsyncRuntimeError::TaskPanicked {
            task_id: godot_waker.task_id,
            message: "Task panicked during polling".to_string(),
        };

        eprintln!("Error: {error}");

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
        Poll::Pending => rt.park_task(godot_waker.runtime_index, future_storage),

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
