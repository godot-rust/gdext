/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::error::Error;
use std::fmt;

use crate::meta::ToGodot;
use crate::meta::error::{CallOutcome, ErrorToGodot};

/// Ergonomic catch-all error type for `#[func]` methods.
///
/// `strat::Unexpected` is intended for potential bugs that are **fixed during development**. They should not appear in Release builds.  \
/// Do **not** use this for runtime errors that are expected (e.g. loading a savegame that can be corrupted).
///
/// When this error is returned, Godot logs it with [`godot_error!`]. The calling code [cannot reliably handle it][godot-proposal-7751].
/// This strategy is comparable to panics in Rust: the calling code is more ergonomic in the happy path, assuming that there are no errors.
/// In case that `Err(strat::Unexpected)` is returned, the calling code either aborts the function (debug varcall only) or continues with a
/// default value of the declared return type, which may introduce silent logic errors. GDScript has best-effort detection of such errors in Debug
/// mode; see [below](#behavior-on-the-call-site). The Rust object and its state remain valid after an error and can be used in subsequent calls.
///
/// This is currently the only [`ErrorToGodot`] impl that preserves type safety: for a Rust `#[func]` returning `Result<T, strat::Unexpected>`,
/// GDScript's static analysis sees the return type `T`. If you want to handle errors at runtime, choose another `ErrorToGodot` impl.
///
/// `strat::Unexpected` enables automatic conversions from other errors via `?` operator. This means you can mix different error types within the
/// same function body -- each one propagates via `?` and its message is forwarded to Godot. Use the [`func_bail!`] macro for early returns with
/// an error message.
///
/// [godot-proposal-7751]: https://github.com/godotengine/godot-proposals/discussions/7751
/// [`godot_error!`]: crate::global::godot_error
/// [`func_bail!`]: crate::meta::error::func_bail
///
/// # Example
/// ```no_run
/// use godot::prelude::*;
/// # #[derive(GodotClass)] #[class(init, base=Node3D)]
/// # struct PlayerCharacter { base: Base<Node3D>, config_map: std::collections::HashMap<String, String>, id: i32 }
///
/// #[godot_api]
/// impl PlayerCharacter {
///     // Verifies that required nodes are present in the developer-authored scene tree.
///     // Missing nodes are a scene setup bug, not an expected runtime condition.
///     #[func]
///     fn init_player(&mut self) -> Result<(), strat::Unexpected> {
///         // Node missing = scene setup bug.
///         let Some(hand) = self.base().try_get_node_as::<Node3D>("Skeleton3D/Hand") else {
///             func_bail!("Player {}: 3D model is missing a hand", self.id);
///         };
///
///         // HashMap loaded at startup; missing or unparseable values are a code bug.
///         let max_health = self.config_map
///             .get("max_health")
///             .ok_or("'max_health' key missing in config")?  // &str
///             .parse::<i64>()?;                              // ParseIntError
///
///         // Initialize self with hand + max_health...
///         Ok(())
///     }
/// }
/// ```
///
/// This example uses [`Node::try_get_node_as()`][crate::classes::Node::try_get_node_as], the fallible counterpart to
/// [`Node::get_node_as()`][crate::classes::Node::get_node_as]. The latter panics on failure. From GDScript, the observable behavior is
/// the same, however the `try_*` + `?` approach allows more control over error propagation and works entirely without a Rust panic.
///
/// # Behavior on the call site
/// How a caller sees the `Err` variant of a returned `Result<T, strat::Unexpected>` depends:
///
/// - **Rust:** When calling a `#[func]` via `Object::try_call()` reflection, the caller will always see `Unexpected` errors manifest as `Err` in
///   the return type. This is the only way to reliably catch `Unexpected` errors, and works only because godot-rust maintains internal state.
/// - **GDScript:** If an `Unexpected` error is returned from Rust, the calling GDScript function will abort/fail if both conditions are true:
///   - the call uses _varcall_ (not _ptrcall_): invoked on an untyped `Variant` or using reflection via `Object.call()`.
///   - it runs in a Godot debug/editor build.
/// - Everything else returns Godot's default value for the type `T` (e.g. `null` for objects/variants, 0 for ints, etc.). This also applies to
///   other languages calling such a method (C#, godot-cpp, etc.).
// A small edge case seems to be a scene with a Rust class, that has a script attached. The GDScript _init() method then calls self.method(),
// which seems to behave like varcall rather than ptrcall, thus failing the call. Adding a `var x: MyClass = self; x.method()` however
// turns it into regular ptrcall, with continued execution and default value. Not yet reproduced in itest.
///
/// # `std::error::Error` and trait coherence
/// This type intentionally does **not** implement [`std::error::Error`] -- the reason is a Rust coherence constraint.
///
/// If `Unexpected` implemented `Error`, the blanket `impl<E: Into<Box<dyn Error + ...>>> From<E> for Unexpected` would conflict with the standard
/// library's `impl<T> From<T> for T`, because `Unexpected` would satisfy both `E = Unexpected` and `Unexpected: Into<Box<dyn Error>>`.
/// Omitting the `Error` impl sidesteps this conflict and is the same technique used by [`anyhow::Error`](https://docs.rs/anyhow).
///
/// Because `Unexpected` is typically the last error in a chain -- returned to Godot via `#[func]` -- the lack of direct error APIs is usually
/// not a big problem.
pub struct Unexpected {
    inner: Box<dyn Error + Send + Sync + 'static>,
}

