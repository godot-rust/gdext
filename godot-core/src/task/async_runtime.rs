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



// ----------------------------------------------------------------------------------------------------------------------------------------------
// Runtime Registry - Thread-Local Only (No Global State)

/// Type alias for the context function to avoid clippy complexity warnings
type ContextFunction = Box<dyn Fn(&dyn Fn()) + Send + Sync>;

/// Runtime storage with context management - now part of thread-local storage
struct RuntimeStorage {
    /// The actual runtime instance (kept alive via RAII)
    _runtime_instance: Box<dyn std::any::Any + Send + Sync>,
    /// Function to execute closures within runtime context
    with_context: ContextFunction,
}

/// Per-thread runtime registry - avoids global state
struct ThreadLocalRuntimeRegistry {
    /// Optional runtime storage for this thread
    runtime_storage: Option<RuntimeStorage>,
    /// Whether this thread has attempted runtime registration
    registration_attempted: bool,
}

thread_local! {
    /// Thread-local runtime registry - no global state needed
    static RUNTIME_REGISTRY: RefCell<ThreadLocalRuntimeRegistry> = const { RefCell::new(ThreadLocalRuntimeRegistry {
        runtime_storage: None,
        registration_attempted: false,
    }) };
}

/// Register an async runtime integration with gdext for the current thread
///
/// This must be called before using any async functions like `#[async_func]` on this thread.
/// Each thread can have its own runtime registration.
///
/// # Errors
///
/// Returns an error if a runtime has already been registered for this thread.
///
/// # Example
///
/// ```rust
/// use your_runtime_integration::YourRuntimeIntegration;
///
/// // Register your runtime at application startup
/// gdext::task::register_runtime::<YourRuntimeIntegration>()?;
///
/// // Now async functions will work on this thread
/// ```
pub fn register_runtime<T: AsyncRuntimeIntegration>() -> Result<(), String> {
    RUNTIME_REGISTRY.with(|registry| {
        let mut registry = registry.borrow_mut();

        if registry.registration_attempted {
            return Err("Async runtime has already been registered for this thread".to_string());
        }

        registry.registration_attempted = true;

        // Create the runtime immediately during registration
        let (runtime_instance, handle) = T::create_runtime()?;

        // Clone the handle for the closure
        let handle_clone = handle.clone();

        // Create the storage structure with context management
        let storage = RuntimeStorage {
            _runtime_instance: runtime_instance,
            with_context: Box::new(move |f| T::with_context(&handle_clone, f)),
        };

        registry.runtime_storage = Some(storage);
        Ok(())
    })
}

/// Check if a runtime is registered for the current thread
pub fn is_runtime_registered() -> bool {
    RUNTIME_REGISTRY.with(|registry| registry.borrow().runtime_storage.is_some())
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
        max_tasks: usize,
    },
}

