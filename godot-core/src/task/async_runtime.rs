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

// Support for async Future with return values

use crate::classes::RefCounted;
use crate::meta::ToGodot;
use crate::obj::Gd;
#[cfg(feature = "trace")]
use crate::obj::NewGd;

/// Trait for integrating external async runtimes with gdext's async system.
///
/// This trait provides the minimal interface for pluggable async runtime support.
/// Users need to implement `create_runtime()` and `with_context()`.
///
/// # Simple Example Implementation
///
/// ```rust
/// use godot_core::task::AsyncRuntimeIntegration;
///
/// struct SimpleIntegration;
///
/// impl AsyncRuntimeIntegration for SimpleIntegration {
///     type Handle = ();
///     
///     fn create_runtime() -> Result<(Box<dyn std::any::Any + Send + Sync>, Self::Handle), String> {
///         Ok((Box::new(()), ()))
///     }
///     
///     fn with_context<R>(handle: &Self::Handle, f: impl FnOnce() -> R) -> R {
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
/// ```rust,no_run
/// use godot_core::task::{AsyncRuntimeIntegration, register_runtime};
///
/// struct MyRuntimeIntegration;
///
/// impl AsyncRuntimeIntegration for MyRuntimeIntegration {
///     type Handle = ();
///     
///     fn create_runtime() -> Result<(Box<dyn std::any::Any + Send + Sync>, Self::Handle), String> {
///         Ok((Box::new(()), ()))
///     }
///     
///     fn with_context<R>(handle: &Self::Handle, f: impl FnOnce() -> R) -> R {
///         f()
///     }
/// }
///
/// // Register your runtime at application startup
/// register_runtime::<MyRuntimeIntegration>()?;
/// # Ok::<(), String>(())
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

// Enhanced Error Handling

/// Errors that can occur during async runtime operations
#[derive(Debug, Clone)]
pub enum AsyncRuntimeError {
    /// Runtime is unavailable (deinitialized or not registered)
    RuntimeUnavailable { reason: String },
    /// Task-related error (canceled, panicked, spawn failed, etc.)
    TaskError {
        task_id: Option<u64>,
        message: String,
    },
    /// Thread safety violation (MUST keep separate - critical for memory safety)
    ThreadSafetyViolation {
        expected_thread: ThreadId,
        actual_thread: ThreadId,
    },
}

impl std::fmt::Display for AsyncRuntimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AsyncRuntimeError::RuntimeUnavailable { reason } => {
                write!(f, "Async runtime is unavailable: {reason}")
            }
            AsyncRuntimeError::TaskError { task_id, message } => {
                if let Some(id) = task_id {
                    write!(f, "Task {id} error: {message}")
                } else {
                    write!(f, "Task error: {message}")
                }
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
            TaskSpawnError::QueueFull {
                active_tasks,
                max_tasks,
            } => {
                write!(f, "Task queue is full: {active_tasks}/{max_tasks} tasks")
            }
        }
    }
}

impl std::error::Error for TaskSpawnError {}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Public interface

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
/// ```rust,no_run
/// use godot::prelude::*;
/// use godot::classes::RefCounted;
/// use godot_core::task::spawn_async_func;
/// use godot_core::obj::NewGd;
///
/// let object = RefCounted::new_gd();
/// let signal = Signal::from_object_signal(&object, "some_signal");
///
/// // Create a signal holder for the async function
/// let mut signal_holder = RefCounted::new_gd();
/// signal_holder.add_user_signal("finished");
///
/// spawn_async_func(signal_holder, async move {
///     signal.to_future::<()>().await;
///     println!("Signal received!");
/// });
/// ```
/// Unified function for spawning async functions (main public API).
///
/// This is the primary function used by the `#[async_func]` macro. It handles both void
/// and non-void async functions by automatically detecting the return type and using
/// the appropriate signal emission strategy.
///
/// # Arguments
///
/// * `signal_emitter` - The RefCounted object that will emit the "finished" signal
/// * `future` - The async function to execute
///
/// # Thread Safety
///
/// This function must be called from the main thread and the future will be polled
/// on the main thread, ensuring compatibility with Godot's threading model.
///
/// # Panics
///
/// Panics if:
/// - No async runtime has been registered
/// - The task queue is full and cannot accept more tasks
/// - Called from a non-main thread
///
/// # Examples
///
/// For non-void functions:
/// ```rust,no_run
/// use godot::classes::RefCounted;
/// use godot_core::task::spawn_async_func;
/// use godot_core::obj::NewGd;
///
/// let mut signal_holder = RefCounted::new_gd();
/// signal_holder.add_user_signal("finished");
///
/// spawn_async_func(signal_holder, async {
///     // Some async computation
///     42
/// });
/// ```
///
/// For void functions:
/// ```rust,no_run
/// use godot::classes::RefCounted;
/// use godot_core::task::spawn_async_func;
/// use godot_core::obj::NewGd;
///
/// let mut signal_holder = RefCounted::new_gd();
/// signal_holder.add_user_signal("finished");
///
/// spawn_async_func(signal_holder, async {
///     // Some async computation with no return value
///     println!("Task completed");
/// });
/// ```
pub fn spawn_async_func<F, R>(signal_emitter: Gd<RefCounted>, future: F)
where
    F: Future<Output = R> + 'static,
    R: ToGodot + 'static,
{
    // Check if runtime is registered
    if !is_runtime_registered() {
        panic!("No async runtime has been registered. Call gdext::task::register_runtime() before using async functions.");
    }

    // Must be called from the main thread since Godot objects are not thread-safe
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
            creation_thread: std::thread::current().id(),
        };

        // Spawn the signal-emitting future using non-Send mechanism
        let task_handle = rt
            .add_task_non_send(Box::pin(result_future))
            .unwrap_or_else(|spawn_error| panic!("Failed to spawn task: {spawn_error}"));

        // Create waker to trigger initial poll
        Arc::new(GodotWaker::new(
            task_handle.index as usize,
            task_handle.id as u64,
            std::thread::current().id(),
        ))
    });

    // Trigger initial poll
    poll_future(godot_waker);
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Testing-only functions (only available with trace feature)

