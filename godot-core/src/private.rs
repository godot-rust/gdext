/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::cell::Cell;
#[cfg(safeguards_strict)]
use std::cell::RefCell;
use std::io::Write;
use std::sync::atomic;

use crate::global::godot_error;
use crate::meta::error::{CallError, CallResult};
use crate::obj::Gd;
use crate::registry::property::Var;
use crate::{classes, sys};

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Public re-exports

mod reexport_pub {
    pub use crate::arg_into_owned;
    #[cfg(all(since_api = "4.3", feature = "register-docs"))]
    pub use crate::docs::{DocsItem, DocsPlugin, InherentImplDocs, StructDocs};
    pub use crate::r#gen::classes::class_macros;
    pub use crate::r#gen::virtuals; // virtual fn names, hashes, signatures
    pub use crate::meta::private_reexport::*;
    #[cfg(feature = "trace")]
    pub use crate::meta::{CowArg, FfiArg, trace};
    pub use crate::obj::rtti::ObjectRtti;
    pub use crate::obj::signal::priv_re_export::*;
    pub use crate::registry::callbacks;
    pub use crate::registry::plugin::{
        ClassPlugin, DynTraitImpl, ErasedDynGd, ErasedRegisterFn, ITraitImpl, InherentImpl,
        PluginItem, Struct,
    };
    pub use crate::storage::{
        IntoVirtualMethodReceiver, RecvGdSelf, RecvMut, RecvRef, Storage, VirtualMethodReceiver,
        as_storage,
    };
    pub use crate::sys::out;
}
pub use reexport_pub::*;

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Global variables

sys::atomic_enum! {
    #[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Debug)]
    pub enum ErrorPrintLevel {
        /// All errors are printed (default).
        Normal = 2,
        /// Reserved for future use; intended for `try_` call sites where errors are expected and printing is noisy.
        Reduced = 1,
        /// No error printing; used during `expect_panic` in tests.
        Silent = 0,
    }
}

static ERROR_PRINT_LEVEL: sys::AtomicEnum<ErrorPrintLevel> = sys::AtomicEnum::default();

sys::plugin_registry!(pub __GODOT_PLUGIN_REGISTRY: ClassPlugin);
#[cfg(all(since_api = "4.3", feature = "register-docs"))]
sys::plugin_registry!(pub __GODOT_DOCS_REGISTRY: DocsPlugin);

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Call error handling

// Thread-local storage for rich `CallError` produced by `#[func]` methods returning `Result<T, E>`.
//
// When a Rust `#[func]` fails (returns Err), the error is stashed here so that Rust's `try_call()` can retrieve it
// after the Godot round-trip. The varcall FFI callback simultaneously sets `CALL_FAILED_STATUS` so that Godot's own
// GDScript VM recognizes the failure and aborts the calling script function.
//
// Thread-safety: varcall callbacks execute on the calling thread, and `try_call` reads the result on the same
// thread before any other call can overwrite it. No mutex is needed.
thread_local! {
    static LAST_CALL_ERROR: Cell<Option<CallError>> = const { Cell::new(None) };

    // Depth of active Rust-initiated out-calls to Godot on the class out-call path (`out_class_varcall`, reached via
    // `call`/`try_call`). When > 0, we're waiting for an FFI round-trip. If Godot re-enters Rust and a `#[func]` fails,
    // the Rust caller will observe the error via the `CallResult`/`CallError` return -- so the in-Godot print would
    // just be noise. Panic prints are still emitted (backtrace info is worth keeping regardless of out-call context).
    static OUT_CALL_DEPTH: Cell<u32> = const { Cell::new(0) };
}

/// Store a [`CallError`] in thread-local storage for later retrieval by [`call_error_take`].
fn call_error_store(err: CallError) {
    LAST_CALL_ERROR.set(Some(err));
}

/// Take the [`CallError`] previously stored by [`call_error_store`], if any.
///
/// Returns `None` if no error was stored (i.e. the failure originated from Godot, not from gdext).
pub(crate) fn call_error_take() -> Option<CallError> {
    LAST_CALL_ERROR.take()
}

