/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! Internal registration machinery used by proc-macro APIs.

use std::collections::HashMap;

use sys::GodotFfi;

use crate::builtin::{GString, StringName};
use crate::global::PropertyUsageFlags;
use crate::init::InitLevel;
use crate::meta::{ClassId, GodotConvert, GodotType, PropertyHintInfo, PropertyInfo};
use crate::obj::GodotClass;
use crate::registry::property::{Export, Var};
use crate::sys::Global;
use crate::{classes, sys};

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Deferred property override validation

/// Stores property override validation data pending ClassDB availability.
#[derive(Debug)]
struct PendingValidation {
    class_name: ClassId,
    base_class_name: ClassId,
    property_name: String,
    marked_override: bool,
}

/// Global registry of pending property override validations, organized by init level.
///
/// During class registration, validation requests are stored here instead of being executed immediately.
/// After `auto_register_classes(level)` completes, `validate_pending_overrides(level)` queries ClassDB
/// to perform the actual validation.
static PENDING_VALIDATIONS: Global<HashMap<InitLevel, Vec<PendingValidation>>> = Global::default();

// ----------------------------------------------------------------------------------------------------------------------------------------------

/// Same as [`register_var()`], but statically verifies the `Export` trait (again) and the fact that nodes can only be exported from nodes.
pub fn register_export<C: GodotClass, T: Export>(
    property_name: &str,
    getter_name: &str,
    setter_name: &str,
    hint_info: PropertyHintInfo,
    usage: PropertyUsageFlags,
    marked_override: bool,
) {
    // Note: if the user manually specifies `hint`, `hint_string` or `usage` keys, and thus is routed to `register_var()` instead,
    // they can bypass this validation.
    if !C::inherits::<classes::Node>() {
        if let Some(class) = T::as_node_class() {
            panic!(
                "#[export] for Gd<{t}>: nodes can only be exported in Node-derived classes, but the current class is {c}.",
                t = class,
                c = C::class_id()
            );
        }
    }

    register_var::<C, T>(
        property_name,
        getter_name,
        setter_name,
        hint_info,
        usage,
        marked_override,
    );
}

pub fn register_var<C: GodotClass, T: Var>(
    property_name: &str,
    getter_name: &str,
    setter_name: &str,
    hint_info: PropertyHintInfo,
    usage: PropertyUsageFlags,
    marked_override: bool,
) {
    // Validate override flag against base class properties.
    validate_property_override::<C>(property_name, marked_override);

    let info = PropertyInfo {
        variant_type: <<T as GodotConvert>::Via as GodotType>::Ffi::VARIANT_TYPE.variant_as_nil(),
        class_id: <T as GodotConvert>::Via::class_id(),
        property_name: StringName::from(property_name),
        hint_info,
        usage,
    };

    let class_name = C::class_id();

    register_var_or_export_inner(info, class_name, getter_name, setter_name);
}

fn register_var_or_export_inner(
    info: PropertyInfo,
    class_name: ClassId,
    getter_name: &str,
    setter_name: &str,
) {
    let getter_name = StringName::from(getter_name);
    let setter_name = StringName::from(setter_name);

    let property_info_sys = info.property_sys();

    unsafe {
        sys::interface_fn!(classdb_register_extension_class_property)(
            sys::get_library(),
            class_name.string_sys(),
            std::ptr::addr_of!(property_info_sys),
            setter_name.string_sys(),
            getter_name.string_sys(),
        );
    }
}

pub fn register_group<C: GodotClass>(group_name: &str, prefix: &str) {
    let group_name = GString::from(group_name);
    let prefix = GString::from(prefix);
    let class_name = C::class_id();

    unsafe {
        sys::interface_fn!(classdb_register_extension_class_property_group)(
            sys::get_library(),
            class_name.string_sys(),
            group_name.string_sys(),
            prefix.string_sys(),
        );
    }
}

