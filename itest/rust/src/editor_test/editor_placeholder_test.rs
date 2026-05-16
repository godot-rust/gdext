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
use std::sync::Mutex;

use godot::builtin::{GString, StringName, Variant};
use godot::classes::notify::NodeNotification;
use godot::classes::{INode, Node};
use godot::meta::ToGodot;
use godot::obj::{Base, Gd, NewAlloc};
use godot::register::{GodotClass, godot_api};

use crate::framework::{itest, suppress_godot_print, suppress_panic_log};

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Tally (global counter for individual operations)

// Mutex good enough, no real contention.
const N: usize = Tally::_Count as usize;
static COUNTS: Mutex<[usize; N]> = Mutex::new([0; N]);

#[derive(Copy, Clone)]
#[repr(usize)]
enum Tally {
    RuntimeInit,
    RuntimeReady,
    RuntimeFunc,
    ToolInit,
    PropCustomGet,
    PropCustomSet,
    PropOnGet,
    PropOnSet,
    _Count,
}

impl Tally {
    fn inc(self) {
        COUNTS.lock().unwrap()[self as usize] += 1;
    }

    fn get(self) -> usize {
        COUNTS.lock().unwrap()[self as usize]
    }

    /// Runs `op`, returns the increment observed on `self`.
    fn delta(self, op: impl FnOnce()) -> usize {
        let before = self.get();
        op();
        self.get() - before
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Probe classes

/// Runtime (non-tool) class. Combines `#[var]` + `#[export]` (default-value cache outcomes) with custom `#[var(get=..., set=...)]`
/// accessors and `IObject::on_get` / `on_set` overrides -- placeholders bypass all of these, so they share one probe.
#[derive(GodotClass)]
#[class(base = Node)]
struct RuntimeProbe {
    #[var]
    var_field: i64,

    #[export]
    export_field: GString,

    #[var(get = get_custom, set = set_custom)]
    custom_field: i64,
}

#[godot_api]
impl INode for RuntimeProbe {
    fn init(_base: Base<Node>) -> Self {
        Tally::RuntimeInit.inc();
        Self {
            var_field: 43,
            export_field: GString::from("hello"),
            custom_field: 0,
        }
    }

    fn ready(&mut self) {
        Tally::RuntimeReady.inc();
    }

    fn on_property_get_revert(&self, property: StringName) -> Option<Variant> {
        (property == "var_field").then(|| 0.to_variant())
    }

    fn on_get(&self, _property: StringName) -> Option<Variant> {
        Tally::PropOnGet.inc();
        None
    }

    fn on_set(&mut self, _property: StringName, _value: Variant) -> bool {
        Tally::PropOnSet.inc();
        false
    }
}

#[godot_api]
impl RuntimeProbe {
    #[func]
    fn touch(&mut self) {
        Tally::RuntimeFunc.inc();
    }

    #[func]
    fn get_custom(&self) -> i64 {
        Tally::PropCustomGet.inc();
        self.custom_field
    }

    #[func]
    fn set_custom(&mut self, value: i64) {
        Tally::PropCustomSet.inc();
        self.custom_field = value;
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
        Tally::ToolInit.inc();
        Self {
            var_field: 43,
            export_field: GString::from("hello"),
        }
    }

    fn on_property_get_revert(&self, property: StringName) -> Option<Variant> {
        (property == "var_field").then(|| 0.to_variant())
    }
}

#[godot_api]
impl ToolProbe {
    #[func]
    fn touch(&mut self) {
        // Shared counter with `RuntimeProbe::touch` is fine: each test only reads the delta around its own call.
        Tally::RuntimeFunc.inc();
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Test helpers

/// Runs `op`, expects a panic, asserts the panic message contains every fragment.
///
/// Panic payload is assumed to be `String` -- godot-rust's runtime panics use `format!`, never plain string literals.
fn expect_panic_with_fragments(label: &str, fragments: &[&str], op: impl FnOnce()) {
    let result = suppress_panic_log(|| panic::catch_unwind(panic::AssertUnwindSafe(op)));
    let payload = match result {
        Ok(()) => panic!("{label}: must panic"),
        Err(p) => p,
    };
    let msg = payload
        .downcast_ref::<String>()
        .expect("panic payload should be String");

    for fragment in fragments {
        assert!(
            msg.contains(fragment),
            "{label}: missing {fragment:?} in panic: {msg}"
        );
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Tests -- runtime class (placeholdered in editor)

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
    assert!(
        has_name("custom_field"),
        "custom_field missing from property list"
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

/// Lazy default-value probe: the per-class cache is populated by exactly one real instantiation. Once warm, no further `init()` calls
/// happen for `new_alloc()` or `get()`. The editor warms the cache at startup (see module doc).
///
/// Doubles as regression test for <https://github.com/godot-rust/gdext/issues/1404>: `new_alloc()` on a runtime class in editor
/// must not run `init()`.
#[itest(editor)]
fn editor_runtime_lazy_default_probe() {
    // `>= 1` (not `== 1`): the cache is warm by the time any test runs, but the exact startup probe count is a Godot impl detail.
    let baseline = Tally::RuntimeInit.get();
    assert!(
        baseline >= 1,
        "editor startup must populate the ClassDB default-value cache by instantiating the class at least once; got {baseline}",
    );

    let node = RuntimeProbe::new_alloc();
    assert_eq!(
        Tally::RuntimeInit.get() - baseline,
        0,
        "new_alloc() must not call init() for a runtime class in the editor",
    );

    let export_value = node.get("export_field");
    let var_value = node.get("var_field");
    assert_eq!(
        Tally::RuntimeInit.get() - baseline,
        0,
        "get() must not call init() -- the default-value cache is already populated",
    );
    assert_eq!(
        export_value,
        "hello".to_variant(),
        "placeholder get() must return cached class default for #[export] properties"
    );
    assert_eq!(
        var_value,
        Variant::nil(),
        "placeholder get() must return nil for #[var]-only properties (not in default-value cache)"
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
    let mut node = None;
    let init_delta = Tally::ToolInit.delta(|| node = Some(ToolProbe::new_alloc()));
    let node = node.unwrap();

    assert_eq!(
        init_delta, 1,
        "tool class must run user init() exactly once on new_alloc()"
    );
    assert_eq!(
        node.bind().var_field,
        43,
        "bind() on tool class must return the real Rust struct"
    );

    node.free();
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Additional placeholder API / behavior tests

/// `Gd::is_editor_placeholder()`: true for runtime classes in editor, false for tool classes.
#[itest(editor)]
fn editor_placeholder_flag() {
    let runtime = RuntimeProbe::new_alloc();
    let tool = ToolProbe::new_alloc();

    assert!(
        runtime.is_editor_placeholder(),
        "runtime class must report as placeholder in editor"
    );
    assert!(
        !tool.is_editor_placeholder(),
        "tool class must never report as placeholder"
    );

    runtime.free();
    tool.free();
}

/// Both `bind()` and `bind_mut()` panic on a placeholder; panic message names the placeholder context and the class.
#[itest(editor)]
fn editor_runtime_bind_panics() {
    let mut node = RuntimeProbe::new_alloc();
    let fragments = &["placeholder", "RuntimeProbe"];

    expect_panic_with_fragments("bind()", fragments, || drop(node.bind()));
    expect_panic_with_fragments("bind_mut()", fragments, || drop(node.bind_mut()));

    node.free();
}

/// `#[func]` is callable through `Object::call()` on a placeholder (method is in `ClassDB`), but the wrapper's internal `bind()` panics --
/// so the user body never runs. Tool counterpart runs the body normally.
#[itest(editor)]
fn editor_runtime_func_body_skipped() {
    let mut placeholder = RuntimeProbe::new_alloc();
    let mut tool = ToolProbe::new_alloc();

    let placeholder_delta = Tally::RuntimeFunc.delta(|| {
        // FFI boundary catches the inner panic and turns the call into a nil-returning error log.
        suppress_godot_print(|| {
            let _ = placeholder.call("touch", &[]);
        });
    });
    assert_eq!(
        placeholder_delta, 0,
        "#[func] body must not execute on placeholder"
    );

    let tool_delta = Tally::RuntimeFunc.delta(|| {
        let _ = tool.call("touch", &[]);
    });
    assert_eq!(tool_delta, 1, "tool counterpart must execute #[func] body");

    placeholder.free();
    tool.free();
}

/// On a placeholder, `set`/`get` go through Godot's internal property map and bypass both `#[var(get=..., set=...)]` accessors and the
/// `IObject::on_get` / `on_set` overrides.
#[itest(editor)]
fn editor_runtime_property_hooks_skipped() {
    let mut node = RuntimeProbe::new_alloc();

    let hooks = [
        (Tally::PropCustomSet, "custom #[var(set=...)]"),
        (Tally::PropOnSet, "IObject::on_set"),
        (Tally::PropCustomGet, "custom #[var(get=...)]"),
        (Tally::PropOnGet, "IObject::on_get"),
    ];
    let before = hooks.map(|(c, _)| c.get());

    node.set("custom_field", &7.to_variant());
    let roundtrip_value = node.get("custom_field");

    for (i, (c, label)) in hooks.iter().enumerate() {
        assert_eq!(c.get(), before[i], "{label} must not run on placeholder");
    }

    // Roundtrip resolves via the placeholder's internal map, not our accessors.
    assert_eq!(
        roundtrip_value,
        7.to_variant(),
        "placeholder set/get roundtrip must echo the raw Variant"
    );

    node.free();
}

/// Virtual `ready()` is bypassed on a placeholder: `NOTIFICATION_READY` does not invoke the Rust override.
#[itest(editor)]
fn editor_runtime_ready_skipped() {
    let mut node = RuntimeProbe::new_alloc();

    let ready_delta = Tally::RuntimeReady.delta(|| node.notify(NodeNotification::READY));
    assert_eq!(
        ready_delta, 0,
        "ready() override must not run for placeholder (notification routed to Godot stub)"
    );

    node.free();
}

/// Placeholder property storage is per-instance: setting on one placeholder does not leak to another.
#[itest(editor)]
fn editor_runtime_storage_isolation() {
    let mut a = RuntimeProbe::new_alloc();
    let b = RuntimeProbe::new_alloc();

    a.set("var_field", &111.to_variant());

    assert_eq!(
        a.get("var_field"),
        111.to_variant(),
        "A must read back its own value"
    );
    assert_eq!(
        b.get("var_field"),
        Variant::nil(),
        "B must be independent (no shared storage; #[var] has no cached default)"
    );

    a.free();
    b.free();
}

/// Upcasts and downcasts work on placeholders; the Godot-side class hierarchy is intact.
#[itest(editor)]
fn editor_runtime_upcast_downcast() {
    let node = RuntimeProbe::new_alloc();
    assert!(node.is_editor_placeholder());

    // `is_editor_placeholder()` is only defined on user-declared `T`, so verify the upcasted object via its dynamic class.
    let upcast: Gd<Node> = node.clone().upcast();
    assert_eq!(
        upcast.get_class(),
        GString::from("RuntimeProbe"),
        "upcast preserves dynamic class"
    );

    let downcast = upcast
        .try_cast::<RuntimeProbe>()
        .expect("downcast back to RuntimeProbe");
    assert!(
        downcast.is_editor_placeholder(),
        "downcast back to user type preserves placeholder status"
    );

    node.free();
}
