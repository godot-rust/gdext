/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
use crate::registry::plugin::PluginItem;
use std::collections::HashMap;

/// Created for documentation on
/// ```ignore
/// #[derive(GodotClass)]
/// /// Documented
/// struct Struct {
///    /// documented
///    x: f32,
/// }
/// ```
#[derive(Clone, Copy, Debug, Default)]
pub struct StructDocs {
    pub base: &'static str,
    pub description: &'static str,
    pub members: &'static str,
}

/// Created for documentation on
/// ```ignore
/// #[godot_api]
/// impl Struct {
///     #[func]
///     /// This function panics!
///     fn panic() -> f32 { panic!() }
/// }
/// ```
#[derive(Clone, Copy, Debug, Default)]
pub struct InherentImplDocs {
    pub methods: &'static str,
    pub signals: &'static str,
    pub constants: &'static str,
}

#[derive(Default)]
struct DocPieces {
    definition: StructDocs,
    inherent: InherentImplDocs,
    virtual_methods: &'static str,
}

#[doc(hidden)]
/// This function scours the registered plugins to find their documentation pieces,
/// and strings them together.
///
/// It returns an iterator over XML documents.
pub fn gather_xml_docs() -> impl Iterator<Item = String> {
    let mut map = HashMap::<&'static str, DocPieces>::new();
    crate::private::iterate_plugins(|x| match x.item {
        PluginItem::InherentImpl {
            docs: Some(docs), ..
        } => map.entry(x.class_name.as_str()).or_default().inherent = docs,
        PluginItem::ITraitImpl {
            virtual_method_docs,
            ..
        } => {
            map.entry(x.class_name.as_str())
                .or_default()
                .virtual_methods = virtual_method_docs
        }
        PluginItem::Struct {
            docs: Some(docs), ..
        } => map.entry(x.class_name.as_str()).or_default().definition = docs,
        _ => (),
    });
    map.into_iter().map(|(class, pieces)| {
            let StructDocs {
                base,
                description,
                members,
            } = pieces.definition;

            let InherentImplDocs {
                methods,
                signals,
                constants,
            } = pieces.inherent;

            let virtual_methods = pieces.virtual_methods;
            let brief = description.split_once("[br]").map(|(x, _)| x).unwrap_or_default();
format!(r#"
<?xml version="1.0" encoding="UTF-8"?>
<class name="{class}" inherits="{base}" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xsi:noNamespaceSchemaLocation="../class.xsd">
<brief_description>{brief}</brief_description>
<description>{description}</description>
<methods>{methods}{virtual_methods}</methods>
<constants>{constants}</constants>
<signals>{signals}</signals>
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