/// RAII guard marking that a Rust-initiated out-call to Godot is in progress on this thread.
///
/// While any guard is live, inbound `#[func]` failures on the same thread skip their `godot_error!` print, since the Rust
/// caller already observes the failure via the returned `CallError`/panic and the extra print would be redundant noise.
pub(crate) struct OutCallGuard;

impl OutCallGuard {
    #[must_use = "guard must be bound to a local; dropping it immediately ends the out-call scope"]
    pub fn new() -> Self {
        OUT_CALL_DEPTH.with(|d| d.set(d.get() + 1));
        Self
    }
}

impl Drop for OutCallGuard {
    fn drop(&mut self) {
        OUT_CALL_DEPTH.with(|d| d.set(d.get() - 1));
    }
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

#[cfg(all(since_api = "4.3", feature = "register-docs"))]
pub(crate) fn iterate_docs_plugins(mut visitor: impl FnMut(&DocsPlugin)) {
    sys::plugin_foreach!(__GODOT_DOCS_REGISTRY; visitor);
}

#[cfg(feature = "codegen-full")] // Remove if used in other scenarios.
pub(crate) fn find_inherent_impl(class_name: crate::meta::ClassId) -> Option<InherentImpl> {
    // We do this manually instead of using `iterate_plugins()` because we want to break as soon as we find a match.
    let plugins = __GODOT_PLUGIN_REGISTRY.lock().unwrap();

    plugins.iter().find_map(|elem| {
        if elem.class_name == class_name
            && let PluginItem::InherentImpl(inherent_impl) = &elem.item
        {
            return Some(inherent_impl.clone());
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
// Type-checkers for user-defined getters/setters in Var

// These functions are used to generate nice error messages if a #[var(get)], [var(get = my_getter)] etc. mismatches types.
// Don't modify without thorough UX testing; the use of `impl Fn` vs. `fn` is deliberate.
pub fn typecheck_getter<C, T: Var>(_getter: impl Fn(&C) -> T::PubType) {}
pub fn typecheck_setter<C, T: Var>(_setter: fn(&mut C, T::PubType)) {}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Capability queries and internal access

pub fn auto_init<T>(l: &mut crate::obj::OnReady<T>, base: &Gd<classes::Node>) {
    l.init_auto(base);
}

#[cfg(since_api = "4.3")]
pub unsafe fn has_virtual_script_method(
    object_ptr: sys::GDExtensionObjectPtr,
    method_sname: sys::GDExtensionConstStringNamePtr,
) -> bool {
    unsafe {
        sys::interface_fn!(object_has_script_method)(sys::to_const_ptr(object_ptr), method_sname)
            != 0
    }
}

/// Ensure `T` is an editor plugin.
pub const fn is_editor_plugin<T: crate::obj::Inherits<classes::EditorPlugin>>() {}

// Starting from 4.3, Godot has "runtime classes"; this emulation is no longer needed.
#[cfg(before_api = "4.3")]
pub fn is_class_inactive(is_tool: bool) -> bool {
    if is_tool {
        return false;
    }

    // SAFETY: only invoked after global library initialization.
    let global_config = unsafe { sys::config() };

    // Unknown is unreachable here: virtual dispatch only runs post-registration, by which point editor state is populated
    // (InitLevel::Scene on Godot < 4.4). `false` is a safe fallback.
    global_config.tool_only_in_editor && sys::is_editor_or_unknown().unwrap_or(false)
}

// Starting from 4.3, Godot has "runtime classes"; we only need to check whether editor is running.
// Runtime classes only get placeholder instances in the editor (no Rust constructor is called). `bind()` panics if called on placeholders.
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

/// Converts a default parameter value to a runtime-immutable `Variant`.
///
/// This function is used internally by the `#[opt(default)]` attribute to:
/// 1. Convert the value using `AsArg` trait for argument conversions (e.g. `"str"` for `AsArg<GString>`).
/// 2. Apply immutability transformation.
/// 3. Convert to `Variant` for Godot's storage.
pub fn opt_default_value<T>(value: impl crate::meta::AsArg<T>) -> crate::builtin::Variant
where
    T: crate::meta::GodotImmutable + crate::meta::ToGodot + Clone,
{
    // We currently need cow_into_owned() to create an owned value for the immutability transform. This may be revisited once `#[opt]`
    // supports more types (e.g. `Gd<RefCounted>`, where `cow_into_owned()` would increment ref-counts).

    let value = crate::meta::AsArg::<T>::into_arg(value);
    let value = value.cow_into_owned();
    let value = <T as crate::meta::GodotImmutable>::into_runtime_immutable(value);
    crate::builtin::Variant::from(value)
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Panic *hook* management

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

    if let Some(context) = fetch_last_panic_context() {
        msg = format!("{msg}\nin {context}"); // used to be "Context: {context}".
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
#[cfg(safeguards_strict)]
#[macro_export]
macro_rules! format_backtrace {
    ($prefix:expr_2021, $backtrace:expr_2021) => {{
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

    ($prefix:expr_2021) => {
        $crate::format_backtrace!($prefix, std::backtrace::Backtrace::capture())
    };
}

#[cfg(not(safeguards_strict))]
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

pub fn set_error_print_level(level: ErrorPrintLevel) -> ErrorPrintLevel {
    ERROR_PRINT_LEVEL.replace(level)
}

pub(crate) fn has_error_print_level(level: ErrorPrintLevel) -> bool {
    ERROR_PRINT_LEVEL.load() >= level
}

/// Internal type used to store context information for debug purposes. Debug context is stored on the thread-local
/// ERROR_CONTEXT_STACK, which can later be used to retrieve the current context in the event of a panic. This value
/// probably shouldn't be used directly; use ['get_gdext_panic_context()'](fetch_last_panic_context) instead.
#[cfg(safeguards_strict)]
struct ScopedFunctionStack {
    functions: Vec<*const dyn Fn() -> String>,
}

#[cfg(safeguards_strict)]
impl ScopedFunctionStack {
    /// # Safety
    /// Function must be removed (using [`pop_function()`](Self::pop_function)) before lifetime is invalidated.
    unsafe fn push_function<'a, 'b>(&'a mut self, function: &'b (dyn Fn() -> String + 'b)) {
        // SAFETY: Function has its lifetime `'b` extended to `'static` to satisfy the signature
        // of `functions` which has an implied `'static` bound.
        // Given function must be removed before lifetime `'b` is invalidated.
        let function = unsafe {
            std::mem::transmute::<
                *const (dyn Fn() -> String + 'b),
                *const (dyn Fn() -> String + 'static),
            >(function)
        };

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

#[cfg(safeguards_strict)]
thread_local! {
    static ERROR_CONTEXT_STACK: RefCell<ScopedFunctionStack> = const {
        RefCell::new(ScopedFunctionStack { functions: Vec::new() })
    }
}

// Value may return `None`, even from panic hook, if called from a non-Godot thread.
pub fn fetch_last_panic_context() -> Option<String> {
    #[cfg(safeguards_strict)]
    return ERROR_CONTEXT_STACK.with(|cell| cell.borrow().get_last());

    #[cfg(not(safeguards_strict))]
    None
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Panic unwinding and catching

pub struct PanicPayload {
    payload: Box<dyn std::any::Any + Send + 'static>,
}

impl PanicPayload {
    pub fn new(payload: Box<dyn std::any::Any + Send + 'static>) -> Self {
        Self { payload }
    }

    // While this could be `&self`, it's usually good practice to pass panic payloads around linearly and have only 1 representation at a time.
    pub fn into_panic_message(self) -> String {
        extract_panic_message(self.payload.as_ref())
    }

    pub fn repanic(self) -> ! {
        std::panic::resume_unwind(self.payload)
    }
}

/// Executes `code`. If a panic is thrown, it is caught and an error message is printed to Godot.
///
/// Returns `Err(message)` if a panic occurred, and `Ok(result)` with the result of `code` otherwise.
///
/// In contrast to [`handle_fallible_varcall`] and [`handle_fallible_ptrcall`], this function is not intended for use in `try_` functions,
/// where the error is propagated as a `CallError` in a global variable.
pub fn handle_panic<E, F, R>(error_context: E, code: F) -> Result<R, PanicPayload>
where
    E: Fn() -> String,
    F: FnOnce() -> R + std::panic::UnwindSafe,
{
    #[cfg(not(safeguards_strict))]
    let _ = error_context; // Unused in Release.

    #[cfg(safeguards_strict)]
    ERROR_CONTEXT_STACK.with(|cell| unsafe {
        // SAFETY: &error_context is valid for lifetime of function, and is removed from LAST_ERROR_CONTEXT before end of function.
        cell.borrow_mut().push_function(&error_context)
    });

    let result = std::panic::catch_unwind(code).map_err(PanicPayload::new);

    #[cfg(safeguards_strict)]
    ERROR_CONTEXT_STACK.with(|cell| cell.borrow_mut().pop_function());
    result
}

// Error code set on the varcall output when a `#[func]` fails (panic, parameter conversion, or `Result<T, E>` returning `Err`).
//
// None of the existing GDExtension call errors is great for this scenario -- all lead to misleading messages in the Godot console.
// A custom out-of-range value causes "Bug: Invalid call error code 1337." in Godot's output, which is at least clearly non-standard.
// Note that INVALID_METHOD must not be used: it signals that the method doesn't exist, which GDScript may treat as a fatal static error.
// An alternative would be GDEXTENSION_CALL_ERROR_INSTANCE_IS_NULL.
//
// The GDScript VM interprets any non-OK code as "call failed, abort calling function", which is what we want. The "Bug: ..." print is
// unavoidable at the VM level (no GDExtension code maps to a clean message); the preceding godot-rust `CallError` print carries the
// actual diagnostic information.
const CALL_FAILED_STATUS: sys::GDExtensionCallErrorType = 1337;

/// Invokes a function with the _varcall_ calling convention, handling both expected errors and user panics.
pub fn handle_fallible_varcall<F, R>(
    call_ctx: &CallContext,
    out_err: &mut sys::GDExtensionCallError,
    code: F,
) where
    F: FnOnce() -> CallResult<R> + std::panic::UnwindSafe,
{
    if handle_fallible_call(call_ctx, code) {
        // Use CALL_FAILED_STATUS so the GDScript VM recognizes the failure and aborts the calling function.
        // The Rust-side CallError has been stored in the thread-local, so that try_call() can retrieve it later.
        *out_err = sys::GDExtensionCallError {
            error: CALL_FAILED_STATUS,
            argument: 0,
            expected: 0,
        };
    };
}

/// Invokes a function with the _ptrcall_ calling convention, handling both expected errors and user panics.
pub fn handle_fallible_ptrcall<F>(call_ctx: &CallContext, code: F)
where
    F: FnOnce() -> CallResult<()> + std::panic::UnwindSafe,
{
    handle_fallible_call(call_ctx, code);
}

/// Common error handling for fallible calls, handling detectable errors and user panics.
///
/// Returns `true` if the call failed, `false` if it succeeded.
///
/// On failure, the [`CallError`] is stored in thread-local storage for later retrieval via [`call_error_take`].
fn handle_fallible_call<F, R>(call_ctx: &CallContext, code: F) -> bool
where
    F: FnOnce() -> CallResult<R> + std::panic::UnwindSafe,
{
    let outcome: Result<CallResult<R>, PanicPayload> =
        handle_panic(|| format!("{call_ctx}()"), code);

    let call_error = match outcome {
        // All good.
        Ok(Ok(_result)) => return false,

        // Error from Godot or godot-rust validation (e.g. parameter conversion).
        Ok(Err(err)) => err,

        // User panic occurred: forward message.
        Err(panic_msg) => CallError::failed_by_user_panic(call_ctx, panic_msg),
    };

    // Print failed calls to Godot's console.
    //
    // OUT_CALL_DEPTH > 0 means this failure is observed during a Rust-initiated out-call (e.g. `try_call`); the caller already sees
    // the `CallError` via return value, so printing here would just be noise.
    //
    // Coverage gap: only `Signature::out_class_varcall` sets the guard. If a `#[func]` re-enters Rust via `out_utility_call`,
    // `out_builtin_ptrcall`, or `out_script_virtual_call`, the redundant print returns. Extend the guard to those paths if reported.
    //
    // caused_by_panic() check to avoid printing (2) once the panic message (1) is already printed:
    //
    // (1)  ERROR: [panic hot-reload/rust/src/lib.rs:37]
    //      some panic message
    //      Context: MyClass::my_method
    //       at: godot_core::private::set_gdext_hook::{{closure}} (/.../godot-core/src/private.rs:354)
    //       GDScript backtrace (most recent call first):
    //           [0] _ready (res://script.gd:9)
    //     (backtrace disabled, run application with `RUST_BACKTRACE=1` environment variable)
    //
    // (2) ERROR: godot-rust function call failed: MyClass::my_method()
    //        Reason: function panicked: some panic message
    //     at: ...
    if has_error_print_level(ErrorPrintLevel::Normal)
        && !call_error.caused_by_panic()
        && OUT_CALL_DEPTH.with(|d| d.get() == 0)
    {
        godot_error!("{call_error}");
    }

    call_error_store(call_error);
    true
}

// Currently unused; implemented due to temporary need and may come in handy.
pub fn rebuild_gd(object_ref: &classes::Object) -> Gd<classes::Object> {
    let ptr = object_ref.__object_ptr();

    // SAFETY: ptr comes from valid internal API (and is non-null, so unwrap in from_obj_sys won't fail).
    unsafe { Gd::from_obj_sys(ptr) }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::{CallError, PanicPayload, call_error_store, call_error_take};
    use crate::meta::CallContext;
    use crate::sys;

    fn make(index: usize) -> CallError {
        let method_name = format!("method_{index}");
        let ctx = CallContext::func("Class", &method_name);
        let payload = PanicPayload::new(Box::new("some panic reason".to_string()));

        CallError::failed_by_user_panic(&ctx, payload)
    }

    #[test]
    fn thread_local_store_and_take() {
        // Initially empty.
        assert!(call_error_take().is_none());

        // Store, then take.
        call_error_store(make(1));
        let e = call_error_take().expect("must be present");
        assert_eq!(e.method_name(), "method_1");

        // Second take returns None.
        assert!(call_error_take().is_none());
    }

    #[test]
    fn thread_local_overwrite() {
        // Storing twice overwrites the first.
        call_error_store(make(1));
        call_error_store(make(2));
        let e = call_error_take().expect("must be present");
        assert_eq!(e.method_name(), "method_2");

        assert!(call_error_take().is_none());
    }

    /// Regression test: a stale TLS entry from an earlier `#[func]` failure must not be misattributed to a later, unrelated varcall failure.
    /// `check_out_varcall` drains TLS unconditionally, so a Godot-side error (e.g. wrong arg count) after a stale store must *not* wrap the
    /// stale error. "TLS" means thread-local storage.
    #[test]
    fn stale_tls_not_misattributed() {
        use crate::meta::error::CallError;

        // Simulate a previous #[func] failure that was never consumed (e.g. GDScript was the caller).
        call_error_store(make(99));

        // Simulate a subsequent varcall that succeeds -- TLS must be drained.
        let call_ctx = CallContext::outbound("Object", "call");
        let ok_err = sys::GDExtensionCallError {
            error: sys::GDEXTENSION_CALL_OK,
            argument: 0,
            expected: 0,
        };
        let result =
            CallError::check_out_varcall(&call_ctx, ok_err, &[] as &[crate::builtin::Variant], &[]);
        assert!(result.is_ok(), "successful call must return Ok");

        // TLS must now be empty.
        assert!(
            call_error_take().is_none(),
            "TLS must be drained after check_out_varcall"
        );
    }

    /// Verify that when a varcall fails with a Godot-side error and there is *no* stale TLS entry,
    /// the error is decoded from the Godot error struct (no source wrapping).
    #[test]
    fn varcall_godot_error_without_tls() {
        use std::error::Error as _;

        use crate::meta::error::CallError;

        // Ensure TLS is clean.
        let _ = call_error_take();

        let call_ctx = CallContext::outbound("Node", "rpc_config");
        let godot_err = sys::GDExtensionCallError {
            error: sys::GDEXTENSION_CALL_ERROR_TOO_FEW_ARGUMENTS,
            argument: 2,
            expected: 3,
        };
        let result = CallError::check_out_varcall(
            &call_ctx,
            godot_err,
            &[] as &[crate::builtin::Variant],
            &[],
        );
        let err = result.expect_err("must fail");

        // Must be a direct Godot error, not a wrapped source error.
        assert!(
            err.source().is_none(),
            "Godot-side error must not have a source (stale or otherwise)"
        );
    }
}
