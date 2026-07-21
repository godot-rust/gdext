/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! Pre-registration validation of class symbols against Godot's `ClassDB`.
//!
//! All `classdb_register_extension_class*` functions return `void`; on failure, Godot prints an error via `ERR_FAIL_MSG` and returns. The
//! extension cannot observe the rejection, the symbol is silently missing at runtime and CI cannot detect it. Godot exposes neither a status code
//! nor an error-handler hook, so this module queries `ClassDB` *before* each registration and reports the problem itself. See
//! [issue #1024](https://github.com/godot-rust/gdext/issues/1024).
//!
//! Runs only under `safeguards_strict` (the default in debug builds) -- see the [safeguard levels](../index.html#safeguard-levels) section; in
//! other builds every function here compiles to an empty body.
//!
//! `ClassDB` is queried afresh on every call instead of mirroring registration state in Rust. Each registration is immediately visible there, so
//! the same query catches duplicates within a class and against already-existing symbols.
//!
//! Godot rejects a symbol only if the *own* class already declares it, while most `ClassDB` queries walk the inheritance chain. Methods,
//! constants and properties therefore run the inheriting query first and exit early on a miss -- the common case, and free of allocations. Only
//! on a hit does a second, own-class-only query separate a true duplicate (error) from a shadowed base symbol (warning, see below).
//!
//! # Shadowing base-class symbols
//! Godot checks methods and properties per class, so a `#[func]` or `#[var]` may reuse a base-class name; only signals share a namespace with the
//! base and are rejected outright. Reusing a name is still warned about: both symbols stay registered, and which one applies depends on the static
//! type at the call or access site -- practically always a mistake rather than an override.
//!
//! TODO(v0.6): promote these warnings to hard errors, with an opt-in marker on the symbol (e.g. `#[func(shadow_base)]`) as escape hatch.
//!
//! # Reachability
//! - Reachable through the proc-macro API: duplicate *class*, *method*, *property*, *signal*: mostly via `rename` keys, or plain
//!   `#[func]`/`#[var]`/`#[signal]` names shadowing a base class.
//! - Not reachable: duplicate *constant*, missing *base class*, unregistered *property accessor*: `#[constant]` has no `rename` and Rust
//!   rejects same-named associated constants; only engine classes of the same or a lower init level can be derived; `#[var(get = ident)]`
//!   resolves through the generated `__godot_*_Funcs`. These checks only guard a future builder API.
//!
//! # Limitations
//! Inherent to what `ClassDB` exposes; each is documented at the relevant check below.
//!
//! - Godot 4.6 and earlier register the `ClassDB` singleton only at `Scene` level, leaving classes with a `Core`-level base (`Object`,
//!   `RefCounted`, `Resource`) unchecked there. From Godot 4.7 on, all levels are covered.
//! - Godot rejects per-symbol registrations whose class was not registered by *this same* extension library, but `ClassDB` does not report class
//!   ownership.
//! - Godot rejects registrations once a type's `init_state` is no longer `MUTABLE`; that state cannot be queried.
//! - `ClassDB::add_property` only performs its duplicate/setter/getter checks under `DEBUG_ENABLED`, and Godot 4.6 and earlier gate the
//!   duplicate-signal check on `DEBUG_METHODS_ENABLED`. Against a release Godot, these checks may thus be the only ones reporting a real bug.
//! - Reports via [`godot_error!`][crate::godot_error] do not abort registration. Aborting would change observable behavior between safeguard
//!   levels, and panicking would poison the registration locks (see `register_class_raw`).

#[cfg(not(safeguards_strict))]
pub use noop::*;
#[cfg(safeguards_strict)]
pub use strict::*;

#[cfg(safeguards_strict)]
mod strict {
    use crate::builtin::{GString, StringName};
    use crate::classes::ClassDb;
    use crate::meta::ClassId;
    use crate::obj::Singleton;
    use crate::{godot_error, godot_warn};

    /// Early-returns from a `validate_*` function when the `ClassDB` singleton cannot be queried at the current init level.
    ///
    /// Godot 4.6 and earlier register the singleton only at `Scene` level, so its presence must be checked at runtime.
    ///
    /// Runs per registered symbol, costing 1 FFI call + 1 `StringName` alloc. Could be memoized inside `is_singleton_available()`, but
    /// hardly worth it.
    macro_rules! return_if_unavailable {
        () => {
            if !crate::init::is_singleton_available::<ClassDb>() {
                return;
            }
        };
    }

