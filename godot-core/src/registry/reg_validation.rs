/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! Pre-registration validation of class symbols against Godot's `ClassDB`.
//!
//! `classdb_register_extension_class*` returns `void` and only prints to stderr on failure, so this module queries `ClassDB` *before* each
//! registration; see [issue #1024](https://github.com/godot-rust/gdext/issues/1024). Active under `safeguards_strict` and Godot 4.5+, no-op
//! otherwise. Problems are reported via `defer_startup_fatal!`/`defer_startup_warn!` rather than panics, which mid-registration would poison the
//! registration locks (see `register_class_raw`). Godot's own checks run in all builds, except a few gated behind `DEBUG_ENABLED` (e.g.
//! `ClassDB::add_property` verifying that accessors exist).
//!
//! # Limitations
//! Godot rejects a symbol only if the *own* class declares it (except signals, which share the base's namespace). Reusing a base-class name thus
//! only warns: both symbols stay registered, and the static type at the call site decides which applies.
//!
//! Cases Godot rejects but this module cannot detect:
//! - Symbols registered at `InitLevel::Core` up to Godot 4.6, where the `ClassDB` singleton appears only after `Core`-level extension init.
//!   Needs an explicit `ExtensionLibrary::min_level()` override, since the default is `Scene`. Godot 4.7+ covers all levels.
//! - A class registered by *another* extension: `class_get_api_type()` reveals only that a class stems from *some* extension, not which one.
//! - A type whose `init_state` has left `MUTABLE`: that state cannot be queried.
//! - A property registered with neither getter nor setter (`#[var]`/`#[export]` always emit at least one).
// TODO(v0.6): promote shadowing warnings to hard errors, possibly with `#[var(override)] escape hatch if true use-case found.

#[cfg(not(all(safeguards_strict, since_api = "4.5")))]
pub use noop::*;
#[cfg(all(safeguards_strict, since_api = "4.5"))]
pub use strict::*;

#[cfg(all(safeguards_strict, since_api = "4.5"))]
mod strict {
    use crate::builtin::{GString, StringName};
    use crate::classes::ClassDb;
    use crate::meta::ClassId;
    use crate::obj::Singleton;
    use crate::sys::{defer_startup_fatal, defer_startup_warn};

    // `ClassDB` is queried afresh in each function rather than mirrored in Rust: registrations are immediately visible there, so one query
    // catches both duplicates within the class and names already taken by Godot. Most queries walk the inheritance chain, so a miss (common
    // case) exits early without allocations; only a hit runs the own-class query that separates duplicate (error) from shadowing (warning).

    /// Early-returns from a `validate_*` function when the `ClassDB` singleton cannot be queried at the current init level.
    ///
    /// Godot 4.6 and earlier add the singleton only after `Core`-level extension init, so its presence must be checked at runtime.
    #[cfg(before_api = "4.7")]
    macro_rules! return_if_unavailable {
        () => {
            if !crate::init::is_singleton_available::<ClassDb>() {
                return;
            }
        };
    }

    /// Godot 4.7+ adds `ClassDB` before `Core`-level extension init, so it is available at every level.
    #[cfg(since_api = "4.7")]
    macro_rules! return_if_unavailable {
        () => {};
    }

    /// Checks the preconditions of `classdb_register_extension_class*`, before the class itself is registered.
    pub(crate) fn validate_class(class_id: ClassId, parent_class_id: ClassId) {
        return_if_unavailable!();

        let class_db = ClassDb::singleton();
        let class_name = class_id.to_string_name();

        // Godot rejects a class whose name is taken. ClassRegistrationInfo::validate_unique() only sees Rust-side shards, so a name used by an
        // engine or other-extension class slips through it. Not a hot-reload false positive, see unregister_classes().
        if class_db.class_exists(&class_name) {
            defer_startup_fatal!(
                "Class `{class_id}` cannot be registered: a class of that name already exists in ClassDB.\n\
                 Rename it with #[class(rename = ...)]."
            );
        }

        // Godot rejects a class whose base does not exist. Rust classes can only derive engine classes, so this is never an ordering problem
        // between user classes; it fires when the base is not yet available at this init level.
        let parent_class_name = parent_class_id.to_string_name();
        if !class_db.class_exists(&parent_class_name) {
            defer_startup_fatal!(
                "Class `{class_id}` cannot be registered: its base class `{parent_class_id}` does not exist in ClassDB.\n\
                 The base class may belong to a later initialization level than `{class_id}`."
            );
        }
    }

