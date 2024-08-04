/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! Internal registration machinery used by proc-macro APIs.

use crate::builtin::StringName;
use crate::global::PropertyUsageFlags;
use crate::meta::{ClassName, GodotConvert, GodotType, PropertyHintInfo, PropertyInfo};
use crate::obj::GodotClass;
use crate::registry::property::Var;
use crate::sys;
use godot_ffi::GodotFfi;

pub fn register_var_or_export<C: GodotClass, T: Var>(
    property_name: &str,
    getter_name: &str,
    setter_name: &str,
    hint_info: PropertyHintInfo,
    usage: PropertyUsageFlags,
) {
    let info = PropertyInfo {
        variant_type: <<T as GodotConvert>::Via as GodotType>::Ffi::variant_type(),
        class_name: <T as GodotConvert>::Via::class_name(),
        property_name: StringName::from(property_name),
        hint_info,
        usage,
    };

    let class_name = C::class_name();

    register_var_or_export_inner(info, class_name, getter_name, setter_name);
}

fn register_var_or_export_inner(
    info: PropertyInfo,
    class_name: ClassName,
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
