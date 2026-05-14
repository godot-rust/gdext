/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! Editor-only tests for placeholder substitution of runtime classes.
//!
//! In the editor, Godot substitutes a `PlaceholderExtensionInstance` for runtime (non-`#[class(tool)]`) classes instead of running the
//! extension's `create_instance`. `Gd::new_alloc` / `Gd::new_gd` (via `Gd::default_instance`) used to bypass that substitution -- see
//! <https://github.com/godot-rust/gdext/issues/1404>.
//!
//! Godot keeps a per-class default-value cache, populated only with properties whose `PropertyUsageFlags` include editor/storage bits:
//! `#[export]` fields are in it (placeholder `get()` returns the default), `#[var]`-only fields are not (placeholder `get()` returns nil).
//! The cache is warmed at editor startup via doc generation, so the real `init()` has already run by the time these tests execute.

use std::panic;
use std::sync::atomic::{AtomicUsize, Ordering};

use godot::builtin::{GString, StringName, Variant};
use godot::classes::{INode, Node};
use godot::meta::ToGodot;
use godot::obj::{Base, NewAlloc};
use godot::register::{GodotClass, godot_api};

use crate::framework::{itest, suppress_panic_log};

static RUNTIME_INIT_COUNT: AtomicUsize = AtomicUsize::new(0);
static TOOL_INIT_COUNT: AtomicUsize = AtomicUsize::new(0);

/// Runtime (non-tool) class. Has both `#[var]` and `#[export]` to cover both default-value cache outcomes.
#[derive(GodotClass)]
#[class(base = Node)]
struct RuntimeProbe {
    #[var]
    var_field: i64,

    #[export]
    export_field: GString,
}

#[godot_api]
impl INode for RuntimeProbe {
    fn init(_base: Base<Node>) -> Self {
        RUNTIME_INIT_COUNT.fetch_add(1, Ordering::SeqCst);
        Self {
            var_field: 43,
            export_field: GString::from("hello"),
        }
    }

    fn on_property_get_revert(&self, property: StringName) -> Option<Variant> {
        (property == "var_field").then(|| 0.to_variant())
    }
}

/// Tool counterpart of [`RuntimeProbe`]: runs as a real instance in the editor.
#[derive(GodotClass)]
#[class(base = Node, tool)]
struct ToolProbe {
    #[var]
    var_field: i64,

    #[export]
    export_field: GString,
}

#[godot_api]
impl INode for ToolProbe {
    fn init(_base: Base<Node>) -> Self {
        TOOL_INIT_COUNT.fetch_add(1, Ordering::SeqCst);
        Self {
            var_field: 43,
            export_field: GString::from("hello"),
        }
    }