    /// Checks the preconditions of `classdb_register_extension_class_method`.
    ///
    /// Not applicable to `classdb_register_extension_class_virtual_method`: Godot stores virtual methods in a separate map, which `ClassDB` does
    /// not expose. Callers must skip this check for virtual methods, or it would query the wrong map.
    pub(crate) fn validate_method(class_id: ClassId, method_name: &StringName) {
        return_if_unavailable!();

        let class_db = ClassDb::singleton();
        let class_name = class_id.to_string_name();

        if !class_db.class_has_method(&class_name, method_name) {
            return;
        }

        // Godot rejects a method name already declared in the own class only, hence `no_inheritance = true`.
        let is_own_duplicate = class_db
            .class_has_method_ex(&class_name, method_name)
            .no_inheritance(true)
            .done();

        if is_own_duplicate {
            defer_startup_fatal!(
                "Method `{class_id}::{method_name}` is already registered; overloading is not supported by Godot. Common causes:\n\
                 * two #[func]s mapping to the same Godot name via #[func(rename = ...)]\n\
                 * a #[func] colliding with the `get_*`/`set_*` accessor generated by a #[var] or #[export] field\n\
                 * the same name in a primary and secondary #[godot_api] block."
            );
        } else {
            // The base method stays reachable through a base-typed reference, so the two coexist and dispatch depends on the caller's static type.
            defer_startup_warn!(
                id: "shadowed_method",
                "Method `{class_id}::{method_name}` shadows a method of a base class.\n\
                 Both stay registered -- either your method or the base one is called, depending on the static type at the call site.\n\
                 Rename the Rust fn or use #[func(rename = ...)]. This will become a hard error in godot-rust v0.6."
            );
        }
    }

    /// Checks the preconditions of `classdb_register_extension_class_integer_constant`.
    pub(crate) fn validate_constant(class_id: ClassId, constant_name: &StringName) {
        return_if_unavailable!();

        // Godot rejects a constant name already declared in the own class. `class_has_integer_constant()` lacks a `no_inheritance` parameter,
        // so the own-class list is fetched on a hit.
        let class_db = ClassDb::singleton();
        let class_name = class_id.to_string_name();

        if !class_db.class_has_integer_constant(&class_name, constant_name) {
            return;
        }

        let constants = class_db
            .class_get_integer_constant_list_ex(&class_name)
            .no_inheritance(true)
            .done();

        if constants.as_slice().contains(&GString::from(constant_name)) {
            defer_startup_fatal!(
                "Constant `{class_id}::{constant_name}` is already registered.\n\
                 Note that Godot stores all integer constants of a class in one namespace, even those belonging to different enums."
            );
        } else {
            defer_startup_warn!(
                id: "shadowed_constant",
                "Constant `{class_id}::{constant_name}` shadows a constant of a base class.\n\
                 Both stay registered -- yours or the base one is read, depending on the static type at the access site.\n\
                 Rename it. This will become a hard error in godot-rust v0.6."
            );
        }
    }