pub fn register_subgroup<C: GodotClass>(subgroup_name: &str, prefix: &str) {
    let subgroup_name = GString::from(subgroup_name);
    let prefix = GString::from(prefix);
    let class_name = C::class_id();

    unsafe {
        sys::interface_fn!(classdb_register_extension_class_property_subgroup)(
            sys::get_library(),
            class_name.string_sys(),
            subgroup_name.string_sys(),
            prefix.string_sys(),
        );
    }
}

/// Stores property override validation request for deferred validation.
///
/// Validation is deferred until after `auto_register_classes(level)` completes, at which point
/// ClassDB is populated with the newly-registered classes and safe to query.
///
/// Future improvement: Generate property symbols per class to enable compile-time validation; similar to virtual method hashes in
/// `godot-codegen/src/generator/virtual_definitions.rs`.
///
/// This would create modules like:
/// ```ignore
/// pub mod Node {
///     pub const name: &str = "name";
///     pub const position: &str = "position";
///     pub use super::Object::*; // Inherit parent properties
/// }
/// ```
/// Then macros could check `<Base as GodotClass>::PropertySymbols::name` at compile time.
fn validate_property_override<C: GodotClass>(property_name: &str, marked_override: bool) {
    // Store validation request for later execution when ClassDB is available.
    let mut pending = PENDING_VALIDATIONS.lock();

    pending
        .entry(C::INIT_LEVEL)
        .or_default()
        .push(PendingValidation {
            class_name: C::class_id(),
            base_class_name: C::Base::class_id(),
            property_name: property_name.to_string(),
            marked_override,
        });
}

/// Validates all pending property overrides for the given initialization level.
///
/// This must be called AFTER `auto_register_classes(level)` completes, ensuring
/// that ClassDB has been populated with all classes for this level.
///
/// # Panics
///
/// Panics if any property override validation fails. All errors for the level are
/// collected and reported together before panicking.
pub fn validate_pending_overrides(level: InitLevel) {
    // ClassDB is not available during Core level initialization.
    // Skip validation for Core level to avoid panicking when trying to access ClassDb::singleton().
    if level == InitLevel::Core {
        return;
    }

    let mut pending = PENDING_VALIDATIONS.lock();

    // Take ownership of validations for this level (removing from map).
    let Some(validations) = pending.remove(&level) else {
        return; // No validations pending for this level.
    };

    // Collect all errors before panicking (better UX).
    let mut errors = Vec::new();

    for validation in validations {
        let base_has_property =
            check_base_has_property_by_id(validation.base_class_name, &validation.property_name);

        if validation.marked_override && !base_has_property {
            errors.push(format!(
                "Property `{}` in class `{}` has #[var(override)], but neither direct base class `{}`\n\
                nor any indirect one has a property with that name.",
                validation.property_name,
                validation.class_name,
                validation.base_class_name
            ));
        }

        if !validation.marked_override && base_has_property {
            errors.push(format!(
                "Property `{}` in class `{}` overrides property from base class `{}`, but is missing #[var(override)].\n\
                Add #[var(override)] to explicitly indicate this override is intentional.",
                validation.property_name,
                validation.class_name,
                validation.base_class_name
            ));
        }
    }

    // Report all errors together.
    if !errors.is_empty() {
        panic!(
            "Property override validation failed:\n\n{}",
            errors.join("\n\n")
        );
    }
}

/// Checks if a base class (identified by ClassId) has a property with the given name.
///
/// This function queries ClassDB, so it must only be called when ClassDB is available
/// (i.e., after class registration completes).
fn check_base_has_property_by_id(base_class_id: ClassId, property_name: &str) -> bool {
    use crate::builtin::{GString, StringName};
    use crate::classes::ClassDb;
    use crate::obj::Singleton;

    let class_name: StringName = base_class_id.to_string_name();
    let class_db = ClassDb::singleton();
    let property_list = class_db.class_get_property_list(&class_name);

    property_list.iter_shared().any(|dict| {
        dict.get("name")
            .and_then(|v| v.try_to::<GString>().ok())
            .is_some_and(|name| name == property_name)
    })
}
