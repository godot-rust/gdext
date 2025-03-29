/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

pub use crate::gen::classes::class_macros;
pub use crate::obj::rtti::ObjectRtti;
pub use crate::registry::callbacks;
pub use crate::registry::plugin::{
    ClassPlugin, DynTraitImpl, ErasedDynGd, ErasedRegisterFn, ITraitImpl, InherentImpl, PluginItem,
    Struct,
};
pub use crate::storage::{as_storage, Storage};
pub use sys::out;

#[cfg(feature = "trace")]
pub use crate::meta::trace;
use std::cell::{Cell, RefCell};

use crate::global::godot_error;
use crate::meta::error::CallError;
use crate::meta::CallContext;
use crate::sys;
use std::io::Write;
use std::sync::atomic;
use sys::Global;

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Global variables

static CALL_ERRORS: Global<CallErrors> = Global::default();

/// Level:
/// - 0: no error printing (during `expect_panic` in test)
/// - 1: not yet implemented, but intended for `try_` function calls (which are expected to fail, so error is annoying)
/// - 2: normal printing
static ERROR_PRINT_LEVEL: atomic::AtomicU8 = atomic::AtomicU8::new(2);

sys::plugin_registry!(pub __GODOT_PLUGIN_REGISTRY: ClassPlugin);

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Call error handling

// Note: if this leads to many allocated IDs that are not removed, we could limit to 1 per thread-ID.
// Would need to check if re-entrant calls with multiple errors per thread are possible.
struct CallErrors {
    ring_buffer: Vec<Option<CallError>>,
    next_id: u8,
    generation: u16,
}

impl Default for CallErrors {
    fn default() -> Self {
        Self {
            ring_buffer: [const { None }; Self::MAX_ENTRIES as usize].into(),
            next_id: 0,
            generation: 0,
        }
    }
}

impl CallErrors {
    const MAX_ENTRIES: u8 = 32;

    fn insert(&mut self, err: CallError) -> i32 {
        let id = self.next_id;

        self.next_id = self.next_id.wrapping_add(1) % Self::MAX_ENTRIES;
        if self.next_id == 0 {
            self.generation = self.generation.wrapping_add(1);
        }

        self.ring_buffer[id as usize] = Some(err);

        (self.generation as i32) << 16 | id as i32
    }

    // Returns success or failure.
    fn remove(&mut self, id: i32) -> Option<CallError> {
        let generation = (id >> 16) as u16;
        let id = id as u8;

        // If id < next_id, the generation must be the current one -- otherwise the one before.
        if id < self.next_id {
            if generation != self.generation {
                return None;
            }
        } else if generation != self.generation.wrapping_sub(1) {
            return None;
        }

        // Returns Some if there's still an entry, None if it was already removed.
        self.ring_buffer[id as usize].take()
    }
}

/// Inserts a `CallError` into a global variable and returns its ID to later remove it.
fn call_error_insert(err: CallError) -> i32 {
    // Wraps around if entire i32 is depleted. If this happens in practice (unlikely, users need to deliberately ignore errors that are printed),
    // we just overwrite the oldest errors, should still work.
    let id = CALL_ERRORS.lock().insert(err);
    id
}

pub(crate) fn call_error_remove(in_error: &sys::GDExtensionCallError) -> Option<CallError> {
    // Error checks are just quality-of-life diagnostic; do not throw panics if they fail.

    if in_error.error != sys::GODOT_RUST_CUSTOM_CALL_ERROR {
        godot_error!("Tried to remove non-godot-rust call error {in_error:?}");
        return None;
    }

    let call_error = CALL_ERRORS.lock().remove(in_error.argument);
    if call_error.is_none() {
        // Just a quality-of-life diagnostic; do not throw panics if something like this fails.
        godot_error!("Failed to remove call error {in_error:?}");
    }

    call_error
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Plugin and global state handling

pub fn next_class_id() -> u16 {
    static NEXT_CLASS_ID: atomic::AtomicU16 = atomic::AtomicU16::new(0);
    NEXT_CLASS_ID.fetch_add(1, atomic::Ordering::Relaxed)
}

pub(crate) fn iterate_plugins(mut visitor: impl FnMut(&ClassPlugin)) {
    sys::plugin_foreach!(__GODOT_PLUGIN_REGISTRY; visitor);
}

#[cfg(feature = "codegen-full")] // Remove if used in other scenarios.
pub(crate) fn find_inherent_impl(class_name: crate::meta::ClassName) -> Option<InherentImpl> {
    // We do this manually instead of using `iterate_plugins()` because we want to break as soon as we find a match.
    let plugins = __GODOT_PLUGIN_REGISTRY.lock().unwrap();

    plugins.iter().find_map(|elem| {
        if elem.class_name == class_name {
            if let PluginItem::InherentImpl(inherent_impl) = &elem.item {
                return Some(inherent_impl.clone());
            }
        }

        None
    })
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Traits and types

// If someone forgets #[godot_api], this causes a compile error, rather than virtual functions not being called at runtime.
#[allow(non_camel_case_types)]
#[diagnostic::on_unimplemented(
    message = "`impl` blocks for Godot classes require the `#[godot_api]` attribute",
    label = "missing `#[godot_api]` before `impl`",
    note = "see also: https://godot-rust.github.io/book/register/functions.html#godot-special-functions"
)]
pub trait You_forgot_the_attribute__godot_api {}

