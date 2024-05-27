/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! Rustdoc for various symbols.
//!
//! Single module for documentation, rather than having it in each symbol-specific file, so it's easier to keep docs consistent.

use crate::models::domain::{ModName, TyName};
use crate::special_cases;
use proc_macro2::Ident;

pub fn make_class_doc(
    class_name: &TyName,
    base_ident_opt: Option<Ident>,
    has_notification_enum: bool,
    has_sidecar_module: bool,
) -> String {
    let TyName { rust_ty, godot_ty } = class_name;

    let inherits_line = if let Some(base) = base_ident_opt {
        format!("Inherits [`{base}`][crate::classes::{base}].")
    } else {
        "This is the base class for all other classes at the root of the hierarchy. \
        Every instance of `Object` can be stored in a [`Gd`][crate::obj::Gd] smart pointer."
            .to_string()
    };

    let notify_line = if has_notification_enum {
        format!("* [`{rust_ty}Notification`][crate::classes::notify::{rust_ty}Notification]: notification type\n")
    } else {
        String::new()
    };

    let sidecar_line = if has_sidecar_module {
        let module_name = ModName::from_godot(&class_name.godot_ty).rust_mod;
        format!("* [`{module_name}`][crate::classes::{module_name}]: sidecar module with related enum/flag types\n")
    } else {
        String::new()
    };

    let online_link = format!(
        "https://docs.godotengine.org/en/stable/classes/class_{}.html",
        godot_ty.to_ascii_lowercase()
    );

    let trait_name = class_name.virtual_trait_name();

    let notes = special_cases::get_class_extra_docs(class_name)
        .map(|notes| format!("# Specific notes for this class\n\n{}", notes))
        .unwrap_or_default();

    format!(
        "Godot class `{godot_ty}.`\n\n\
        \
        {inherits_line}\n\n\
        \
        Related symbols:\n\n\
        {sidecar_line}\
        * [`{trait_name}`][crate::classes::{trait_name}]: virtual methods\n\
        {notify_line}\
        \n\n\
        See also [Godot docs for `{godot_ty}`]({online_link}).\n\n{notes}",
    )
}

pub fn make_virtual_trait_doc(trait_name_str: &str, class_name: &TyName) -> String {
    let TyName { rust_ty, godot_ty } = class_name;

    let online_link = format!(
        "https://docs.godotengine.org/en/stable/classes/class_{}.html#methods",
        godot_ty.to_ascii_lowercase()
    );

    let notes = special_cases::get_interface_extra_docs(trait_name_str)
        .map(|notes| format!("# Specific notes for this interface\n\n{}", notes))
        .unwrap_or_default();

    format!(
        "Virtual methods for class [`{rust_ty}`][crate::classes::{rust_ty}].\
        \n\n\
        These methods represent constructors (`init`) or callbacks invoked by the engine.\
        \n\n\
        See also [Godot docs for `{godot_ty}` methods]({online_link}).\n\n{notes}"
    )
}

pub fn make_module_doc(class_name: &TyName) -> String {
    let TyName { rust_ty, godot_ty } = class_name;

    let online_link = format!(
        "https://docs.godotengine.org/en/stable/classes/class_{}.html#enumerations",
        godot_ty.to_ascii_lowercase()
    );

    format!(
        "Sidecar module for class [`{rust_ty}`][crate::classes::{rust_ty}].\
        \n\n\
        Defines related flag and enum types. In GDScript, those are nested under the class scope.\
        \n\n\
        See also [Godot docs for `{godot_ty}` enums]({online_link}).\n\n"
    )
}
