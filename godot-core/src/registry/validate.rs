/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! Pre-registration validation of class symbols against Godot's `ClassDB`.
//!
//! # Why this exists
//!
//! All `classdb_register_extension_class*` functions return `void`. On failure, Godot prints an error via `ERR_FAIL_MSG` and returns, so the
//! extension has no way to observe that a registration was rejected. The symbol is then silently missing at runtime, and CI cannot detect it.
//! See [issue #1024](https://github.com/godot-rust/gdext/issues/1024).
//!
//! Since Godot exposes no status code and no error-handler hook, the only remaining option is to query `ClassDB` *before* each registration and
//! report the problem ourselves. That is what this module does.
//!
//! # No Rust-side bookkeeping
//!
//! Validation deliberately queries `ClassDB` on every call instead of caching results per class. This is not an oversight: each registration is
//! immediately visible in `ClassDB`, so a duplicate *within* the same class is caught by the very same query that catches a duplicate against an
//! already-existing symbol. Caching would require mirroring registration state in Rust, which is exactly the duplication this approach avoids.
//!
//! # When it runs
//!
//! Only under `safeguards_strict` (the default in debug builds) — see [`crate::init`] docs on safeguard levels. In other builds every function
//! here compiles to an empty body.
//!
//! # Limitations
//!
//! These are inherent to what `ClassDB` exposes; each is documented at the relevant check below.
//!
//! - **Coverage depends on the `ClassDB` singleton being live.** It is only registered at `Core` init level from Godot 4.7 on (see
//!   [PR #1474](https://github.com/godot-rust/gdext/pull/1474)), while most user classes -- anything deriving `Object`, `RefCounted` or
//!   `Resource` -- register at `Core`. Availability is therefore probed at runtime rather than assumed (see [`is_available()`]): on Godot 4.7+
//!   everything is validated, below that only classes registering at `Scene` level or later (in practice, `Node`-derived ones).
//! - **Ownership is invisible.** Godot rejects any per-symbol registration whose class was not registered by *this same* extension library.
//!   `ClassDB` only reports that a class exists, not which library owns it, so that guard cannot be mirrored.
//! - **Type freeze is invisible.** Godot rejects registrations once a type's `init_state` is no longer `MUTABLE` (i.e. late registration after
//!   `ClassDB` froze the type). There is no API to query that state.
//! - **Stricter than release Godot.** `ClassDB::add_property` only performs its duplicate/setter/getter checks under `DEBUG_ENABLED`. Against a
//!   release Godot build these checks are ours alone — a reported error is still a real bug, but Godot itself would have stayed silent.
//! - Reports are emitted via [`godot_error!`] and do not abort registration. Godot's own error follows, if it disagrees. Aborting would change
//!   observable behavior between safeguard levels, and panicking here would poison the registration locks (see `register_class_raw`).

#[cfg(safeguards_strict)]
use strict::*;

use crate::builtin::StringName;
use crate::meta::ClassId;

#[cfg(safeguards_strict)]
mod strict {
    pub use crate::classes::{ClassDb, Engine};
    pub use crate::godot_error;
    pub use crate::obj::Singleton;

