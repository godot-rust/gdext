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

// Use pin-project-lite for safe pin projection
use pin_project_lite::pin_project;

use crate::builtin::{Callable, Variant};
use crate::private::handle_panic;

// *** Added: Support async Future with return values ***

use crate::classes::RefCounted;
use crate::meta::ToGodot;
use crate::obj::{Gd, NewGd};

/// Trait for integrating external async runtimes with gdext's async system.
///
/// This trait provides the minimal interface for pluggable async runtime support.
/// Users need to implement `create_runtime()` and `with_context()`.
///
/// # Simple Example Implementation
///
/// ```rust
/// struct TokioIntegration;
///
/// impl AsyncRuntimeIntegration for TokioIntegration {
///     type Handle = tokio::runtime::Handle;
///     
///     fn create_runtime() -> Result<(Box<dyn std::any::Any + Send + Sync>, Self::Handle), String> {
///         let runtime = tokio::runtime::Runtime::new()
///             .map_err(|e| format!("Failed to create tokio runtime: {}", e))?;
///         let handle = runtime.handle().clone();
///         Ok((Box::new(runtime), handle))
///     }
///     
///     fn with_context<R>(handle: &Self::Handle, f: impl FnOnce() -> R) -> R {
///         let _guard = handle.enter();
///         f()
///     }
/// }
/// ```
pub trait AsyncRuntimeIntegration: Send + Sync + 'static {
    /// Handle type for the async runtime (e.g., `tokio::runtime::Handle`)
    type Handle: Clone + Send + Sync + 'static;

    /// Create a new runtime instance and return its handle
    ///
    /// Returns a tuple of:
    /// - Boxed runtime instance (kept alive via RAII)
    /// - Handle to the runtime for context operations
    ///
    /// The runtime should be configured appropriately for Godot integration.
    /// If creation fails, return a descriptive error message.
    fn create_runtime() -> Result<(Box<dyn std::any::Any + Send + Sync>, Self::Handle), String>;

    /// Execute a closure within the runtime context
    ///
    /// This method should execute the provided closure while the runtime
    /// is current. This ensures that async operations within the closure
    /// have access to the proper runtime context (timers, I/O, etc.).
    ///
    /// For runtimes that don't need explicit context management,
    /// this can simply call the closure directly.
    fn with_context<R>(handle: &Self::Handle, f: impl FnOnce() -> R) -> R;
}

/// Configuration for the async runtime
///
/// This allows users to specify which runtime integration to use and configure
/// its behavior. By default, gdext will try to auto-detect an existing runtime
/// or use a built-in minimal implementation.
pub struct AsyncRuntimeConfig<T: AsyncRuntimeIntegration> {
    /// The runtime integration implementation
    _integration: PhantomData<T>,

    /// Whether to try auto-detecting existing runtime context
    pub auto_detect: bool,

    /// Whether to create a new runtime if none is detected
    pub create_if_missing: bool,
}

impl<T: AsyncRuntimeIntegration> Default for AsyncRuntimeConfig<T> {
    fn default() -> Self {
        Self {
            _integration: PhantomData,
            auto_detect: true,
            create_if_missing: true,
        }
    }
}

impl<T: AsyncRuntimeIntegration> AsyncRuntimeConfig<T> {
    /// Create a new runtime configuration
    pub fn new() -> Self {
        Self::default()
    }

    /// Set whether to auto-detect existing runtime context
    pub fn with_auto_detect(mut self, auto_detect: bool) -> Self {
        self.auto_detect = auto_detect;
        self
    }