impl Unexpected {
    /// Create a `Unexpected` from any type implementing [`std::error::Error`].
    pub fn new(err: impl Error + Send + Sync + 'static) -> Self {
        Self {
            inner: Box::new(err),
        }
    }

    /// Attempt to downcast the inner error to a concrete type.
    pub fn downcast_ref<E: Error + 'static>(&self) -> Option<&E> {
        self.inner.downcast_ref::<E>()
    }
}

impl fmt::Display for Unexpected {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.inner, f)
    }
}

impl fmt::Debug for Unexpected {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.inner, f)
    }
}

// Unexpected intentionally does NOT implement std::error::Error -- see the type-level docs for why.

/// Enables the `?` operator for any `E: Into<Box<dyn Error + Send + Sync + 'static>>`.
///
/// The following types satisfy this bound and can therefore be used with `?` or passed to [`Unexpected::new()`]:
///
/// | Source type | How it converts |
/// |---|---|
/// | Any `E: Error + Send + Sync + 'static` | Boxed directly; covers `std::io::Error`, `ParseIntError`, and any custom error type |
/// | `Box<dyn Error + Send + Sync + 'static>` | Used as-is |
/// | `String` | Wrapped in a message-only error |
/// | `&str` (any lifetime) | Copied to `String`, then wrapped -- the lifetime is not propagated |
impl<E: Into<Box<dyn Error + Send + Sync + 'static>>> From<E> for Unexpected {
    fn from(err: E) -> Self {
        Self { inner: err.into() }
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// ErrorToGodot impl -- unexpected mode

impl<T: ToGodot> ErrorToGodot<T> for Unexpected {
    type Mapped = T;

    fn result_to_godot(result: Result<T, Self>) -> CallOutcome<T> {
        match result {
            Ok(val) => CallOutcome::Return(val),
            Err(e) => CallOutcome::CallFailed(format!("Err(Unexpected) in #[func]: {e}")),
        }
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

#[cfg(test)] #[cfg_attr(published_docs, doc(cfg(test)))]
mod tests {
    use super::*;
    use crate::meta::error::func_bail;

    // Type-checks the argument.
    fn assert_bail(_: Unexpected) {}

    #[test]
    fn from_concrete_error() {
        // Accepts any E: Error + Send + Sync + 'static -- here ParseIntError.
        let err: Result<i32, _> = "x".parse();
        assert_bail(err.unwrap_err().into());
    }

    #[test]
    fn from_io_error() {
        let err = std::io::Error::other("io failure");
        assert_bail(err.into());
    }

    #[test]
    fn from_boxed_error() {
        let err: Box<dyn Error + Send + Sync + 'static> = Box::new(std::io::Error::other("boxed"));
        assert_bail(err.into());
    }

    #[test]
    fn from_string() {
        assert_bail(String::from("string error").into());
    }

    #[test]
    fn from_str_static() {
        assert_bail("static str error".into());
    }

    #[test]
    fn from_str_non_static() {
        // Non-static &str: the slice is copied to String internally, so the lifetime is not propagated.
        let s = String::from("runtime string");
        let slice: &str = s.as_str();
        assert_bail(slice.into());
    }

    #[test]
    fn display_forwards_inner_message() {
        let e: Unexpected = "hello error".into();
        assert_eq!(e.to_string(), "hello error");
    }

    #[test]
    fn macro_literal_returns_early() {
        fn run() -> Result<i32, Unexpected> {
            func_bail!("literal message");
        }
        let err = run().unwrap_err();
        assert_eq!(err.to_string(), "literal message");
    }

    #[test]
    fn macro_format_returns_early() {
        fn run(x: i32) -> Result<i32, Unexpected> {
            func_bail!("value was {x}");
        }
        let err = run(42).unwrap_err();
        assert_eq!(err.to_string(), "value was 42");
    }

    #[test]
    fn downcast_ref_succeeds() {
        let inner = std::io::Error::other("downcastable");
        let e: Unexpected = inner.into();
        assert!(e.downcast_ref::<std::io::Error>().is_some());
    }

    #[test]
    fn downcast_ref_wrong_type_returns_none() {
        let e: Unexpected = "not an io::Error".into();
        assert!(e.downcast_ref::<std::io::Error>().is_none());
    }
}