    /// Checks the preconditions of `classdb_register_extension_class_property`.
    ///
    /// `getter_name`/`setter_name` may be empty, in which case they are not checked.
    pub(crate) fn validate_property(
        class_id: ClassId,
        property_name: &StringName,
        getter_name: &StringName,
        setter_name: &StringName,
    ) {
        return_if_unavailable!();

        let mut class_db = ClassDb::singleton();
        let class_name = class_id.to_string_name();

        validate_property_uniqueness(&mut class_db, class_id, &class_name, property_name);

        // Godot resolves setter/getter through the inheritance chain, so the inheriting `class_has_method()` is correct here, unlike above.
        // register_class_raw() guarantees that methods are registered before properties.
        for (accessor_name, role) in [(getter_name, "getter"), (setter_name, "setter")] {
            if accessor_name.is_empty() {
                continue;
            }

            if !class_db.class_has_method(&class_name, accessor_name) {
                defer_startup_fatal!(
                    "Property `{class_id}::{property_name}` declares {role} `{accessor_name}`, which is not a registered method.\n\
                     Declare the accessor as #[func]."
                );
            }
        }
    }

    /// Detects properties that are already registered in the same class, or that shadow one of a base class.
    fn validate_property_uniqueness(
        class_db: &mut ClassDb,
        class_id: ClassId,
        class_name: &StringName,
        property_name: &StringName,
    ) {
        // Godot rejects a property name already declared in the own class. Lacking a `class_has_property()`, the getter/setter queries stand in
        // for it; the own-class list is fetched on a hit.
        let exists_in_chain = !class_db
            .class_get_property_getter(class_name, property_name)
            .is_empty()
            || !class_db
                .class_get_property_setter(class_name, property_name)
                .is_empty();

        if !exists_in_chain {
            return;
        }

        let properties = class_db
            .class_get_property_list_ex(class_name)
            .no_inheritance(true)
            .done();

        let property_name_str = GString::from(property_name);
        let is_own_duplicate = properties.iter_shared().any(|dict| {
            dict.get("name")
                .and_then(|name| name.try_to::<GString>().ok())
                .is_some_and(|name| name == property_name_str)
        });

        if is_own_duplicate {
            defer_startup_fatal!("Property `{class_id}::{property_name}` is already registered.");
        } else {
            // The inherited property keeps its own accessors, so the two disagree about which storage a name refers to.
            defer_startup_warn!(
                id: "shadowed_property",
                "Property `{class_id}::{property_name}` shadows a property of a base class.\n\
                 Both stay registered, with separate accessors -- reads/writes access your field or the base one,\n\
                 depending on the static type at the access site.\n\
                 Rename the Rust field or use #[var(rename = ...)]. This will become a hard error in godot-rust v0.6."
            );
        }
    }

    /// Checks the preconditions of `classdb_register_extension_class_signal`.
    ///
    /// Public because signals are registered by proc-macro generated code, not by `godot-core`; re-exported through `godot::private`.
    #[doc(hidden)]
    pub fn validate_signal(class_id: ClassId, signal_name: &StringName) {
        return_if_unavailable!();

        // Godot's signal namespace includes inherited signals -- unlike methods and constants -- so the inheriting `class_has_signal()` is correct.
        if ClassDb::singleton().class_has_signal(&class_id.to_string_name(), signal_name) {
            defer_startup_fatal!(
                "Signal `{class_id}::{signal_name}` is already registered.\n\
                 Unlike methods, signals share a namespace with the base class, so a signal cannot shadow one of a base class."
            );
        }
    }
}

#[cfg(not(all(safeguards_strict, since_api = "4.5")))]
mod noop {
    use crate::builtin::StringName;
    use crate::meta::ClassId;

    pub(crate) fn validate_class(_class_id: ClassId, _parent_class_id: ClassId) {}
    pub(crate) fn validate_method(_class_id: ClassId, _method_name: &StringName) {}
    pub(crate) fn validate_constant(_class_id: ClassId, _constant_name: &StringName) {}

    pub(crate) fn validate_property(
        _class_id: ClassId,
        _property_name: &StringName,
        _getter_name: &StringName,
        _setter_name: &StringName,
    ) {
    }

    pub fn validate_signal(_class_id: ClassId, _signal_name: &StringName) {}
}
