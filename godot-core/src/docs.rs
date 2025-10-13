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
    pub members: &'static str,
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
    pub methods: &'static str,
    pub signals: &'static str,
    pub constants: &'static str,
}

#[derive(Default)]
struct DocPieces {
    definition: StructDocs,
    methods: Vec<&'static str>,
    signals: Vec<&'static str>,
    constants: Vec<&'static str>,
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
    let mut map = HashMap::<ClassId, DocPieces>::new();
    crate::private::iterate_docs_plugins(|x| {
        let class_name = x.class_name;
        match &x.item {
            DocsItem::Struct(s) => {
                map.entry(class_name).or_default().definition = *s;
            }
            DocsItem::InherentImpl(trait_docs) => {
                let InherentImplDocs {
                    methods,
                    constants,
                    signals,
                } = trait_docs;
                map.entry(class_name).or_default().methods.push(methods);
                map.entry(class_name)
                    .and_modify(|pieces| pieces.constants.push(constants));
                map.entry(class_name)
                    .and_modify(|pieces| pieces.signals.push(signals));
            }
            DocsItem::ITraitImpl(methods) => {
                map.entry(class_name).or_default().methods.push(methods);
            }
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


            let method_docs = String::from_iter(pieces.methods);
            let signal_docs = String::from_iter(pieces.signals);
            let constant_docs = String::from_iter(pieces.constants);

            let methods_block = if method_docs.is_empty() {
                String::new()
            } else {
                format!("<methods>{method_docs}</methods>")
            };
            let signals_block = if signal_docs.is_empty() {
                String::new()
            } else {
                format!("<signals>{signal_docs}</signals>")
            };
            let constants_block = if constant_docs.is_empty() {
                String::new()
            } else {
                format!("<constants>{constant_docs}</constants>")
            };
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