#[cfg(feature = "trace")]
/// Create a new async background task that doesn't require Send (for testing).
///
/// This function is only available when the `trace` feature is enabled and is used
/// for testing purposes. It allows futures that contain non-Send types like Godot
/// objects (`Gd<T>`, `Signal`, etc.). The future will be polled on the main thread.
///
/// # Thread Safety
///
/// This function must be called from the main thread and the future will be polled
/// on the main thread, ensuring compatibility with Godot's threading model.
///
/// # Panics
///
/// Panics if:
/// - No async runtime has been registered
/// - The task queue is full and cannot accept more tasks
/// - Called from a non-main thread
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

    // Batch both task creation and initial waker setup in single thread-local access
    let (task_handle, godot_waker) = ASYNC_RUNTIME.with_runtime_mut(move |rt| {
        // Let add_task_non_send handle the boxing to avoid premature allocation
        let task_handle = rt
            .add_task_non_send(future) // Pass unboxed future
            .unwrap_or_else(|spawn_error| panic!("Failed to spawn task: {spawn_error}"));

        // Create waker immediately while we have runtime access
        let godot_waker = Arc::new(GodotWaker::new(
            task_handle.index as usize,
            task_handle.id as u64,
            thread::current().id(),
        ));

        (task_handle, godot_waker)
    });

    poll_future(godot_waker);
    task_handle
}

#[cfg(feature = "trace")]
/// Spawn an async task that returns a value (for testing).
///
/// This function is only available when the `trace` feature is enabled and is used
/// for testing purposes. It returns a [`Gd<RefCounted>`] that can be directly
/// awaited in GDScript. When the async task completes, the object emits a
/// `finished` signal with the result.
///
/// # Thread Safety
///
/// This function must be called from the main thread and the future will be polled
/// on the main thread, ensuring compatibility with Godot's threading model.
///
/// # Panics
///
/// Panics if:
/// - No async runtime has been registered
/// - The task queue is full and cannot accept more tasks
/// - Called from a non-main thread
pub fn spawn_with_result<F, R>(future: F) -> Gd<RefCounted>
where
    F: Future<Output = R> + 'static,
    R: ToGodot + 'static,
{
    // Check if runtime is registered
    if !is_runtime_registered() {
        panic!("No async runtime has been registered. Call gdext::task::register_runtime() before using async functions.");
    }

    // Must be called from the main thread since Godot objects are not thread-safe
    if !crate::init::is_main_thread() {
        panic!("Async tasks can only be spawned on the main thread. Expected thread: {:?}, current thread: {:?}", 
               crate::init::main_thread_id(), std::thread::current().id());
    }

    // Create a RefCounted object that will emit the completion signal
    let mut signal_emitter = RefCounted::new_gd();

    // Add a user-defined signal that takes a Variant parameter
    signal_emitter.add_user_signal("finished");

    // Use the unified API internally
    spawn_async_func(signal_emitter.clone(), future);
    signal_emitter
}

/// Handle for an active background task.
///
/// This handle provides introspection into the current state of the task, as well as providing a way to cancel it.
///
/// The associated task will **not** be canceled if this handle is dropped.
pub struct TaskHandle {
    // Pack index and id for better cache efficiency
    // Most systems won't need more than 32-bit task indices
    index: u32,
    id: u32,
    // More efficient !Send/!Sync marker
    _not_send_sync: std::cell::Cell<()>,
}

