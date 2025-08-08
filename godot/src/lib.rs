/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! # Rust bindings for Godot 4
//!
//! The **gdext** library implements Rust bindings for the [Godot](https://godotengine.org) engine, more precisely its version 4.
//! It does so using the GDExtension API, a C interface to integrate third-party language bindings with the engine.
//!
//! This API doc is accompanied by the [book](https://github.com/godot-rust/book), which provides tutorials
//! that guide you along the way.
//!
//! An overview of fundamental types and concepts can be found on [this page](__docs).
//!
//!
//! ## Module organization
//!
//! The contains generated code, which is derived from the GDExtension API specification. This code spans the official Godot API and
//! is mostly the same as the API you would use in GDScript.
//!
//! The Godot API is divided into several modules:
//!
//! * [`builtin`]: Built-in types, such as `Vector2`, `Color`, and `String`.
//! * [`classes`]: Godot classes, such as `Node`, `RefCounted` or `Resource`.
//! * [`global`]: Global functions and enums, such as `godot_print!`, `smoothstep` or `JoyAxis`.
//!
//! In addition to generated code, we provide a framework that allows you to easily interface the Godot engine.
//! Noteworthy modules in this context are:
//!
//! * [`register`], used to register **your own** Rust symbols (classes, methods, constants etc.) with Godot.
//! * [`obj`], everything related to handling Godot objects, such as the `Gd<T>` type.
//! * [`tools`], higher-level utilities that extend the generated code, e.g. `load<T>()`.
//! * [`meta`], fundamental information about types, properties and conversions.
//! * [`init`], entry point and global library configuration.
//! * [`task`], integration with async code.
//!
//! The [`prelude`] contains often-imported symbols; feel free to `use godot::prelude::*` in your code.
//! <br><br>
//!
//!
//! ## Public API
//!
//! Some symbols in the API are not intended for users, however Rust's visibility feature is not strong enough to express that in all cases
//! (for example, proc-macros and separated crates may need access to internals).
//!
//! The following API symbols are considered private:
//!
//! * Symbols annotated with `#[doc(hidden)]`.
//! * Any of the dependency crates (crate `godot` is the only public interface).
//! * Modules named `private` and all their contents.
//!
//! This means there are **no guarantees** regarding API stability, robustness or correctness. Problems arising from using private APIs are
//! not considered bugs, and anything relying on them may stop working without announcement. Please refrain from using undocumented and
//! private features; if you are missing certain functionality, bring it up for discussion instead. This allows us to improve the library!
//! <br><br>
//!
//!
//! ## Cargo features
//!
//! The following features can be enabled for this crate. All of them are off by default.
//!
//! Avoid `default-features = false` unless you know exactly what you are doing; it will disable some required internal features.
//!
//! _Godot version and configuration:_
//!
//! * **`api-4-{minor}`**
//! * **`api-4-{minor}-{patch}`**
//! * **`api-custom`**
//! * **`api-custom-json`**
//!
//!   Sets the [**API level**](https://godot-rust.github.io/book/toolchain/godot-version.html) to the specified Godot version,
//!   or a custom-built local binary.
//!   You can use at most one `api-*` feature. If absent, the current Godot minor version is used, with patch level 0.
//!
//!   `api-custom` feature requires specifying `GODOT4_BIN` environment variable with a path to your Godot4 binary.
//!
//!   The `api-custom-json` feature requires specifying `GODOT4_GDEXTENSION_JSON` environment variable with a path
//!   to your custom-defined `extension_api.json`.<br><br>
//!
//! * **`double-precision`**
//!
//!   Use `f64` instead of `f32` for the floating-point type [`real`][type@builtin::real]. Requires Godot to be compiled with the
//!   scons flag `precision=double`.<br><br>
//!
//! * **`experimental-godot-api`**
//!
//!   Access to `godot::classes` APIs that Godot marks "experimental". These are under heavy development and may change at any time.
//!   If you opt in to this feature, expect breaking changes at compile and runtime.
//!
//! _Rust functionality toggles:_
//!
//! * **`lazy-function-tables`**
//!
//!   Instead of loading all engine function pointers at startup, load them lazily on first use. This reduces startup time and RAM usage, but
//!   incurs additional overhead in each FFI call. Also, you lose the guarantee that once the library has booted, all function pointers are
//!   truly available. Function calls may thus panic only at runtime, possibly in deeply nested code paths.
//!   This feature is not yet thread-safe and can thus not be combined with `experimental-threads`.<br><br>
//!
//! * **`experimental-threads`**
//!
//!   Experimental threading support. This adds synchronization to access the user instance in `Gd<T>` and disables several single-thread checks.
//!   The safety aspects are not ironed out yet; there is a high risk of unsoundness at the moment.
//!   As this evolves, it is very likely that the API becomes stricter.<br><br>
//!
//! * **`experimental-wasm`**
//!
//!   Support for WebAssembly exports is still a work-in-progress and is not yet well tested. This feature is in place for users
//!   to explicitly opt in to any instabilities or rough edges that may result.
//!
//!   Please read [Export to Web](https://godot-rust.github.io/book/toolchain/export-web.html) in the book.
//!
//!   By default, Wasm threads are enabled and require the flag `"-C", "link-args=-pthread"` in the `wasm32-unknown-unknown` target.
//!   This must be kept in sync with Godot's Web export settings (threading support enabled). To disable it, use **additionally* the feature
//!   `experimental-wasm-nothreads`.<br><br>
//!
//!   It is recommended to use this feature in combination with `lazy-function-tables` to reduce the size of the generated Wasm binary.
//!
//! * **`experimental-wasm-nothreads`**
//!
//!   Requires the `experimental-wasm` feature. Disables threading support for WebAssembly exports. This needs to be kept in sync with
//!   Godot's Web export setting (threading support disabled), and must _not_ use the `"-C", "link-args=-pthread"` flag in the
//!   `wasm32-unknown-unknown` target.<br><br>
//!
//! * **`codegen-rustfmt`**
//!
//!   Use rustfmt to format generated binding code. Because rustfmt is so slow, this is detrimental to initial compile time.
//!   Without it, we use a lightweight and fast custom formatter to enable basic human readability.<br><br>
//!
//! * **`register-docs`**
//!
//!   Generates documentation for your structs from your Rust documentation.
//!   Documentation is visible in Godot via `F1` -> searching for that class.
//!   This feature requires at least Godot 4.3.
//!   See also: [`#[derive(GodotClass)]`](register/derive.GodotClass.html#documentation)
//!
//! _Integrations:_
//!
//! * **`serde`**
//!
//!   Implement the [serde](https://serde.rs/) traits `Serialize` and `Deserialize` traits for certain built-in types.
//!   The serialized representation underlies **no stability guarantees** and may change at any time, even without a SemVer-breaking change.
//!

