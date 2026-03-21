/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! Internal registration machinery used by proc-macro APIs.

use crate::builtin::{GString, StringName};
use crate::meta::ClassId;
use crate::obj::GodotClass;
use crate::registry::info::{PropertyHintInfo, PropertyInfo, PropertyUsageFlags};
use crate::registry::property::{Export, Var};
use crate::{classes, sys};

/// Registers a `#[export]` property with Godot's ClassDB.
///
/// Statically verifies the `Export` trait and that nodes can only be exported from nodes.
/// Defaults are resolved from `T::godot_shape()`: [`export_hint()`] for hints, [`DEFAULT`] for usage flags. Pass `Some(...)` to override either.
///
/// [`export_hint()`]: crate::meta::shape::GodotShape::export_hint
/// [`DEFAULT`]: PropertyUsageFlags::DEFAULT
pub fn register_export<C: GodotClass, T: Export>(
    property_name: &str,
    getter_name: &str,
    setter_name: &str,
    hint_override: Option<PropertyHintInfo>,
    usage_override: Option<PropertyUsageFlags>,
) {
    // Note: if the user manually specifies `hint`, `hint_string` or `usage` keys, and thus is routed to `register_var()` instead,
    // they can bypass this validation.
    if !C::inherits::<classes::Node>()
        && let Some(class) = T::as_node_class()
    {
        panic!(
            "#[export] for Gd<{t}>: nodes can only be exported in Node-derived classes, but the current class is {c}.",
            t = class,
            c = C::class_id()
        );
    }

    let mut property = T::godot_shape().to_export_property(property_name);
    if let Some(i) = hint_override {
        property.hint_info = i;
    }
    if let Some(u) = usage_override {
        property.usage = u;
    }

    register_var_or_export_inner(property, C::class_id(), getter_name, setter_name);
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
    let mut property = T::godot_shape().to_var_property(property_name);
    if let Some(i) = hint_override {
        property.hint_info = i;
    }
    if let Some(u) = usage_override {
        property.usage = u;
    }

    register_var_or_export_inner(property, C::class_id(), getter_name, setter_name);
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