pub struct ClassConfig {
    pub is_tool: bool,
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Capability queries and internal access

pub fn auto_init<T>(l: &mut crate::obj::OnReady<T>, base: &crate::obj::Gd<crate::classes::Node>) {
    l.init_auto(base);
}

#[cfg(since_api = "4.3")]
pub unsafe fn has_virtual_script_method(
    object_ptr: sys::GDExtensionObjectPtr,
    method_sname: sys::GDExtensionConstStringNamePtr,
) -> bool {
    sys::interface_fn!(object_has_script_method)(sys::to_const_ptr(object_ptr), method_sname) != 0
}

/// Ensure `T` is an editor plugin.
pub const fn is_editor_plugin<T: crate::obj::Inherits<crate::classes::EditorPlugin>>() {}

// Starting from 4.3, Godot has "runtime classes"; this emulation is no longer needed.
#[cfg(before_api = "4.3")]
pub fn is_class_inactive(is_tool: bool) -> bool {
    if is_tool {
        return false;
    }

    // SAFETY: only invoked after global library initialization.
    let global_config = unsafe { sys::config() };
    let is_editor = || crate::classes::Engine::singleton().is_editor_hint();

    global_config.tool_only_in_editor //.
        && global_config.is_editor_or_init(is_editor)
}

// Starting from 4.3, Godot has "runtime classes"; we only need to check whether editor is running.
#[cfg(since_api = "4.3")]
pub fn is_class_runtime(is_tool: bool) -> bool {
    if is_tool {
        return false;
    }

    // SAFETY: only invoked after global library initialization.
    let global_config = unsafe { sys::config() };

    // If this is not a #[class(tool)] and we only run tool classes in the editor, then don't run this in editor -> make it a runtime class.
    // If we run all classes in the editor (!tool_only_in_editor), then it's not a runtime class.
    global_config.tool_only_in_editor
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Panic handling

pub fn extract_panic_message(err: &(dyn Send + std::any::Any)) -> String {
    if let Some(s) = err.downcast_ref::<&'static str>() {
        s.to_string()
    } else if let Some(s) = err.downcast_ref::<String>() {
        s.clone()
    } else {
        format!("(panic of type ID {:?})", err.type_id())
    }
}

pub fn format_panic_message(panic_info: &std::panic::PanicHookInfo) -> String {
    let mut msg = extract_panic_message(panic_info.payload());

    if let Some(context) = get_gdext_panic_context() {
        msg = format!("{msg}\nContext: {context}");
    }

    let prefix = if let Some(location) = panic_info.location() {
        format!("panic {}:{}", location.file(), location.line())
    } else {
        "panic".to_string()
    };

    // If the message contains newlines, print all of the lines after a line break, and indent them.
    let lbegin = "\n  ";
    let indented = msg.replace('\n', lbegin);

    if indented.len() != msg.len() {
        format!("[{prefix}]{lbegin}{indented}")
    } else {
        format!("[{prefix}]  {msg}")
    }
}

// Macro instead of function, to avoid 1 extra frame in backtrace.
#[cfg(debug_assertions)]
#[macro_export]
macro_rules! format_backtrace {
    ($prefix:expr, $backtrace:expr) => {{
        use std::backtrace::BacktraceStatus;

        let backtrace = $backtrace;

        match backtrace.status() {
            BacktraceStatus::Captured => format!("\n[{}]\n{}\n", $prefix, backtrace),
            BacktraceStatus::Disabled => {
                "(backtrace disabled, run application with `RUST_BACKTRACE=1` environment variable)"
                    .to_string()
            }
            BacktraceStatus::Unsupported => {
                "(backtrace unsupported for current platform)".to_string()
            }
            _ => "(backtrace status unknown)".to_string(),
        }
    }};

    ($prefix:expr) => {
        $crate::format_backtrace!($prefix, std::backtrace::Backtrace::capture())
    };
}

#[cfg(not(debug_assertions))]
#[macro_export]
macro_rules! format_backtrace {
    ($prefix:expr $(, $backtrace:expr)? ) => {
        String::new()
    };
}

pub fn set_gdext_hook<F>(godot_print: F)
where
    F: Fn() -> bool + Send + Sync + 'static,
{
    std::panic::set_hook(Box::new(move |panic_info| {
        // Flush, to make sure previous Rust output (e.g. test announcement, or debug prints during app) have been printed.
        let _ignored_result = std::io::stdout().flush();

        let message = format_panic_message(panic_info);
        if godot_print() {
            // Also prints to stdout/stderr -- do not print twice.
            godot_error!("{message}");
        } else {
            eprintln!("{message}");
        }

        let backtrace = format_backtrace!("panic backtrace");
        eprintln!("{backtrace}");
        let _ignored_result = std::io::stderr().flush();
    }));
}

pub fn set_error_print_level(level: u8) -> u8 {
    assert!(level <= 2);
    ERROR_PRINT_LEVEL.swap(level, atomic::Ordering::Relaxed)
}

pub(crate) fn has_error_print_level(level: u8) -> bool {
    assert!(level <= 2);
    ERROR_PRINT_LEVEL.load(atomic::Ordering::Relaxed) >= level
}

/// Internal type used to store context information for debug purposes. Debug context is stored on the thread-local
/// ERROR_CONTEXT_STACK, which can later be used to retrieve the current context in the event of a panic. This value
/// probably shouldn't be used directly; use ['get_gdext_panic_context()'](get_gdext_panic_context) instead.
#[cfg(all(debug_assertions, not(wasm_nothreads)))]
struct ScopedFunctionStack {
    functions: Vec<*const dyn Fn() -> String>,
}

#[cfg(all(debug_assertions, not(wasm_nothreads)))]
impl ScopedFunctionStack {
    /// # Safety
    /// Function must be removed (using [`pop_function()`](Self::pop_function)) before lifetime is invalidated.
    unsafe fn push_function(&mut self, function: &dyn Fn() -> String) {
        let function = std::ptr::from_ref(function);
        #[allow(clippy::unnecessary_cast)]
        let function = function as *const (dyn Fn() -> String + 'static);
        self.functions.push(function);
    }

    fn pop_function(&mut self) {
        self.functions.pop().expect("function stack is empty!");
    }

    fn get_last(&self) -> Option<String> {
        self.functions.last().cloned().map(|pointer| {
            // SAFETY:
            // Invariants provided by push_function assert that any and all functions held by ScopedFunctionStack
            // are removed before they are invalidated; functions must always be valid.
            unsafe { (*pointer)() }
        })
    }
}

/// A thread local which adequately behaves as a global variable when compiling under `experimental-wasm-nothreads`, as it does
/// not support thread locals. Aims to support similar APIs as [`std::thread::LocalKey`].
pub(crate) struct GodotThreadLocal<T: 'static> {
    #[cfg(not(wasm_nothreads))]
    threaded_val: &'static std::thread::LocalKey<T>,