#![doc(
    html_logo_url = "https://raw.githubusercontent.com/godot-rust/assets/master/gdext/ferris.svg"
)]

#[cfg(doc)]
pub mod __docs;

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Validations

// Many validations are moved to godot-ffi. #[cfg]s are not emitted in this crate, so move checks for those up to godot-core.

#[cfg(all(target_family = "wasm", not(feature = "experimental-wasm")))]
compile_error!(
    "Wasm target requires opt-in via `experimental-wasm` Cargo feature;\n\
    keep in mind that this is work in progress."
);

// See also https://github.com/godotengine/godot/issues/86346.
// Could technically be moved to godot-codegen to reduce time-to-failure slightly, but would scatter validations even more.
#[cfg(all(
    feature = "double-precision",
    not(feature = "api-custom"),
    not(feature = "api-custom-json")
))]
compile_error!("The feature `double-precision` currently requires `api-custom` or `api-custom-json` due to incompatibilities in the GDExtension API JSON. \
See: https://github.com/godotengine/godot/issues/86346");

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Modules

#[doc(hidden)]
pub use godot_core::possibly_docs as docs;
#[doc(hidden)]
pub use godot_core::sys;
#[doc(inline)]
pub use godot_core::{builtin, classes, global, meta, obj, task, tools};

/// Entry point and global init/shutdown of the library.
pub mod init {
    pub use godot_core::init::*;
    // Re-exports
    pub use godot_macros::gdextension;
}

/// Register/export Rust symbols to Godot: classes, methods, enums...
pub mod register {
    pub use godot_core::registry::property;
    pub use godot_core::registry::signal::re_export::*;
    #[cfg(feature = "__codegen-full")]
    pub use godot_core::registry::RpcConfig;
    pub use godot_macros::{godot_api, godot_dyn, Export, GodotClass, GodotConvert, Var};

    /// Re-exports used by proc-macro API.
    #[doc(hidden)]
    pub mod private {
        #[cfg(feature = "__codegen-full")]
        pub use godot_core::registry::class::auto_register_rpcs;
        pub use godot_core::registry::godot_register_wrappers::*;
        pub use godot_core::registry::{constant, method};
    }
}

/// Testing facilities (unstable).
#[doc(hidden)]
pub mod test {
    pub use godot_macros::{bench, itest};
}

#[doc(hidden)]
pub use godot_core::__deprecated;
#[doc(hidden)]
pub use godot_core::private;

/// Often-imported symbols.
pub mod prelude;