    /// Whether the `ClassDB` singleton can be queried at the current init level.
    ///
    /// Probed rather than derived from the Godot version: `ClassDB`'s *singleton* only exists at `Core` level from Godot 4.7 on, but at `Scene`
    /// level and later it is available on all supported versions. A probe covers both cases without hardcoding a version, and degrades to
    /// "no validation" exactly where there is nothing to query.
    ///
    /// `Engine` itself is a Core-level singleton, so this is safe to call at any point during registration.
    pub fn is_available() -> bool {
        Engine::singleton().has_singleton("ClassDB")
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Class

/// Checks the preconditions of `classdb_register_extension_class*`, before the class itself is registered.
#[cfg(safeguards_strict)]
pub(crate) fn validate_class(class_id: ClassId, parent_class_id: ClassId) {
    if !is_available() {
        return;
    }

    let class_db = ClassDb::singleton();
    let class_name = class_id.to_string_name();

    // Godot: ERR_FAIL_COND(ClassDB::class_exists(class_name)). Registering over an existing class is rejected, whether that class is an engine
    // class, one from another extension, or a duplicate of our own. gdext's own duplicate-class check (ClassRegistrationInfo::validate_unique)
    // only sees Rust-side shards, so name clashes with the engine slip through it.
    //
    // Not a hot-reload false positive: classes are unregistered at level deinit (see unregister_classes), so ClassDB is clean by re-registration.
    if class_db.class_exists(&class_name) {
        godot_error!(
            "Class `{class_id}` cannot be registered: a class of that name already exists in ClassDB.\n  \
             Rename it with #[class(rename = ...)]."
        );
    }

    // Godot: ERR_FAIL_COND(!ClassDB::class_exists(parent_class_name)).
    //
    // Rust classes can only derive engine classes (`#[class(base = ...)]`), so this cannot be an ordering problem between user classes -- despite
    // `auto_register_classes` iterating a HashMap in arbitrary order. It fires when the base is not yet available at this init level.
    let parent_class_name = parent_class_id.to_string_name();
    if !class_db.class_exists(&parent_class_name) {
        godot_error!(
            "Class `{class_id}` cannot be registered: its base class `{parent_class_id}` does not exist in ClassDB.\n  \
             The base class may belong to a later initialization level than `{class_id}`."
        );
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Method

/// Checks the preconditions of `classdb_register_extension_class_method` and `..._virtual_method`.
#[cfg(safeguards_strict)]
pub(crate) fn validate_method(class_id: ClassId, method_name: &StringName) {
    if !is_available() {
        return;
    }

    // Godot: ClassDB::_bind_method_custom() fails on `type->method_map.has(name)` -- that map is per class, so shadowing a base-class method is
    // explicitly allowed and must NOT be reported. Hence `no_inheritance = true`.
    //
    // Note: two #[func]s resolving to the same Godot name (e.g. via #[func(rename = ...)]) are collapsed before reaching this point -- only one
    // registration is emitted. This check therefore mainly guards symbols that gdext cannot see in one place, such as collisions with methods
    // already present on the class from another source.
    let has_method = ClassDb::singleton()
        .class_has_method_ex(&class_id.to_string_name(), method_name)
        .no_inheritance(true)
        .done();

    if has_method {
        godot_error!(
            "Method `{class_id}::{method_name}` is already registered; overloading is not supported by Godot.\n  \
             Common causes: two #[func]s mapping to the same Godot name via #[func(rename = ...)]; a #[func] colliding with the \
             `get_*`/`set_*` accessor generated by a #[var] or #[export] field; or the same name in a primary and secondary #[godot_api] block."
        );
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Constant

/// Checks the preconditions of `classdb_register_extension_class_integer_constant`.
#[cfg(safeguards_strict)]
pub(crate) fn validate_constant(class_id: ClassId, constant_name: &StringName) {
    if !is_available() {
        return;
    }

    // Godot: GDType::bind_integer_constant() fails on `self_constant_map.has(name)` -- own class only, so a constant shadowing a base-class one is
    // allowed. `class_has_integer_constant()` has no `no_inheritance` parameter and would thus produce false positives; query the list instead.
    //
    // Note: the check is per constant name, not per enum -- Godot keeps all integer constants in one map, regardless of their enum.
    let constants = ClassDb::singleton()
        .class_get_integer_constant_list_ex(&class_id.to_string_name())
        .no_inheritance(true)
        .done();

    let constant_str = constant_name.to_string();
    if constants
        .as_slice()
        .iter()
        .any(|c| *c == constant_str.as_str())
    {
        godot_error!(
            "Constant `{class_id}::{constant_name}` is already registered.\n  \
             Note that Godot stores all integer constants of a class in one namespace, even those belonging to different enums."
        );
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Property

/// Checks the preconditions of `classdb_register_extension_class_property`.
///
/// `getter_name`/`setter_name` may be empty, in which case they are not checked.
#[cfg(safeguards_strict)]
pub(crate) fn validate_property(
    class_id: ClassId,
    property_name: &StringName,
    getter_name: &StringName,
    setter_name: &StringName,
) {
    if !is_available() {
        return;
    }

    let class_db = ClassDb::singleton();
    let class_name = class_id.to_string_name();

    // Godot: ClassDB::add_property() fails on `type->property_setget.has(name)` -- own class only. There is no `class_has_property()`, so the
    // property list is queried. This is O(properties) per property; acceptable, as it only runs under safeguards_strict.
    //
    // The list also contains category/group markers, but those never carry a registered property's name.
    let properties = class_db
        .class_get_property_list_ex(&class_name)
        .no_inheritance(true)
        .done();

    let property_str = property_name.to_string();
    let is_duplicate = properties.iter_shared().any(|dict| {
        dict.get("name")
            .and_then(|name| name.try_to::<String>().ok())
            .is_some_and(|name| name == property_str)
    });

    if is_duplicate {
        godot_error!("Property `{class_id}::{property_name}` is already registered.");
    }

    // Godot: ClassDB::add_property() resolves setter/getter via ClassDB::get_method(), which walks the inheritance chain -- so the inheriting
    // `class_has_method()` is the correct query here, unlike for the duplicate check above.
    //
    // Godot only performs this check under DEBUG_ENABLED; against a release Godot build, we are the only ones reporting it.
    //
    // Registration order matters and is guaranteed by register_class_raw(): methods and constants are registered before properties.
    for (accessor_name, role) in [(getter_name, "getter"), (setter_name, "setter")] {
        if accessor_name.is_empty() {
            continue;
        }

        if !class_db.class_has_method(&class_name, accessor_name) {
            godot_error!(
                "Property `{class_id}::{property_name}` declares {role} `{accessor_name}`, which is not a registered method.\n  \
                 Make sure the accessor is declared as #[func] and is registered before the property."
            );
        }
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Signal

/// Checks the preconditions of `classdb_register_extension_class_signal`.
///
/// Public because signals are registered by proc-macro generated code, not by `godot-core`; re-exported through `godot::private`.
#[doc(hidden)]
#[cfg(safeguards_strict)]
pub fn validate_signal(class_id: ClassId, signal_name: &StringName) {
    if !is_available() {
        return;
    }

    // Godot: GDType::add_signal() fails on `signal_map.has(name)`, and that map INCLUDES inherited signals -- unlike methods and constants. The
    // inheriting `class_has_signal()` is therefore correct, and a signal shadowing a base-class signal is a genuine error.
    if ClassDb::singleton().class_has_signal(&class_id.to_string_name(), signal_name) {
        godot_error!(
            "Signal `{class_id}::{signal_name}` is already registered.\n  \
             Unlike methods, signals share a namespace with the base class, so a signal cannot shadow one of an inherited class."
        );
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// No-op implementations outside safeguards_strict

#[cfg(not(safeguards_strict))]
pub(crate) fn validate_class(_class_id: ClassId, _parent_class_id: ClassId) {}

#[cfg(not(safeguards_strict))]
pub(crate) fn validate_method(_class_id: ClassId, _method_name: &StringName) {}

#[cfg(not(safeguards_strict))]
pub(crate) fn validate_constant(_class_id: ClassId, _constant_name: &StringName) {}

#[cfg(not(safeguards_strict))]
pub(crate) fn validate_property(
    _class_id: ClassId,
    _property_name: &StringName,
    _getter_name: &StringName,
    _setter_name: &StringName,
) {
}

#[doc(hidden)]
#[cfg(not(safeguards_strict))]
pub fn validate_signal(_class_id: ClassId, _signal_name: &StringName) {}