    fn on_property_get_revert(&self, property: StringName) -> Option<Variant> {
        (property == "var_field").then(|| 0.to_variant())
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Tests -- runtime class (placeholdered in editor)

#[itest(editor)]
fn editor_runtime_get_returns_cached_default() {
    let node = RuntimeProbe::new_alloc();

    assert_eq!(node.get("var_field"), Variant::nil());
    assert_eq!(node.get("export_field"), "hello".to_variant());

    node.free();
}

/// `PlaceholderExtensionInstance` stores property values in an internal map so the editor can preserve them while the extension is unavailable.
#[itest(editor)]
fn editor_runtime_set_get_roundtrip() {
    let mut node = RuntimeProbe::new_alloc();

    node.set("var_field", &99.to_variant());
    node.set("export_field", &"world".to_variant());

    assert_eq!(node.get("var_field"), 99.to_variant());
    assert_eq!(node.get("export_field"), "world".to_variant());

    node.free();
}

/// Placeholder set/get tolerate unknown names (generic property map), so `get()` on an unknown property returns nil rather than panicking.
#[itest(editor)]
fn editor_runtime_get_unknown_property() {
    let node = RuntimeProbe::new_alloc();
    assert_eq!(node.get("does_not_exist"), Variant::nil());
    node.free();
}

/// Property list comes from class metadata, not the instance, so all registered properties are listed.
#[itest(editor)]
fn editor_runtime_property_list_complete() {
    let node = RuntimeProbe::new_alloc();
    let prop_list = node.get_property_list();

    let has_name = |name: &str| {
        prop_list
            .iter_shared()
            .any(|d| d.get("name") == Some(name.to_variant()))
    };

    assert!(
        has_name("var_field"),
        "var_field missing from property list"
    );
    assert!(
        has_name("export_field"),
        "export_field missing from property list"
    );

    node.free();
}

/// Godot does not leak `PlaceholderExtensionInstance` -- class identity reports the user-defined name.
#[itest(editor)]
fn editor_runtime_class_identity() {
    let node = RuntimeProbe::new_alloc();

    assert_eq!(node.get_class(), GString::from("RuntimeProbe"));
    assert!(node.is_class("RuntimeProbe"));
    assert!(
        node.is_class("Node"),
        "is_class() must walk inheritance chain"
    );

    node.free();
}

/// Regression test for <https://github.com/godot-rust/gdext/issues/1404>: `new_alloc()` on a runtime class in editor must not run `init()`.
#[itest(editor)]
fn editor_runtime_init_not_called() {
    // Delta-based; doesn't care about #times that RuntimeProbe was constructed. Assumes serial itest execution.
    let before = RUNTIME_INIT_COUNT.load(Ordering::SeqCst);
    let node = RuntimeProbe::new_alloc();
    let delta = RUNTIME_INIT_COUNT.load(Ordering::SeqCst) - before;
    node.free();

    assert_eq!(
        delta, 0,
        "runtime class instantiated in editor ran user init {delta} time(s); placeholder should have been substituted",
    );
}

/// `bind()` on a placeholder panics with a message naming both the placeholder context and the class.
#[itest(editor)]
fn editor_runtime_bind_panics() {
    let node = RuntimeProbe::new_alloc();

    let result = suppress_panic_log(|| {
        panic::catch_unwind(panic::AssertUnwindSafe(|| {
            let _guard = node.bind();
        }))
    });

    let payload = result.expect_err("bind() on placeholder must panic");
    let msg = payload
        .downcast_ref::<String>()
        .map(String::as_str)
        .or_else(|| payload.downcast_ref::<&'static str>().copied())
        .unwrap_or("<non-string panic payload>");

    assert!(
        msg.contains("placeholder"),
        "panic message missing 'placeholder': {msg}"
    );
    assert!(
        msg.contains("RuntimeProbe"),
        "panic message missing class name: {msg}"
    );

    node.free();
}

/// Lazy default-value probe: the per-class cache is populated by exactly one real instantiation. Once warm, no further `init()` calls
/// happen for `new_alloc()` or `get()`. The editor warms the cache at startup (see module doc).
#[itest(editor)]
fn editor_runtime_lazy_default_probe() {
    // Use `>= 1` (not `== 1`): the cache is guaranteed warm by the time any test runs, but the exact probe count at startup is a Godot
    // implementation detail that may change across versions. We only care that no further inits are triggered below.
    let baseline = RUNTIME_INIT_COUNT.load(Ordering::SeqCst);
    assert!(
        baseline >= 1,
        "editor startup must populate the ClassDB default-value cache by instantiating the class at least once; got {baseline}",
    );

    let node = RuntimeProbe::new_alloc();
    assert_eq!(
        RUNTIME_INIT_COUNT.load(Ordering::SeqCst) - baseline,
        0,
        "new_alloc() must not call init() for a runtime class in the editor",
    );

    let value = node.get("export_field");
    assert_eq!(
        RUNTIME_INIT_COUNT.load(Ordering::SeqCst) - baseline,
        0,
        "get() must not call init() -- the default-value cache is already populated",
    );
    assert_eq!(
        value,
        "hello".to_variant(),
        "placeholder get() must return cached class default for #[export] properties"
    );

    node.free();
}

/// Virtual callbacks defined on a runtime class are bypassed for placeholders: Godot replaces every extension callback with a stub
/// (e.g. `placeholder_instance_property_can_revert` always returns `false`). The tool counterpart is shown alongside as a contrast.
#[itest(editor)]
fn editor_runtime_virtual_callbacks_bypassed() {
    let placeholder = RuntimeProbe::new_alloc();
    let real = ToolProbe::new_alloc();

    let field = StringName::from("var_field");

    assert!(
        !placeholder.property_can_revert(&field),
        "placeholder must always return false for property_can_revert, ignoring on_property_get_revert",
    );
    assert_eq!(
        placeholder.property_get_revert(&field),
        Variant::nil(),
        "placeholder must always return nil for property_get_revert, ignoring on_property_get_revert",
    );

    assert!(
        real.property_can_revert(&field),
        "real instance must return true for property_can_revert when on_property_get_revert returns Some",
    );
    assert_eq!(
        real.property_get_revert(&field),
        0i64.to_variant(),
        "real instance must return the value from on_property_get_revert",
    );

    placeholder.free();
    real.free();
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Tests -- tool class (never placeholdered)

/// Tool classes must *not* be placeholdered, even when constructed from Rust in editor mode. Their user code is supposed to run in the
/// editor; `bind()` succeeds and returns the real Rust struct.
#[itest(editor)]
fn editor_tool_init_runs() {
    let before = TOOL_INIT_COUNT.load(Ordering::SeqCst);
    let node = ToolProbe::new_alloc();
    let delta = TOOL_INIT_COUNT.load(Ordering::SeqCst) - before;

    assert_eq!(
        delta, 1,
        "tool class must run user init() exactly once on new_alloc()"
    );
    assert_eq!(
        node.bind().var_field,
        43,
        "bind() on tool class must return the real Rust struct"
    );

    node.free();
}
