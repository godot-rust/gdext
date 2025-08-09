/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::collections::HashMap;

use crate::meta::ClassName;
use crate::registry::plugin::{ITraitImpl, InherentImpl, PluginItem, Struct};

/// Created for documentation on
/// ```ignore
/// #[derive(GodotClass)]
/// /// Documented
/// struct Struct {
///    /// documented
///    x: f32,
/// }
/// ```
/// All fields are XML parts, escaped where necessary.
#[derive(Default, Copy, Clone, Debug)]
pub struct StructDocs {
    pub base: &'static str,
    pub description: &'static str,
    pub experimental: &'static str,
    pub deprecated: &'static str,
    pub members: &'static str,
}

/// Keeps documentation for inherent `impl` blocks, such as:
/// ```ignore
/// #[godot_api]
/// impl Struct {
///     /// This function panics!
///     #[func]
///     fn panic() -> f32 { panic!() }
///     /// this signal signals
///     #[signal]
///     fn documented_signal(p: Vector3, w: f64);
///     /// this constant consts
///     #[constant]
///     const CON: i64 = 42;
///
/// }
/// ```
/// All fields are XML parts, escaped where necessary.
#[derive(Default, Copy, Clone, Debug)]
pub struct InherentImplDocs {
    pub methods: &'static str,
    pub signals_block: &'static str,
    pub constants_block: &'static str,
}

#[derive(Default)]
struct DocPieces {
    definition: StructDocs,
    inherent: InherentImplDocs,
    virtual_methods: &'static str,
}

/// This function scours the registered plugins to find their documentation pieces,
/// and strings them together.
///
/// Returns an iterator over XML documents.
///
/// Documentation for signals and constants is being processed at compile time
/// and can take the form of an already formatted XML `<block><doc></doc>…</block>`, or an
/// empty string if no such attribute has been documented.
///
/// Since documentation for methods comes from two different sources
/// -- inherent implementations (`methods`) and `I*` trait implementations (`virtual_method_docs`) --
/// it is undesirable to merge them at compile time. Instead, they are being kept as a
/// strings of not-yet-parented XML tags (or empty string if no method has been documented).
#[doc(hidden)]
pub fn gather_xml_docs() -> impl Iterator<Item = String> {
    let mut map = HashMap::<ClassName, DocPieces>::new();
    crate::private::iterate_plugins(|x| {
        let class_name = x.class_name;

        match x.item {
            PluginItem::InherentImpl(InherentImpl { docs, .. }) => {
                map.entry(class_name).or_default().inherent = docs
            }

            PluginItem::ITraitImpl(ITraitImpl {
                virtual_method_docs,
                ..
            }) => map.entry(class_name).or_default().virtual_methods = virtual_method_docs,

            PluginItem::Struct(Struct { docs, .. }) => {
                map.entry(class_name).or_default().definition = docs
            }

            _ => (),
        }
    });

    map.into_iter().map(|(class, pieces)| {
            let StructDocs {
                base,
                description,
                experimental,
                deprecated,
                members,
            } = pieces.definition;

            let InherentImplDocs {
                methods,
                signals_block,
                constants_block,
            } = pieces.inherent;

            let virtual_methods = pieces.virtual_methods;
            let methods_block = (virtual_methods.is_empty() && methods.is_empty())
                .then(String::new)
                .unwrap_or_else(|| format!("<methods>{methods}{virtual_methods}</methods>"));

            let (brief, description) = match description
                .split_once("[br]") {
                    Some((brief, description)) => (brief, description.trim_start_matches("[br]")),
                    None => (description, ""),
                };

            format!(r#"<?xml version="1.0" encoding="UTF-8"?>
<class name="{class}" inherits="{base}"{deprecated}{experimental} xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xsi:noNamespaceSchemaLocation="../class.xsd">
<brief_description>{brief}</brief_description>
<description>{description}</description>
{methods_block}
{constants_block}
{signals_block}
<members>{members}</members>
</class>"#)
        },
        )
}

/// # Safety
///
/// The Godot binding must have been initialized before calling this function.
///
/// If "experimental-threads" is not enabled, then this must be called from the same thread that the bindings were initialized from.
pub unsafe fn register() {
    for xml in gather_xml_docs() {
        crate::sys::interface_fn!(editor_help_load_xml_from_utf8_chars_and_len)(
            xml.as_ptr() as *const std::ffi::c_char,
            xml.len() as i64,
        );
    }
}
