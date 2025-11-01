/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::collections::HashMap;

use crate::meta::ClassId;
use crate::obj::GodotClass;

/// Piece of information that is gathered by the self-registration ("plugin") system.
///
/// You should not manually construct this struct, but rather use [`DocsPlugin::new()`].
#[derive(Debug)]
pub struct DocsPlugin {
    /// The name of the class to register docs for.
    class_name: ClassId,

    /// The actual item being registered.
    item: DocsItem,
}

impl DocsPlugin {
    /// Creates a new `DocsPlugin`, automatically setting the `class_name` to the values defined in [`GodotClass`].
    pub fn new<T: GodotClass>(item: DocsItem) -> Self {
        Self {
            class_name: T::class_id(),
            item,
        }
    }
}

type ITraitImplDocs = &'static str;

#[derive(Debug)]
pub enum DocsItem {
    /// Docs for `#[derive(GodotClass)] struct MyClass`.
    Struct(StructDocs),
    /// Docs for `#[godot_api] impl MyClass`.
    InherentImpl(InherentImplDocs),
    /// Docs for `#[godot_api] impl ITrait for MyClass`.
    ITraitImpl(ITraitImplDocs),
}

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
    pub properties: &'static str,
}

/// Keeps documentation for inherent `impl` blocks (primary and secondary), such as:
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
#[derive(Default, Clone, Debug)]
pub struct InherentImplDocs {
    pub methods_xml: &'static str,
    pub signals_xml: &'static str,
    pub constants_xml: &'static str,
}

/// Godot editor documentation for a class, combined from individual definitions (struct + impls).
///
/// All fields are collections of XML parts, escaped where necessary.
#[derive(Default)]
struct AggregatedDocs {
    definition: StructDocs,
    methods_xmls: Vec<&'static str>,
    signals_xmls: Vec<&'static str>,
    constants_xmls: Vec<&'static str>,
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
    let mut map = HashMap::<ClassId, AggregatedDocs>::new();

    crate::private::iterate_docs_plugins(|shard| {
        let class_name = shard.class_name;
        match &shard.item {
            DocsItem::Struct(struct_docs) => {
                map.entry(class_name).or_default().definition = *struct_docs;
            }

            DocsItem::InherentImpl(trait_docs) => {
                map.entry(class_name)
                    .or_default()
                    .methods_xmls
                    .push(trait_docs.methods_xml);

                map.entry(class_name)
                    .and_modify(|pieces| pieces.constants_xmls.push(trait_docs.constants_xml));

                map.entry(class_name)
                    .and_modify(|pieces| pieces.signals_xmls.push(trait_docs.signals_xml));
            }

            DocsItem::ITraitImpl(methods_xml) => {
                map.entry(class_name)
                    .or_default()
                    .methods_xmls
                    .push(methods_xml);
            }
        }
    });

    map.into_iter().map(|(class, pieces)| {
        let StructDocs {
            base,
            description,
            experimental,
            deprecated,
            properties,
        } = pieces.definition;

        let methods_block = wrap_in_xml_block("methods", pieces.methods_xmls);
        let signals_block = wrap_in_xml_block("signals", pieces.signals_xmls);
        let constants_block = wrap_in_xml_block("constants", pieces.constants_xmls);

        let (brief, description) = match description.split_once("[br]") {
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
<members>{properties}</members>
</class>"#)
    })
}

fn wrap_in_xml_block(tag: &str, mut blocks: Vec<&'static str>) -> String {
    // We sort the blocks for deterministic output. No need to sort individual methods/signals/constants, this is already done by Godot.
    // See https://github.com/godot-rust/gdext/pull/1391 for more information.
    blocks.sort();

    let content = String::from_iter(blocks);

    if content.is_empty() {
        String::new()
    } else {
        format!("<{tag}>{content}</{tag}>")
    }
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
