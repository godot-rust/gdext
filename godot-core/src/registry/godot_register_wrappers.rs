/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! Internal registration machinery used by proc-macro APIs.

use crate::builtin::{GString, StringName};
use crate::meta::ClassId;
use crate::meta::shape::GodotShape;
use crate::obj::GodotClass;
use crate::registry::info::{PropertyHintInfo, PropertyUsageFlags};
use crate::registry::property::{Export, Var};
use crate::{classes, sys};

/// Registers a `#[export]` property with Godot's ClassDB.
///
/// Statically verifies the `Export` trait and that nodes can only be exported from nodes.
/// Defaults are resolved from `T::godot_shape()`: [`export_hint()`] for hints, [`DEFAULT`] for usage flags. Pass `Some(...)` to override either.
///
/// [`export_hint()`]: crate::meta::shape::GodotShape::export_hint
/// [`DEFAULT`]: PropertyUsageFlags::DEFAULT
// Optimization: we tried removing `C` here to reduce monomorphizations (passing `ClassId` instead) -- not much effect; #(C, T) aren't >> #T.
pub fn register_export<C: GodotClass, T: Export>(
    property_name: &str,
    getter_name: &str,
    setter_name: &str,
    hint_override: Option<PropertyHintInfo>,
    usage_override: Option<PropertyUsageFlags>,
) {
    // `Some` only if a node type is exported from a non-node class, i.e. the case rejected by `register_export_inner()`.
    let disallowed_node_class = (!C::inherits::<classes::Node>())
        .then(T::as_node_class)
        .flatten();

    register_export_inner(
        C::class_id(),
        disallowed_node_class,
        T::godot_shape(),
        property_name,
        getter_name,
        setter_name,
        hint_override,
        usage_override,
    )
}

// Non-generic version to reduce #monomorphizations.
#[expect(clippy::too_many_arguments)]
fn register_export_inner(
    class_id: ClassId,
    disallowed_node_class: Option<ClassId>,
    shape: GodotShape,
    property_name: &str,
    getter_name: &str,
    setter_name: &str,
    hint_override: Option<PropertyHintInfo>,
    usage_override: Option<PropertyUsageFlags>,
) {
    // Note: if the user manually specifies `hint`, `hint_string` or `usage` keys, and thus is routed to `register_var()` instead,
    // they can bypass this validation.
    if let Some(t) = disallowed_node_class {
        panic!(
            "#[export] for Gd<{t}>: nodes can only be exported in Node-derived classes, but current class is {class_id}.",
        );
    }

    register_var_or_export_inner(
        class_id,
        shape,
        true,
        property_name,
        getter_name,
        setter_name,
        hint_override,
        usage_override,
    );
}

/// Registers a `#[var]` property with Godot's ClassDB.
///
/// Defaults are resolved from `T::godot_shape()`: [`var_hint()`] for hints, [`NONE`] for usage flags. Pass `Some(...)` to override either.
///
/// [`var_hint()`]: crate::meta::shape::GodotShape::var_hint
/// [`NONE`]: PropertyUsageFlags::NONE
pub fn register_var<C: GodotClass, T: Var>(
    property_name: &str,
    getter_name: &str,
    setter_name: &str,
    hint_override: Option<PropertyHintInfo>,
    usage_override: Option<PropertyUsageFlags>,
) {
    register_var_or_export_inner(
        C::class_id(),
        T::godot_shape(),
        false,
        property_name,
        getter_name,
        setter_name,
        hint_override,
        usage_override,
    )
}

// Non-generic version to reduce #monomorphizations. Also builds the `PropertyInfo`, to keep it out of the generic callers.
#[expect(clippy::too_many_arguments)]
fn register_var_or_export_inner(
    class_id: ClassId,
    shape: GodotShape,
    is_export: bool,
    property_name: &str,
    getter_name: &str,
    setter_name: &str,
    hint_override: Option<PropertyHintInfo>,
    usage_override: Option<PropertyUsageFlags>,
) {
    let mut info = if is_export {
        shape.to_export_property(property_name)
    } else {
        shape.to_var_property(property_name)
    };

    if let Some(i) = hint_override {
        info.hint_info = i;
    }
    if let Some(u) = usage_override {
        info.usage = u;
    }

    let getter_name = StringName::from(getter_name);
    let setter_name = StringName::from(setter_name);

    crate::registry::reg_validation::validate_property(
        class_id,
        &info.property_name,
        &getter_name,
        &setter_name,
    );

    let property_info_sys = info.property_sys();

    unsafe {
        sys::interface_fn!(classdb_register_extension_class_property)(
            sys::get_library(),
            class_id.string_sys(),
            std::ptr::addr_of!(property_info_sys),
            setter_name.string_sys(),
            getter_name.string_sys(),
        );
    }
}

pub fn register_group<C: GodotClass>(group_name: &str, prefix: &str) {
    let group_name = GString::from(group_name);
    let prefix = GString::from(prefix);
    let class_id = C::class_id();

    unsafe {
        sys::interface_fn!(classdb_register_extension_class_property_group)(
            sys::get_library(),
            class_id.string_sys(),
            group_name.string_sys(),
            prefix.string_sys(),
        );
    }
}

pub fn register_subgroup<C: GodotClass>(subgroup_name: &str, prefix: &str) {
    let subgroup_name = GString::from(subgroup_name);
    let prefix = GString::from(prefix);
    let class_id = C::class_id();

    unsafe {
        sys::interface_fn!(classdb_register_extension_class_property_subgroup)(
            sys::get_library(),
            class_id.string_sys(),
            subgroup_name.string_sys(),
            prefix.string_sys(),
        );
    }
}