    #[cfg(wasm_nothreads)]
    non_threaded_val: std::cell::OnceCell<T>,

    #[cfg(wasm_nothreads)]
    initializer: fn() -> T,
}

// SAFETY: there can only be one thread with `wasm_nothreads`.
#[cfg(wasm_nothreads)]
unsafe impl<T: 'static> Sync for GodotThreadLocal<T> {}

impl<T: 'static> GodotThreadLocal<T> {
    #[cfg(not(wasm_nothreads))]
    pub const fn new_threads(key: &'static std::thread::LocalKey<T>) -> Self {
        Self { threaded_val: key }
    }

    #[cfg(wasm_nothreads)]
    pub const fn new_nothreads(initializer: fn() -> T) -> Self {
        Self {
            non_threaded_val: std::cell::OnceCell::new(),
            initializer,
        }
    }

    /// Acquires a reference to the value in this TLS key.
    ///
    /// See [`std::thread::LocalKey::with`] for details.
    pub fn with<F, R>(&'static self, f: F) -> R
    where
        F: FnOnce(&T) -> R,
    {
        #[cfg(not(wasm_nothreads))]
        return self.threaded_val.with(f);

        #[cfg(wasm_nothreads)]
        f(self.non_threaded_val.get_or_init(self.initializer))
    }

    /// Acquires a reference to the value in this TLS key.
    ///
    /// See [`std::thread::LocalKey::try_with`] for details.
    #[allow(dead_code)]
    #[inline]
    pub fn try_with<F, R>(&'static self, f: F) -> Result<R, std::thread::AccessError>
    where
        F: FnOnce(&T) -> R,
    {
        #[cfg(not(wasm_nothreads))]
        return self.threaded_val.try_with(f);

        #[cfg(wasm_nothreads)]
        Ok(self.with(f))
    }
}

#[allow(dead_code)]
impl<T: 'static> GodotThreadLocal<Cell<T>> {
    /// Sets or initializes the contained value.
    ///
    /// See [`std::thread::LocalKey::set`] for details.
    pub fn set(&'static self, value: T) {
        #[cfg(not(wasm_nothreads))]
        return self.threaded_val.set(value);

        // According to `LocalKey` docs, this method must not call the default initializer.
        #[cfg(wasm_nothreads)]
        if let Some(initialized) = self.non_threaded_val.get() {
            initialized.set(value);
        } else {
            self.non_threaded_val.get_or_init(|| Cell::new(value));
        }
    }

    /// Returns a copy of the contained value.
    ///
    /// See [`std::thread::LocalKey::get`] for details.
    pub fn get(&'static self) -> T
    where
        T: Copy,
    {
        #[cfg(not(wasm_nothreads))]
        return self.threaded_val.get();

        #[cfg(wasm_nothreads)]
        self.with(Cell::get)
    }

    /// Takes the contained value, leaving `Default::default()` in its place.
    ///
    /// See [`std::thread::LocalKey::take`] for details.
    pub fn take(&'static self) -> T
    where
        T: Default,
    {
        #[cfg(not(wasm_nothreads))]
        return self.threaded_val.take();

        #[cfg(wasm_nothreads)]
        self.with(Cell::take)
    }

    /// Replaces the contained value, returning the old value.
    ///
    /// See [`std::thread::LocalKey::replace`] for details.
    pub fn replace(&'static self, value: T) -> T {
        #[cfg(not(wasm_nothreads))]
        return self.threaded_val.replace(value);

        #[cfg(wasm_nothreads)]
        self.with(|cell| cell.replace(value))
    }
}

#[allow(dead_code)]
impl<T: 'static> GodotThreadLocal<RefCell<T>> {
    /// Acquires a reference to the contained value.
    ///
    /// See [`std::thread::LocalKey::with_borrow`] for details.
    pub fn with_borrow<F, R>(&'static self, f: F) -> R
    where
        F: FnOnce(&T) -> R,
    {
        #[cfg(not(wasm_nothreads))]
        return self.threaded_val.with_borrow(f);

        #[cfg(wasm_nothreads)]
        self.with(|cell| f(&cell.borrow()))
    }

    /// Acquires a mutable reference to the contained value.
    ///
    /// See [`std::thread::LocalKey::with_borrow_mut`] for details.
    pub fn with_borrow_mut<F, R>(&'static self, f: F) -> R
    where
        F: FnOnce(&mut T) -> R,
    {
        #[cfg(not(wasm_nothreads))]
        return self.threaded_val.with_borrow_mut(f);

        #[cfg(wasm_nothreads)]
        self.with(|cell| f(&mut cell.borrow_mut()))
    }

    /// Sets or initializes the contained value.
    ///
    /// See [`std::thread::LocalKey::set`] for details.
    pub fn set(&'static self, value: T) {
        #[cfg(not(wasm_nothreads))]
        return self.threaded_val.set(value);

        // According to `LocalKey` docs, this method must not call the default initializer.
        #[cfg(wasm_nothreads)]
        if let Some(initialized) = self.non_threaded_val.get() {
            *initialized.borrow_mut() = value;
        } else {
            self.non_threaded_val.get_or_init(|| RefCell::new(value));
        }
    }

    /// Takes the contained value, leaving `Default::default()` in its place.
    ///
    /// See [`std::thread::LocalKey::take`] for details.
    pub fn take(&'static self) -> T
    where
        T: Default,
    {
        #[cfg(not(wasm_nothreads))]
        return self.threaded_val.take();

        #[cfg(wasm_nothreads)]
        self.with(RefCell::take)
    }

    /// Replaces the contained value, returning the old value.
    ///
    /// See [`std::thread::LocalKey::replace`] for details.
    pub fn replace(&'static self, value: T) -> T {
        #[cfg(not(wasm_nothreads))]
        return self.threaded_val.replace(value);

        #[cfg(wasm_nothreads)]
        self.with(|cell| cell.replace(value))
    }
}

impl<T: 'static> std::fmt::Debug for GodotThreadLocal<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GodotThreadLocal").finish_non_exhaustive()
    }
}

#[cfg(not(wasm_nothreads))]
macro_rules! godot_thread_local {
    // empty (base case for the recursion)
    () => {};

    ($(#[$attr:meta])* $vis:vis static $name:ident: $ty:ty = const $init:block; $($rest:tt)*) => {
        $crate::private::godot_thread_local!($(#[$attr])* $vis static $name: $ty = const $init);
        $crate::private::godot_thread_local!($($rest)*);
    };

    ($(#[$attr:meta])* $vis:vis static $name:ident: $ty:ty = const $init:block) => {
        $(#[$attr])*
        $vis static $name: $crate::private::GodotThreadLocal<$ty> = {
            ::std::thread_local! {
                static $name: $ty = const $init
            }

            $crate::private::GodotThreadLocal::new_threads(&$name)
        };
    };

    ($(#[$attr:meta])* $vis:vis static $name:ident: $ty:ty = $init:expr; $($rest:tt)*) => {
        $crate::private::godot_thread_local!($(#[$attr])* $vis static $name: $ty = $init);
        $crate::private::godot_thread_local!($($rest)*);
    };

    ($(#[$attr:meta])* $vis:vis static $name:ident: $ty:ty = $init:expr) => {
        $(#[$attr])*
        $vis static $name: $crate::private::GodotThreadLocal<$ty> = {
            ::std::thread_local! {
                static $name: $ty = $init
            }

            $crate::private::GodotThreadLocal::new_threads(&$name)
        };
    };
}

#[cfg(wasm_nothreads)]
macro_rules! godot_thread_local {
    // empty (base case for the recursion)
    () => {};

    ($(#[$attr:meta])* $vis:vis static $name:ident: $ty:ty = const $init:block; $($rest:tt)*) => {
        $crate::private::godot_thread_local!($(#[$attr])* $vis static $name: $ty = const $init);
        $crate::private::godot_thread_local!($($rest)*);
    };

    ($(#[$attr:meta])* $vis:vis static $name:ident: $ty:ty = const $init:block) => {
        $(#[$attr])*
        $vis static $name: $crate::private::GodotThreadLocal<$ty> =
            $crate::private::GodotThreadLocal::new_nothreads(|| $init);
    };

    ($(#[$attr:meta])* $vis:vis static $name:ident: $ty:ty = $init:expr; $($rest:tt)*) => {
        $crate::private::godot_thread_local!($(#[$attr])* $vis static $name: $ty = $init);
        $crate::private::godot_thread_local!($($rest)*);
    };

    ($(#[$attr:meta])* $vis:vis static $name:ident: $ty:ty = $init:expr) => {
        $(#[$attr])*
        $vis static $name: $crate::private::GodotThreadLocal<$ty> =
            $crate::private::GodotThreadLocal::new_nothreads(|| $init);
    };
}

pub(crate) use godot_thread_local;

#[cfg(all(debug_assertions, not(wasm_nothreads)))]
thread_local! {
    static ERROR_CONTEXT_STACK: RefCell<ScopedFunctionStack> = const {
        RefCell::new(ScopedFunctionStack { functions: Vec::new() })
    }
}

// Value may return `None`, even from panic hook, if called from a non-Godot thread.
pub fn get_gdext_panic_context() -> Option<String> {
    #[cfg(all(debug_assertions, not(wasm_nothreads)))]
    return ERROR_CONTEXT_STACK.with(|cell| cell.borrow().get_last());

    #[cfg(not(all(debug_assertions, not(wasm_nothreads))))]
    None
}

/// Executes `code`. If a panic is thrown, it is caught and an error message is printed to Godot.
///
/// Returns `Err(message)` if a panic occurred, and `Ok(result)` with the result of `code` otherwise.
///
/// In contrast to [`handle_varcall_panic`] and [`handle_ptrcall_panic`], this function is not intended for use in `try_` functions,
/// where the error is propagated as a `CallError` in a global variable.
pub fn handle_panic<E, F, R>(error_context: E, code: F) -> Result<R, String>
where
    E: Fn() -> String,
    F: FnOnce() -> R + std::panic::UnwindSafe,
{
    #[cfg(not(all(debug_assertions, not(wasm_nothreads))))]
    let _ = error_context; // Unused in Release or `wasm_nothreads` builds.

    #[cfg(all(debug_assertions, not(wasm_nothreads)))]
    ERROR_CONTEXT_STACK.with(|cell| unsafe {
        // SAFETY: &error_context is valid for lifetime of function, and is removed from LAST_ERROR_CONTEXT before end of function.
        cell.borrow_mut().push_function(&error_context)
    });

    let result =
        std::panic::catch_unwind(code).map_err(|payload| extract_panic_message(payload.as_ref()));

    #[cfg(all(debug_assertions, not(wasm_nothreads)))]
    ERROR_CONTEXT_STACK.with(|cell| cell.borrow_mut().pop_function());
    result
}

// TODO(bromeon): make call_ctx lazy-evaluated (like error_ctx) everywhere;
// or make it eager everywhere and ensure it's cheaply constructed in the call sites.
pub fn handle_varcall_panic<F, R>(
    call_ctx: &CallContext,
    out_err: &mut sys::GDExtensionCallError,
    code: F,
) where
    F: FnOnce() -> Result<R, CallError> + std::panic::UnwindSafe,
{
    let outcome: Result<Result<R, CallError>, String> =
        handle_panic(|| format!("{call_ctx}"), code);

    let call_error = match outcome {
        // All good.
        Ok(Ok(_result)) => return,

        // Call error signalled by Godot's or gdext's validation.
        Ok(Err(err)) => err,

        // Panic occurred (typically through user): forward message.
        Err(panic_msg) => CallError::failed_by_user_panic(call_ctx, panic_msg),
    };

    let error_id = report_call_error(call_error, true);

    // Abuse 'argument' field to store our ID.
    *out_err = sys::GDExtensionCallError {
        error: sys::GODOT_RUST_CUSTOM_CALL_ERROR,
        argument: error_id,
        expected: 0,
    };

    //sys::interface_fn!(variant_new_nil)(sys::AsUninit::as_uninit(ret));
}

pub fn handle_ptrcall_panic<F, R>(call_ctx: &CallContext, code: F)
where
    F: FnOnce() -> R + std::panic::UnwindSafe,
{
    let outcome: Result<R, String> = handle_panic(|| format!("{call_ctx}"), code);

    let call_error = match outcome {
        // All good.
        Ok(_result) => return,

        // Panic occurred (typically through user): forward message.
        Err(panic_msg) => CallError::failed_by_user_panic(call_ctx, panic_msg),
    };

    let _id = report_call_error(call_error, false);
}

fn report_call_error(call_error: CallError, track_globally: bool) -> i32 {
    // Print failed calls to Godot's console.
    // TODO Level 1 is not yet set, so this will always print if level != 0. Needs better logic to recognize try_* calls and avoid printing.
    // But a bit tricky with multiple threads and re-entrancy; maybe pass in info in error struct.
    if has_error_print_level(2) {
        godot_error!("{call_error}");
    }

    // Once there is a way to auto-remove added errors, this could be always true.
    if track_globally {
        call_error_insert(call_error)
    } else {
        0
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::{CallError, CallErrors};
    use crate::meta::CallContext;

    fn make(index: usize) -> CallError {
        let method_name = format!("method_{index}");
        let ctx = CallContext::func("Class", &method_name);
        CallError::failed_by_user_panic(&ctx, "some panic reason".to_string())
    }

    #[test]
    fn test_call_errors() {
        let mut store = CallErrors::default();

        let mut id07 = 0;
        let mut id13 = 0;
        let mut id20 = 0;
        for i in 0..24 {
            let id = store.insert(make(i));
            match i {
                7 => id07 = id,
                13 => id13 = id,
                20 => id20 = id,
                _ => {}
            }
        }

        let e = store.remove(id20).expect("must be present");
        assert_eq!(e.method_name(), "method_20");

        let e = store.remove(id20);
        assert!(e.is_none());

        for i in 24..CallErrors::MAX_ENTRIES as usize {
            store.insert(make(i));
        }
        for i in 0..10 {
            store.insert(make(i));
        }

        let e = store.remove(id07);
        assert!(e.is_none(), "generation overwritten");

        let e = store.remove(id13).expect("generation not yet overwritten");
        assert_eq!(e.method_name(), "method_13");
    }
}
