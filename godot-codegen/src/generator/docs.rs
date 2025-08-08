/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! Rustdoc for various symbols.
//!
//! Single module for documentation, rather than having it in each symbol-specific file, so it's easier to keep docs consistent.

use proc_macro2::Ident;

use crate::generator::signals;
use crate::models::domain::{ModName, TyName};
use crate::special_cases;

pub fn make_class_doc(
    class_name: &TyName,
    base_ident_opt: Option<Ident>,
    has_notification_enum: bool,
    has_sidecar_module: bool,
    has_interface_trait: bool,
    has_signal_collection: bool,
) -> String {
    let TyName { rust_ty, godot_ty } = class_name;

    let inherits_line = if let Some(base) = base_ident_opt {
        format!("Inherits [`{base}`][crate::classes::{base}].")
    } else {
        "This is the base class for all other classes at the root of the hierarchy. \
        Every instance of `Object` can be stored in a [`Gd`][crate::obj::Gd] smart pointer."
            .to_string()
    };

    let (sidecar_signal_lines, module_name);
    if has_sidecar_module {
        let module = ModName::from_godot(&class_name.godot_ty).rust_mod;

        sidecar_signal_lines = format!("* [`{module}`][crate::classes::{module}]: sidecar module with related enum/flag types\n");
        module_name = Some(module);
    } else {
        sidecar_signal_lines = String::new();
        module_name = None;
    };

    let signal_line = if has_signal_collection {
        let signal_coll = signals::make_collection_name(class_name);
        let module = module_name.expect("signal implies presence of sidecar module");

        format!("* [`{signal_coll}`][crate::classes::{module}::{signal_coll}]: signal collection\n")
    } else {
        String::new()
    };

    let notify_line = if has_notification_enum {
        format!("* [`{rust_ty}Notification`][crate::classes::notify::{rust_ty}Notification]: notification type\n")
    } else {
        String::new()
    };

    let online_link = format!(
        "https://docs.godotengine.org/en/stable/classes/class_{}.html",
        godot_ty.to_ascii_lowercase()
    );

    let interface_trait_line = if has_interface_trait {
        let trait_name = class_name.virtual_trait_name();
        format!("* [`{trait_name}`][crate::classes::{trait_name}]: virtual methods\n")
    } else {
        String::new()
    };

    let notes = special_cases::get_class_extra_docs(class_name)
        .map(|notes| format!("# Specific notes for this class\n\n{notes}"))
        .unwrap_or_default();

    format!(
        "Godot class `{godot_ty}.`\n\n\
        \
        {inherits_line}\n\n\
        \
        Related symbols:\n\n\
        {sidecar_signal_lines}\
        {interface_trait_line}\
        {signal_line}\
        {notify_line}\
        \n\n\
        See also [Godot docs for `{godot_ty}`]({online_link}).\n\n{notes}",
    )
}

pub fn make_virtual_trait_doc(
    trait_name_str: &str,
    base_traits: &[(String, bool)],
    class_name: &TyName,
) -> String {
    let TyName { rust_ty, godot_ty } = class_name;

    let online_link = format!(
        "https://docs.godotengine.org/en/stable/classes/class_{}.html#methods",
        godot_ty.to_ascii_lowercase()
    );

    let notes = special_cases::get_interface_extra_docs(trait_name_str)
        .map(|notes| format!("# Specific notes for this interface\n\n{notes}"))
        .unwrap_or_default();

    // Detect if a base interface exists. This is not the case if intermediate Godot classes are marked "abstract" (aka final for GDExtension).
    // In such cases, still show interfaces as strikethrough.
    let inherits_line = if base_traits.is_empty() {
        String::new()
    } else {
        let mut parts = vec![];
        let mut strikethrough_explanation = "";
        for (trait_name, is_generated) in base_traits {
            let part = if *is_generated {
                format!("[`{trait_name}`][crate::classes::{trait_name}]")
            } else {
                strikethrough_explanation =
                    "  \n(Strike-through means some intermediate Godot classes are marked final, \
                    and can thus not be inherited by GDExtension.)\n\n";
                format!("~~`{trait_name}`~~")
            };
            parts.push(part);
        }

        format!(
            "\n\nBase interfaces: {}.{}",
            parts.join(" > "),
            strikethrough_explanation
        )
    };

    format!(
        "# Interface trait for class [`{rust_ty}`][crate::classes::{rust_ty}].\
        \n\n\
        Functions in this trait represent constructors (`init`) or virtual method callbacks invoked by the engine.\
        \n\n{notes}\
        \n\n# Related symbols\
        {inherits_line}\
        \n\nSee also [Godot docs for `{godot_ty}` methods]({online_link})."
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