impl std::fmt::Display for TaskSpawnError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TaskSpawnError::QueueFull { active_tasks, max_tasks } => {
                write!(f, "Task queue is full: {active_tasks}/{max_tasks} tasks")
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
/// Panics if:
/// - No async runtime has been registered
/// - The task queue is full and cannot accept more tasks
/// - Called from a non-main thread in single-threaded mode
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
/// let task = godot::task::spawn(async move {
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
/// let task = godot::task::spawn(async move {
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
        panic!("No async runtime has been registered. Call gdext::task::register_runtime() before using async functions.");
    }

    // In single-threaded mode, spawning is only allowed on the main thread.
    // We can not accept Sync + Send futures since all object references (i.e. Gd<T>) are not thread-safe. So a future has to remain on the
    // same thread it was created on. Godots signals on the other hand can be emitted on any thread, so it can't be guaranteed on which thread
    // a future will be polled.
    // By limiting async tasks to the main thread we can redirect all signal callbacks back to the main thread via `call_deferred`.
    //
    // In multi-threaded mode with experimental-threads, the restriction is lifted.
    #[cfg(all(not(wasm_nothreads), not(feature = "experimental-threads")))]
    if !crate::init::is_main_thread() {
        panic!("Async tasks can only be spawned on the main thread. Expected thread: {:?}, current thread: {:?}", 
               crate::init::main_thread_id(), std::thread::current().id());
    }

    let (task_handle, godot_waker) = ASYNC_RUNTIME.with_runtime_mut(move |rt| {
        let task_handle = rt
            .add_task(Box::pin(future))
            .unwrap_or_else(|spawn_error| panic!("Failed to spawn task: {spawn_error}"));
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
/// # Panics
///
/// Panics if:
/// - No async runtime has been registered
/// - The task queue is full and cannot accept more tasks
/// - Called from a non-main thread
///
/// # Examples
/// ```rust
/// use godot::prelude::*;
/// use godot::task;
///
/// let signal = Signal::from_object_signal(&some_object, "some_signal");
/// let task = task::spawn_local(async move {
///     signal.to_future::<()>().await;
///     println!("Signal received!");
/// });
/// ```
pub fn spawn_local(future: impl Future<Output = ()> + 'static) -> TaskHandle {
    // Check if runtime is registered
    if !is_runtime_registered() {
        panic!("No async runtime has been registered. Call gdext::task::register_runtime() before using async functions.");
    }

    // Must be called from the main thread since Godot objects are not thread-safe
    if !crate::init::is_main_thread() {
        panic!("Async tasks can only be spawned on the main thread. Expected thread: {:?}, current thread: {:?}", 
               crate::init::main_thread_id(), std::thread::current().id());
    }

    let (task_handle, godot_waker) = ASYNC_RUNTIME.with_runtime_mut(move |rt| {
        let task_handle = rt
            .add_task_non_send(Box::pin(future))
            .unwrap_or_else(|spawn_error| panic!("Failed to spawn task: {spawn_error}"));
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
/// }).expect("Failed to spawn task");
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
/// }).expect("Failed to spawn task");
/// ```
///
/// # Thread Safety
///
/// In single-threaded mode (default), this function must be called from the main thread.
/// In multi-threaded mode (with `experimental-threads` feature), it can be called from any thread.
///
/// # Panics
///
/// Panics if:
/// - No async runtime has been registered
/// - The task queue is full and cannot accept more tasks
/// - Called from a non-main thread in single-threaded mode
pub fn spawn_with_result<F, R>(future: F) -> Gd<RefCounted>
where
    F: Future<Output = R> + Send + 'static,
    R: ToGodot + Send + Sync + 'static,
{
    // Check if runtime is registered
    if !is_runtime_registered() {
        panic!("No async runtime has been registered. Call gdext::task::register_runtime() before using async functions.");
    }

    // In single-threaded mode, spawning is only allowed on the main thread
    // In multi-threaded mode, we allow spawning from any thread
    #[cfg(all(not(wasm_nothreads), not(feature = "experimental-threads")))]
    if !crate::init::is_main_thread() {
        panic!("Async tasks can only be spawned on the main thread. Expected thread: {:?}, current thread: {:?}", 
               crate::init::main_thread_id(), std::thread::current().id());
    }

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
/// spawn_with_result_signal(signal_holder, async { 42 }).expect("Failed to spawn task");
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
/// Panics if:
/// - No async runtime has been registered
/// - The task queue is full and cannot accept more tasks
/// - Called from a non-main thread in single-threaded mode
pub fn spawn_with_result_signal<F, R>(signal_emitter: Gd<RefCounted>, future: F)
where
    F: Future<Output = R> + Send + 'static,
    R: ToGodot + Send + Sync + 'static,
{
    // Check if runtime is registered
    if !is_runtime_registered() {
        panic!("No async runtime has been registered. Call gdext::task::register_runtime() before using async functions.");
    }

    // In single-threaded mode, spawning is only allowed on the main thread
    // In multi-threaded mode, we allow spawning from any thread
    #[cfg(all(not(wasm_nothreads), not(feature = "experimental-threads")))]
    if !crate::init::is_main_thread() {
        panic!("Async tasks can only be spawned on the main thread. Expected thread: {:?}, current thread: {:?}", 
               crate::init::main_thread_id(), std::thread::current().id());
    }

    let godot_waker = ASYNC_RUNTIME.with_runtime_mut(|rt| {
        // Create a wrapper that will emit the signal when complete
        let result_future = SignalEmittingFuture {
            inner: future,
            signal_emitter,
            _phantom: PhantomData,
            creation_thread: thread::current().id(),
        };

        // Spawn the signal-emitting future using standard spawn mechanism
        let task_handle = rt
            .add_task_non_send(Box::pin(result_future))
            .unwrap_or_else(|spawn_error| panic!("Failed to spawn task: {spawn_error}"));

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
                FutureSlotState::Gone => false,
                FutureSlotState::Pending(_) => task.id == self.id,
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

            Ok(matches!(slot.value, FutureSlotState::Pending(_)))
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
                let task_count = rt.task_storage.get_active_task_count();

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
    ASYNC_RUNTIME.with_runtime(|rt| rt.has_task_panicked(task_handle.id))
}

/// The current state of a future inside the async runtime.
enum FutureSlotState<T> {
    /// Slot was previously occupied but the future has been canceled or the slot reused.
    Gone,
    /// Slot contains a pending future.
    Pending(T),
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

    /// Checks if the future slot has become unoccupied due to a future completing.
    fn is_empty(&self) -> bool {
        matches!(self.value, FutureSlotState::Gone)
    }

    /// Drop the future from this slot.
    ///
    /// This transitions the slot into the [`FutureSlotState::Gone`] state.
    fn clear(&mut self) {
        self.value = FutureSlotState::Gone;
    }
}

/// Simplified task storage with basic backpressure
const MAX_CONCURRENT_TASKS: usize = 1000;

/// Separated concerns for better architecture
///
/// Task limits and backpressure configuration
#[derive(Debug, Clone)]
pub struct TaskLimits {
    /// Maximum number of concurrent tasks allowed
    pub max_concurrent_tasks: usize,
}

impl Default for TaskLimits {
    fn default() -> Self {
        Self {
            max_concurrent_tasks: MAX_CONCURRENT_TASKS,
        }
    }
}

/// Simplified future storage that avoids unnecessary boxing
/// Only boxes when absolutely necessary (for type erasure)
enum FutureStorage {
    /// Direct storage for Send futures
    Send(Pin<Box<dyn Future<Output = ()> + Send + 'static>>),
    /// For non-Send futures (like Godot integration)
    Local(Pin<Box<dyn Future<Output = ()> + 'static>>),
}

impl FutureStorage {
    /// Create optimized storage for a Send future
    fn new_send<F>(future: F) -> Self
    where
        F: Future<Output = ()> + Send + 'static,
    {
        Self::Send(Box::pin(future))
    }

    /// Create storage for a non-Send future
    fn new_local<F>(future: F) -> Self
    where
        F: Future<Output = ()> + 'static,
    {
        Self::Local(Box::pin(future))
    }
}

/// Simplified task storage component
struct TaskStorage {
    tasks: Vec<FutureSlot<FutureStorage>>,
    next_task_id: u64,
    limits: TaskLimits,
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
        }
    }

    /// Get the next task ID
    fn next_id(&mut self) -> u64 {
        let id = self.next_task_id;
        self.next_task_id += 1;
        id
    }

    /// Store a new Send async task
    fn store_send_task<F>(&mut self, future: F) -> Result<TaskHandle, TaskSpawnError>
    where
        F: Future<Output = ()> + Send + 'static,
    {
        let active_tasks = self.get_active_task_count();
        
        if active_tasks >= self.limits.max_concurrent_tasks {
            return Err(TaskSpawnError::QueueFull {
                active_tasks,
                max_tasks: self.limits.max_concurrent_tasks,
            });
        }

        let id = self.next_id();
        let storage = FutureStorage::new_send(future);
        self.schedule_task_immediately(id, storage)
    }

    /// Store a new non-Send async task
    fn store_local_task<F>(&mut self, future: F) -> Result<TaskHandle, TaskSpawnError>
    where
        F: Future<Output = ()> + 'static,
    {
        let active_tasks = self.get_active_task_count();
        
        if active_tasks >= self.limits.max_concurrent_tasks {
            return Err(TaskSpawnError::QueueFull {
                active_tasks,
                max_tasks: self.limits.max_concurrent_tasks,
            });
        }

        let id = self.next_id();
        let storage = FutureStorage::new_local(future);
        self.schedule_task_immediately(id, storage)
    }

    /// Schedule a task immediately in an available slot
    fn schedule_task_immediately(&mut self, id: u64, storage: FutureStorage) -> Result<TaskHandle, TaskSpawnError> {
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

    /// Clear all tasks
    fn clear_all(&mut self) {
        self.tasks.clear();
    }
}

/// Simplified async runtime 
struct AsyncRuntime {
    task_storage: TaskStorage,
    #[cfg(feature = "trace")]
    panicked_tasks: std::collections::HashSet<u64>,
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
            eprintln!(
                "This would cause memory corruption. Future created on {:?}, polled on {:?}.",
                this.creation_thread, current_thread
            );

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
                        eprintln!(
                            "Signal emission must happen on the same thread as future creation!"
                        );
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
        Self {
            task_storage: TaskStorage::new(),
            #[cfg(feature = "trace")]
            panicked_tasks: std::collections::HashSet::new(),
        }
    }

    /// Store a new async task in the runtime
    fn add_task<F>(&mut self, future: F) -> Result<TaskHandle, TaskSpawnError>
    where
        F: Future<Output = ()> + Send + 'static,
    {
        self.task_storage.store_send_task(future)
    }

    /// Store a new async task in the runtime (for futures that are not Send)
    /// This is used for Godot integration where Gd<T> objects are not Send
    fn add_task_non_send<F>(&mut self, future: F) -> Result<TaskHandle, TaskSpawnError>
    where
        F: Future<Output = ()> + 'static,
    {
        self.task_storage.store_local_task(future)
    }

    /// Remove a future from the storage
    fn clear_task(&mut self, index: usize) {
        self.task_storage.clear_task(index);
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

    /// Clear all data
    fn clear_all(&mut self) {
        self.task_storage.clear_all();
        #[cfg(feature = "trace")]
        self.panicked_tasks.clear();
    }

    /// Poll a future in place without breaking the pin invariant
    /// This safely polls the future while it remains in storage
    fn poll_task_in_place(
        &mut self,
        index: usize,
        id: u64,
        cx: &mut Context<'_>,
    ) -> Result<Poll<()>, AsyncRuntimeError> {
        let slot = self
            .task_storage
            .tasks
            .get_mut(index)
            .ok_or(AsyncRuntimeError::RuntimeDeinitialized)?;

        // Check if the task ID matches and is in the right state
        if slot.id != id {
            return Err(AsyncRuntimeError::InvalidTaskState {
                task_id: id,
                expected_state: "matching task ID".to_string(),
            });
        }

        match &mut slot.value {
            FutureSlotState::Gone => Err(AsyncRuntimeError::TaskCanceled { task_id: id }),
            FutureSlotState::Pending(future_storage) => {
                // Mark as polling to prevent reentrant polling, but don't move the future
                let old_id = slot.id;
                slot.id = u64::MAX; // Special marker for "currently polling"

                // Poll the future in place without moving it - this is safe because:
                // 1. The future remains at the same memory location
                // 2. We're only taking a mutable reference, not moving it
                // 3. Pin guarantees are preserved
                let poll_result = match future_storage {
                    FutureStorage::Send(pinned_future) => pinned_future.as_mut().poll(cx),
                    FutureStorage::Local(pinned_future) => pinned_future.as_mut().poll(cx),
                };

                // Handle the result and restore appropriate state
                match poll_result {
                    Poll::Pending => {
                        // Restore the original ID - future is still pending
                        slot.id = old_id;
                        Ok(Poll::Pending)
                    }
                    Poll::Ready(()) => {
                        // Task completed, mark as gone
                        slot.value = FutureSlotState::Gone;
                        slot.id = old_id; // Restore ID for consistency

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
    let poll_result = RUNTIME_REGISTRY.with(|registry| {
        let registry = registry.borrow();

        if let Some(storage) = &registry.runtime_storage {
            // Poll within the runtime context for proper tokio/async-std support
            let result = std::cell::RefCell::new(None);
            let ctx_ref = std::cell::RefCell::new(Some(ctx));

            (storage.with_context)(&|| {
                let mut ctx = ctx_ref
                    .borrow_mut()
                    .take()
                    .expect("Context should be available");

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
            drop(registry); // Release the borrow before calling ASYNC_RUNTIME
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
        }
    });

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