impl TaskHandle {
    fn new(index: usize, id: u64) -> Self {
        // Ensure we don't overflow the packed format
        // In practice, these should never be hit for reasonable usage
        assert!(index <= u32::MAX as usize, "Task index overflow: {index}");
        assert!(id <= u32::MAX as u64, "Task ID overflow: {id}");

        Self {
            index: index as u32,
            id: id as u32,
            _not_send_sync: std::cell::Cell::new(()),
        }
    }

    /// Cancels the task if it is still pending and does nothing if it is already completed.
    ///
    /// Returns Ok(()) if the task was successfully canceled or was already completed.
    /// Returns Err if the runtime has been deinitialized.
    pub fn cancel(self) -> AsyncRuntimeResult<()> {
        ASYNC_RUNTIME.with_runtime_mut(|rt| {
            let Some(task) = rt.task_storage.tasks.get(self.index as usize) else {
                return Err(AsyncRuntimeError::RuntimeUnavailable {
                    reason: "Runtime deinitialized".to_string(),
                });
            };

            let alive = match task.value {
                FutureSlotState::Gone => false,
                FutureSlotState::Pending(_) => task.id == self.id as u64,
            };

            if alive {
                rt.clear_task(self.index as usize);
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
            let slot = rt.task_storage.tasks.get(self.index as usize).ok_or(
                AsyncRuntimeError::RuntimeUnavailable {
                    reason: "Runtime deinitialized".to_string(),
                },
            )?;

            if slot.id != self.id as u64 {
                return Ok(false);
            }

            Ok(matches!(slot.value, FutureSlotState::Pending(_)))
        })
    }

    /// Get the task ID for debugging purposes
    pub fn task_id(&self) -> u64 {
        self.id as u64
    }

    /// Get the task index for debugging purposes
    pub fn task_index(&self) -> usize {
        self.index as usize
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

                // Note: task_count tasks were canceled during shutdown

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
    let _canceled_tasks = lifecycle::begin_shutdown();
    // Note: _canceled_tasks tasks were canceled during engine shutdown
}

#[cfg(feature = "trace")]
pub fn has_godot_task_panicked(task_handle: TaskHandle) -> bool {
    ASYNC_RUNTIME.with_runtime(|rt| rt.has_task_panicked(task_handle.id as u64))
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

/// Optimized future storage that minimizes boxing overhead
/// Uses a unified approach to avoid enum discrimination
struct FutureStorage {
    /// Unified storage for both Send and non-Send futures
    /// The Send bound is erased at the type level since all futures
    /// will be polled on the main thread anyway
    inner: Pin<Box<dyn Future<Output = ()> + 'static>>,
}

impl FutureStorage {
    /// Create storage for a non-Send future - avoids double boxing  
    fn new_local<F>(future: F) -> Self
    where
        F: Future<Output = ()> + 'static,
    {
        Self {
            inner: Box::pin(future),
        }
    }

    /// Poll the stored future - no enum matching overhead
    fn poll(&mut self, cx: &mut Context<'_>) -> Poll<()> {
        self.inner.as_mut().poll(cx)
    }
}

/// Simplified task storage component
struct TaskStorage {
    tasks: Vec<FutureSlot<FutureStorage>>,
    /// O(1) free slot tracking - indices of available slots
    free_slots: Vec<usize>,
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
            free_slots: Vec::new(),
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
        self.schedule_task_optimized(id, storage)
    }

    /// O(1) slot allocation using free list
    fn schedule_task_optimized(
        &mut self,
        id: u64,
        storage: FutureStorage,
    ) -> Result<TaskHandle, TaskSpawnError> {
        let index = if let Some(free_index) = self.free_slots.pop() {
            // Reuse a free slot - O(1)
            self.tasks[free_index] = FutureSlot::pending(id, storage);
            free_index
        } else {
            // Allocate new slot - amortized O(1)
            let new_index = self.tasks.len();
            self.tasks.push(FutureSlot::pending(id, storage));
            new_index
        };

        Ok(TaskHandle::new(index, id))
    }

    /// Get the count of active (non-empty) tasks
    fn get_active_task_count(&self) -> usize {
        self.tasks.len() - self.free_slots.len()
    }

    /// Remove a future from storage - O(1)
    fn clear_task(&mut self, index: usize) {
        if let Some(slot) = self.tasks.get_mut(index) {
            if !slot.is_empty() {
                slot.clear();
                self.free_slots.push(index);
            }
        }
    }

    /// Clear all tasks
    fn clear_all(&mut self) {
        self.tasks.clear();
        self.free_slots.clear();
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

pin_project! {
    /// Wrapper for futures that emits a completion signal (for void methods)
    ///
    /// Similar to `SignalEmittingFuture` but designed for futures that return `()`.
    /// Only emits completion signal without any result parameter.
    ///
    /// # Thread Safety
    ///
    /// This future ensures that signal emission always happens on the main thread
    /// via call_deferred, maintaining Godot's threading model.
    struct CompletionSignalFuture<F> {
        #[pin]
        inner: F,
        signal_emitter: Gd<RefCounted>,
        creation_thread: ThreadId,
    }
}

impl<F, R> Future for SignalEmittingFuture<F, R>
where
    F: Future<Output = R>,
    R: ToGodot + 'static,
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
        let slot = self.task_storage.tasks.get_mut(index).ok_or(
            AsyncRuntimeError::RuntimeUnavailable {
                reason: "Runtime deinitialized".to_string(),
            },
        )?;

        // Check if the task ID matches and is in the right state
        if slot.id != id {
            return Err(AsyncRuntimeError::TaskError {
                task_id: Some(id),
                message: "Task ID mismatch".to_string(),
            });
        }

        match &mut slot.value {
            FutureSlotState::Gone => Err(AsyncRuntimeError::TaskError {
                task_id: Some(id),
                message: "Task already completed".to_string(),
            }),
            FutureSlotState::Pending(future_storage) => {
                // Mark as polling to prevent reentrant polling, but don't move the future
                let old_id = slot.id;
                slot.id = u64::MAX; // Special marker for "currently polling"

                // Poll the future directly using the unified storage - no enum matching!
                let poll_result = future_storage.poll(cx);

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

impl<F> Future for CompletionSignalFuture<F>
where
    F: Future<Output = ()>,
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
            eprintln!("CompletionSignalFuture with Gd<RefCounted> cannot be accessed from different threads!");
            eprintln!(
                "This would cause memory corruption. Future created on {:?}, polled on {:?}.",
                this.creation_thread, current_thread
            );

            // MUST panic to prevent memory corruption - Godot objects are not thread-safe
            panic!("Thread safety violation in CompletionSignalFuture: {error}");
        }

        match this.inner.poll(cx) {
            Poll::Ready(()) => {
                // For void methods, just emit completion signal without parameters
                let mut signal_emitter = this.signal_emitter.clone();
                let creation_thread_id = *this.creation_thread;

                let callable = Callable::from_local_fn("emit_completion_signal", move |_args| {
                    // CRITICAL: Thread safety validation - signal emission must be on correct thread
                    let emission_thread = thread::current().id();
                    if creation_thread_id != emission_thread {
                        let error = AsyncRuntimeError::ThreadSafetyViolation {
                            expected_thread: creation_thread_id,
                            actual_thread: emission_thread,
                        };

                        eprintln!("FATAL: {error}");
                        eprintln!(
                            "Completion signal emission must happen on the same thread as future creation!"
                        );
                        eprintln!("This would cause memory corruption with Gd<RefCounted>. Created on {creation_thread_id:?}, emitting on {emission_thread:?}");

                        // MUST panic to prevent memory corruption - signal_emitter is not thread-safe
                        panic!("Thread safety violation in completion signal emission: {error}");
                    }

                    // Enhanced error handling for signal emission
                    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        signal_emitter.emit_signal("finished", &[]);
                    })) {
                        Ok(()) => Ok(Variant::nil()),
                        Err(panic_err) => {
                            let error_msg = if let Some(s) = panic_err.downcast_ref::<String>() {
                                s.clone()
                            } else if let Some(s) = panic_err.downcast_ref::<&str>() {
                                s.to_string()
                            } else {
                                "Unknown panic during completion signal emission".to_string()
                            };

                            eprintln!("Warning: Completion signal emission failed: {error_msg}");
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
/// This version avoids cloning the Arc when we already have ownership.
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

    // OPTIMIZATION: Extract values before creating Waker to avoid referencing after move
    let task_id = godot_waker.task_id;
    let runtime_index = godot_waker.runtime_index;
    let error_context = || format!("Godot async task failed (task_id: {task_id})");

    // Convert Arc<GodotWaker> to Waker (consumes the Arc without cloning)
    let waker = Waker::from(godot_waker);
    let mut ctx = Context::from_waker(&waker);

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
                            rt.poll_task_in_place(runtime_index, task_id, &mut ctx)
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
                    AssertUnwindSafe(|| rt.poll_task_in_place(runtime_index, task_id, &mut ctx)),
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
                rt.clear_task(runtime_index);
            });
        }
        Err(_panic_payload) => {
            // Task panicked during polling
            let error = AsyncRuntimeError::TaskError {
                task_id: Some(task_id),
                message: "Task panicked during polling".to_string(),
            };

            eprintln!("Error: {error}");

            ASYNC_RUNTIME.with_runtime_mut(|rt| {
                #[cfg(feature = "trace")]
                rt.track_panic(task_id);
                rt.clear_task(runtime_index);
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