    /// Checks the preconditions of `classdb_register_extension_class*`, before the class itself is registered.
    pub(crate) fn validate_class(class_id: ClassId, parent_class_id: ClassId) {
        return_if_unavailable!();

        let class_db = ClassDb::singleton();
        let class_name = class_id.to_string_name();

        // Godot: ERR_FAIL_COND(ClassDB::class_exists(class_name)). ClassRegistrationInfo::validate_unique() only sees Rust-side shards, so
        // clashes with engine or other-extension classes slip through it. Not a hot-reload false positive, see unregister_classes().
        if class_db.class_exists(&class_name) {
            godot_error!(
                "Class `{class_id}` cannot be registered: a class of that name already exists in ClassDB.\n\
                 Rename it with #[class(rename = ...)]."
            );
        }

        // Godot: ERR_FAIL_COND(!ClassDB::class_exists(parent_class_name)). Rust classes can only derive engine classes, so this is never an
        // ordering problem between user classes; it fires when the base is not yet available at this init level.
        let parent_class_name = parent_class_id.to_string_name();
        if !class_db.class_exists(&parent_class_name) {
            godot_error!(
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

        // Godot: ClassDB::_bind_method_custom() fails on `type->method_map.has(name)` -- own class only, hence `no_inheritance = true`.
        let is_own_duplicate = class_db
            .class_has_method_ex(&class_name, method_name)
            .no_inheritance(true)
            .done();

        if is_own_duplicate {
            godot_error!(
                "Method `{class_id}::{method_name}` is already registered; overloading is not supported by Godot. Common causes:\n\
                 * two #[func]s mapping to the same Godot name via #[func(rename = ...)]\n\
                 * a #[func] colliding with the `get_*`/`set_*` accessor generated by a #[var] or #[export] field\n\
                 * the same name in a primary and secondary #[godot_api] block."
            );
        } else {
            // The base method stays reachable through a base-typed reference, so the two coexist and dispatch depends on the caller's static type.
            // TODO(v0.6): make this a hard error, once an opt-in override syntax exists (see module docs).
            godot_warn!(
                "Method `{class_id}::{method_name}` shadows a method of a base class.\n\
                 Both stay registered -- either your method or the base one is called, depending on the static type at the call site.\n\
                 Rename the Rust fn or use #[func(rename = ...)]. This will become a hard error in godot-rust v0.6."
            );
        }
    }

    /// Checks the preconditions of `classdb_register_extension_class_integer_constant`.
    pub(crate) fn validate_constant(class_id: ClassId, constant_name: &StringName) {
        return_if_unavailable!();

        // Godot: GDType::bind_integer_constant() fails on `self_constant_map.has(name)` -- own class only, and per constant name rather than per
        // enum. `class_has_integer_constant()` lacks a `no_inheritance` parameter, so the own-class list is fetched on a hit.
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
            godot_error!(
                "Constant `{class_id}::{constant_name}` is already registered.\n\
                 Note that Godot stores all integer constants of a class in one namespace, even those belonging to different enums."
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

        let class_db = ClassDb::singleton();
        let class_name = class_id.to_string_name();

        validate_property_uniqueness(class_id, &class_name, property_name);

        // Godot: ClassDB::add_property() resolves setter/getter via ClassDB::get_method(), which walks the inheritance chain -- so the inheriting
        // `class_has_method()` is correct here, unlike above. register_class_raw() guarantees that methods are registered before properties.
        for (accessor_name, role) in [(getter_name, "getter"), (setter_name, "setter")] {
            if accessor_name.is_empty() {
                continue;
            }

            if !class_db.class_has_method(&class_name, accessor_name) {
                godot_error!(
                    "Property `{class_id}::{property_name}` declares {role} `{accessor_name}`, which is not a registered method.\n\
                     Declare the accessor as #[func]."
                );
            }
        }
    }

    /// Detects properties that are already registered in the same class, or that shadow one of a base class.
    ///
    /// Needs `ClassDB::class_get_property_getter|setter()`, added in Godot 4.4; earlier versions skip this check.
    #[cfg(since_api = "4.4")]
    fn validate_property_uniqueness(
        class_id: ClassId,
        class_name: &StringName,
        property_name: &StringName,
    ) {
        let mut class_db = ClassDb::singleton();

        // Godot: ClassDB::add_property() fails on `type->property_setget.has(name)` -- own class only. Lacking a `class_has_property()`, the
        // getter/setter queries stand in for it; the own-class list is fetched on a hit.
        // Limitation: a property registered with neither accessor passes unnoticed, but #[var]/#[export] always emit at least one.
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
            godot_error!("Property `{class_id}::{property_name}` is already registered.");
        } else {
            // The inherited property keeps its own accessors, so the two disagree about which storage a name refers to.
            // TODO(v0.6): make this a hard error, once an opt-in override syntax exists (see module docs).
            godot_warn!(
                "Property `{class_id}::{property_name}` shadows a property of a base class.\n\
                 Both stay registered, with separate accessors -- reads/writes access your field or the base one,\n\
                 depending on the static type at the access site.\n\
                 Rename the Rust fn or use #[func(rename = ...)]. This will become a hard error in godot-rust v0.6."
            );
        }
    }

    #[cfg(before_api = "4.4")]
    fn validate_property_uniqueness(
        _class_id: ClassId,
        _class_name: &StringName,
        _property_name: &StringName,
    ) {
    }

    /// Checks the preconditions of `classdb_register_extension_class_signal`.
    ///
    /// Public because signals are registered by proc-macro generated code, not by `godot-core`; re-exported through `godot::private`.
    #[doc(hidden)]
    pub fn validate_signal(class_id: ClassId, signal_name: &StringName) {
        return_if_unavailable!();

        // Godot: GDType::add_signal() fails on `signal_map.has(name)`, and that map INCLUDES inherited signals -- unlike methods and constants.
        // The inheriting `class_has_signal()` is therefore correct, and shadowing a base-class signal is a genuine error.
        if ClassDb::singleton().class_has_signal(&class_id.to_string_name(), signal_name) {
            godot_error!(
                "Signal `{class_id}::{signal_name}` is already registered.\n\
                 Unlike methods, signals share a namespace with the base class, so a signal cannot shadow one of a base class."
            );
        }
    }
}

#[cfg(not(safeguards_strict))]
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

    #[doc(hidden)]
    pub fn validate_signal(_class_id: ClassId, _signal_name: &StringName) {}
}