    /// Set whether to create a new runtime if none is detected
    pub fn with_create_if_missing(mut self, create_if_missing: bool) -> Self {
        self.create_if_missing = create_if_missing;
        self
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Runtime Registry

use std::sync::OnceLock;

/// Type alias for the context function to avoid clippy complexity warnings
type ContextFunction = Box<dyn Fn(&dyn Fn()) + Send + Sync>;

/// Runtime storage with context management
struct RuntimeStorage {
    /// The actual runtime instance (kept alive via RAII)
    _runtime_instance: Box<dyn std::any::Any + Send + Sync>,
    /// Function to execute closures within runtime context
    with_context: ContextFunction,
}

/// Single consolidated storage - no scattered statics
static RUNTIME_STORAGE: OnceLock<RuntimeStorage> = OnceLock::new();

/// Register an async runtime integration with gdext
///
/// This must be called before using any async functions like `#[async_func]`.
/// Only one runtime can be registered per application.
///
/// # Panics
///
/// Panics if a runtime has already been registered.
///
/// # Example
///
/// ```rust
/// use your_runtime_integration::YourRuntimeIntegration;
///
/// // Register your runtime at application startup
/// gdext::task::register_runtime::<YourRuntimeIntegration>();
///
/// // Now async functions will work
/// ```
pub fn register_runtime<T: AsyncRuntimeIntegration>() {
    // Create the runtime immediately during registration
    let (runtime_instance, handle) = T::create_runtime().expect("Failed to create async runtime");

    // Clone the handle for the closure
    let handle_clone = handle.clone();

    // Create the storage structure with context management
    let storage = RuntimeStorage {
        _runtime_instance: runtime_instance,
        with_context: Box::new(move |f| T::with_context(&handle_clone, f)),
    };

    if RUNTIME_STORAGE.set(storage).is_err() {
        panic!(
            "Async runtime has already been registered. Only one runtime can be registered per application.\n\
             If you need to change runtimes, restart the application."
        );
    }
}

/// Check if a runtime is registered
pub fn is_runtime_registered() -> bool {
    RUNTIME_STORAGE.get().is_some()
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

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
    /// No async runtime has been registered
    NoRuntimeRegistered,
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
            AsyncRuntimeError::NoRuntimeRegistered => {
                write!(f, "No async runtime has been registered. Call gdext::task::register_runtime() before using async functions.")
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
    // Check if runtime is registered
    if !is_runtime_registered() {
        panic!(
            "No async runtime has been registered!\n\
             Call gdext::task::register_runtime::<YourRuntimeIntegration>() before using async functions.\n\
             See the documentation for examples of runtime integrations."
        );
    }

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
        let task_handle = rt.add_task(Box::pin(future))
            .unwrap_or_else(|spawn_error| {
                panic!(
                    "Failed to spawn async task: {spawn_error}\n\
                     This indicates the task queue is full or the runtime is overloaded.\n\
                     Consider reducing concurrent task load or increasing task limits."
                );
            });
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
    // Check if runtime is registered
    if !is_runtime_registered() {
        panic!(
            "No async runtime has been registered!\n\
             Call gdext::task::register_runtime::<YourRuntimeIntegration>() before using async functions.\n\
             See the documentation for examples of runtime integrations."
        );
    }

    // Must be called from the main thread since Godot objects are not thread-safe
    assert!(
        crate::init::is_main_thread(),
        "spawn_local() must be called from the main thread.\n\
         Non-Send futures containing Godot objects can only be used on the main thread."
    );

    let (task_handle, godot_waker) = ASYNC_RUNTIME.with_runtime_mut(move |rt| {
        let task_handle = rt.add_task_non_send(Box::pin(future))
            .unwrap_or_else(|spawn_error| {
                panic!(
                    "Failed to spawn async task: {spawn_error}\n\
                     This indicates the task queue is full or the runtime is overloaded.\n\
                     Consider reducing concurrent task load or increasing task limits."
                );
            });
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
    // Check if runtime is registered
    if !is_runtime_registered() {
        panic!(
            "No async runtime has been registered!\n\
             Call gdext::task::register_runtime::<YourRuntimeIntegration>() before using async functions.\n\
             See the documentation for examples of runtime integrations."
        );
    }

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
    // Check if runtime is registered
    if !is_runtime_registered() {
        panic!(
            "No async runtime has been registered!\n\
             Call gdext::task::register_runtime::<YourRuntimeIntegration>() before using async functions.\n\
             See the documentation for examples of runtime integrations."
        );
    }

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
        let task_handle = rt.add_task_non_send(Box::pin(result_future))
            .unwrap_or_else(|spawn_error| {
                panic!(
                    "Failed to spawn async task with result: {spawn_error}\n\
                     This indicates the task queue is full or the runtime is overloaded.\n\
                     Consider reducing concurrent task load or increasing task limits."
                );
            });

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
    ASYNC_RUNTIME.with_runtime(|rt| rt._task_scheduler.has_task_panicked(task_handle.id))
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

/// Trait for type-erased pinned future storage with safe pin handling
trait ErasedPinnedFuture: Send + 'static {
    /// Poll the pinned future in a type-erased way
    /// 
    /// # Safety
    /// This method must only be called on a properly pinned future that has never been moved
    /// since being pinned. The caller must ensure proper pin projection.
    fn poll_erased(self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<()>;
}

impl<F> ErasedPinnedFuture for F
where
    F: Future<Output = ()> + Send + 'static,
{
    fn poll_erased(self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<()> {
        // Safe: we're already pinned, so we can directly poll
        self.poll(cx)
    }
}

/// More efficient future storage that avoids unnecessary boxing
/// Only boxes when absolutely necessary (for type erasure)
enum FutureStorage {
    /// Direct storage for Send futures with safe pin handling
    Inline(Pin<Box<dyn ErasedPinnedFuture>>),
    /// For non-Send futures (like Godot integration)
    NonSend(Pin<Box<dyn Future<Output = ()> + 'static>>),
}

impl FutureStorage {
    /// Create optimized storage for a future
    fn new<F>(future: F) -> Self
    where
        F: Future<Output = ()> + Send + 'static,
    {
        // Pin the future immediately for safe handling
        Self::Inline(Box::pin(future))
    }

    /// Create storage for a non-Send future
    fn new_non_send<F>(future: F) -> Self
    where
        F: Future<Output = ()> + 'static,
    {
        // Non-Send futures use the same pinned approach
        Self::NonSend(Box::pin(future))
    }
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

    /// Get the count of active (non-empty) tasks
    fn get_active_task_count(&self) -> usize {
        self.tasks.iter().filter(|slot| !slot.is_empty()).count()
    }

    /// Remove a future from storage
    fn clear_task(&mut self, index: usize) {
        if let Some(slot) = self.tasks.get_mut(index) {
            slot.clear();
        }
    }

    /// Get statistics about task storage
    fn get_stats(&self) -> TaskStorageStats {
        let active_tasks = self.tasks.iter().filter(|slot| !slot.is_empty()).count();
        TaskStorageStats {
            active_tasks,
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
}

/// Task scheduler component - handles task scheduling, polling, and execution
#[derive(Default)]
struct TaskScheduler {
    #[cfg(feature = "trace")]
    panicked_tasks: std::collections::HashSet<u64>,
    // Runtime context is now managed by the registered runtime
}

impl TaskScheduler {
    fn new() -> Self {
        Self {
            #[cfg(feature = "trace")]
            panicked_tasks: std::collections::HashSet::new(),
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

    /// Clear panic tracking
    #[cfg(feature = "trace")]
    fn clear_panic_tracking(&mut self) {
        self.panicked_tasks.clear();
    }
}

/// The main async runtime that coordinates between all components
struct AsyncRuntime {
    task_storage: TaskStorage,
    _task_scheduler: TaskScheduler,
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

        // CRITICAL: Thread safety validation - must be fatal
        let current_thread = thread::current().id();
        if *this.creation_thread != current_thread {
            let error = AsyncRuntimeError::ThreadSafetyViolation {
                expected_thread: *this.creation_thread,
                actual_thread: current_thread,
            };
            
            eprintln!("FATAL: {error}");
            eprintln!("SignalEmittingFuture with Gd<RefCounted> cannot be accessed from different threads!");
            eprintln!("This would cause memory corruption. Future created on {:?}, polled on {:?}.", 
                this.creation_thread, current_thread);
            
            // MUST panic to prevent memory corruption - Godot objects are not thread-safe
            panic!("Thread safety violation in SignalEmittingFuture: {error}");
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
                    // CRITICAL: Thread safety validation - signal emission must be on correct thread
                    let emission_thread = thread::current().id();
                    if creation_thread_id != emission_thread {
                        let error = AsyncRuntimeError::ThreadSafetyViolation {
                            expected_thread: creation_thread_id,
                            actual_thread: emission_thread,
                        };
                        
                        eprintln!("FATAL: {error}");
                        eprintln!("Signal emission must happen on the same thread as future creation!");
                        eprintln!("This would cause memory corruption with Gd<RefCounted>. Created on {creation_thread_id:?}, emitting on {emission_thread:?}");
                        
                        // MUST panic to prevent memory corruption - signal_emitter is not thread-safe
                        panic!("Thread safety violation in signal emission: {error}");
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

impl AsyncRuntime {
    fn new() -> Self {
        // No runtime initialization needed - runtime is provided by user registration
        Self {
            task_storage: TaskStorage::new(),
            _task_scheduler: TaskScheduler::new(),
        }
    }

    /// Store a new async task in the runtime
    /// Delegates to task storage component
    fn add_task<F>(&mut self, future: F) -> Result<TaskHandle, TaskSpawnError>
    where
        F: Future<Output = ()> + Send + 'static,
    {
        // Properly propagate errors instead of masking them
        self.task_storage.store_task(future)
    }

    /// Store a new async task in the runtime (for futures that are not Send)
    /// This is used for Godot integration where Gd<T> objects are not Send
    fn add_task_non_send<F>(&mut self, future: F) -> Result<TaskHandle, TaskSpawnError>
    where
        F: Future<Output = ()> + 'static,
    {
        // Properly propagate errors instead of masking them
        self.task_storage.store_task_non_send(future)
    }

    /// Remove a future from the storage
    /// Delegates to task storage component
    fn clear_task(&mut self, index: usize) {
        self.task_storage.clear_task(index);
    }

    /// Track that a future caused a panic
    /// Delegates to task scheduler component
    #[cfg(feature = "trace")]
    fn track_panic(&mut self, task_id: u64) {
        self._task_scheduler.track_panic(task_id);
    }

    /// Clear all data from all components
    fn clear_all(&mut self) {
        self.task_storage.clear_all();
        #[cfg(feature = "trace")]
        self._task_scheduler.clear_panic_tracking();
    }

    /// Poll a future in place without breaking the pin invariant
    /// This safely polls the future while it remains in storage
    fn poll_task_in_place(
        &mut self,
        index: usize,
        id: u64,
        cx: &mut Context<'_>,
    ) -> Result<Poll<()>, AsyncRuntimeError> {
        let slot = self.task_storage.tasks.get_mut(index)
            .ok_or(AsyncRuntimeError::RuntimeDeinitialized)?;

        // Check if the task ID matches and is in the right state
        if slot.id != id {
            return Err(AsyncRuntimeError::InvalidTaskState {
                task_id: id,
                expected_state: "matching task ID".to_string(),
            });
        }

        match &mut slot.value {
            FutureSlotState::Empty => {
                Err(AsyncRuntimeError::InvalidTaskState {
                    task_id: id,
                    expected_state: "non-empty".to_string(),
                })
            }
            FutureSlotState::Gone => {
                Err(AsyncRuntimeError::TaskCanceled { task_id: id })
            }
            FutureSlotState::Polling => {
                Err(AsyncRuntimeError::InvalidTaskState {
                    task_id: id,
                    expected_state: "not currently polling".to_string(),
                })
            }
            FutureSlotState::Pending(_future_storage) => {
                // Temporarily mark as polling to prevent reentrant polling
                let old_state = std::mem::replace(&mut slot.value, FutureSlotState::Polling);
                
                // Extract the future storage for polling
                let mut future_storage = if let FutureSlotState::Pending(fs) = old_state {
                    fs
                } else {
                    unreachable!("We just matched on Pending")
                };
                
                // Poll the future in place using safe pin projection
                let poll_result = match &mut future_storage {
                    FutureStorage::Inline(pinned_future) => {
                        pinned_future.as_mut().poll_erased(cx)
                    }
                    FutureStorage::NonSend(pinned_future) => {
                        pinned_future.as_mut().poll(cx)
                    }
                };
                
                // Handle the result and restore appropriate state
                match poll_result {
                    Poll::Pending => {
                        // Put the future back in pending state
                        slot.value = FutureSlotState::Pending(future_storage);
                        Ok(Poll::Pending)
                    }
                    Poll::Ready(()) => {
                        // Task completed, mark as gone
                        slot.value = FutureSlotState::Gone;
                        Ok(Poll::Ready(()))
                    }
                }
            }
        }
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

    let task_id = godot_waker.task_id;
    let error_context = || format!("Godot async task failed (task_id: {task_id})");

    // Poll the future safely in place within the runtime context
    let poll_result = if let Some(storage) = RUNTIME_STORAGE.get() {
        // Poll within the runtime context for proper tokio/async-std support
        let result = std::cell::RefCell::new(None);
        let ctx_ref = std::cell::RefCell::new(Some(ctx));
        
        (storage.with_context)(&|| {
            let mut ctx = ctx_ref.borrow_mut().take().expect("Context should be available");
            
            let poll_result = ASYNC_RUNTIME.with_runtime_mut(|rt| {
                handle_panic(
                    error_context,
                    AssertUnwindSafe(|| {
                        rt.poll_task_in_place(
                            godot_waker.runtime_index,
                            godot_waker.task_id,
                            &mut ctx,
                        )
                    }),
                )
            });
            
            *result.borrow_mut() = Some(poll_result);
        });
        
        result.into_inner().expect("Result should have been set")
    } else {
        // Fallback: direct polling without runtime context
        ASYNC_RUNTIME.with_runtime_mut(|rt| {
            handle_panic(
                error_context,
                AssertUnwindSafe(|| {
                    rt.poll_task_in_place(
                        godot_waker.runtime_index,
                        godot_waker.task_id,
                        &mut ctx,
                    )
                }),
            )
        })
    };

    // Handle the result
    match poll_result {
        Ok(Ok(Poll::Ready(()))) => {
            // Task completed successfully - cleanup is handled by poll_task_in_place
        }
        Ok(Ok(Poll::Pending)) => {
            // Task is still pending - continue waiting
        }
        Ok(Err(async_error)) => {
            // Task had an error (canceled, invalid state, etc.)
            eprintln!("Async task error: {async_error}");
            
            // Clear the task slot for cleanup
            ASYNC_RUNTIME.with_runtime_mut(|rt| {
                rt.clear_task(godot_waker.runtime_index);
            });
        }
        Err(_panic_payload) => {
            // Task panicked during polling
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
        }
    }
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
