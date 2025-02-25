/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

mod markdown_converter;

use crate::class::{ConstDefinition, Field, FuncDefinition, SignalDefinition};
use proc_macro2::{Ident, TokenStream};
use quote::{quote, ToTokens};
use venial::*;

pub fn make_definition_docs(
    base: String,
    description: &[Attribute],
    members: &[Field],
) -> TokenStream {
    let base_escaped = xml_escape(base);
    let Some(desc_escaped) = make_docs_from_attributes(description).map(xml_escape) else {
        return quote! { None };
    };
    let members = members
        .iter()
        .filter(|x| x.var.is_some() | x.export.is_some())
        .filter_map(member)
        .collect::<String>();
    quote! {
        Some(
            ::godot::docs::StructDocs {
                base: #base_escaped,
                description: #desc_escaped,
                members: #members,
            }
        )
    }
}

pub fn make_inherent_impl_docs(
    functions: &[FuncDefinition],
    constants: &[ConstDefinition],
    signals: &[SignalDefinition],
) -> TokenStream {
    /// Generates TokenStream containing field definitions for documented methods and documentation blocks for constants and signals.
    fn pieces(
        functions: &[FuncDefinition],
        signals: &[SignalDefinition],
        constants: &[ConstDefinition],
    ) -> TokenStream {
        let to_tagged = |s: String, tag: &str| -> String {
            if s.is_empty() {
                s
            } else {
                format!("<{tag}>{s}</{tag}>")
            }
        };

        let signals_block = to_tagged(
            signals
                .iter()
                .filter_map(make_signal_docs)
                .collect::<String>(),
            "signals",
        );
        let constants_block = to_tagged(
            constants
                .iter()
                .map(|ConstDefinition { raw_constant }| raw_constant)
                .filter_map(make_constant_docs)
                .collect::<String>(),
            "constants",
        );

        let methods = functions
            .iter()
            .filter_map(make_method_docs)
            .collect::<String>();

        quote! {
            ::godot::docs::InherentImplDocs {
                methods: #methods,
                signals_block: #signals_block,
                constants_block: #constants_block,
            }
        }
    }
    pieces(functions, signals, constants)
}

pub fn make_virtual_impl_docs(vmethods: &[ImplMember]) -> TokenStream {
    let virtual_methods = vmethods
        .iter()
        .filter_map(|x| match x {
            venial::ImplMember::AssocFunction(f) => Some(f.clone()),
            _ => None,
        })
        .filter_map(make_virtual_method_docs)
        .collect::<String>();

    quote! { #virtual_methods }
}

/// `///` is expanded to `#[doc = "…"]`.
/// This function goes through and extracts the …
fn siphon_docs_from_attributes(doc: &[Attribute]) -> impl Iterator<Item = String> + '_ {
    doc.iter()
        // find #[doc]
        .filter(|x| x.get_single_path_segment().is_some_and(|x| x == "doc"))
        // #[doc = "…"]
        .filter_map(|x| match &x.value {
            AttributeValue::Equals(_, doc) => Some(doc),
            _ => None,
        })
        .flat_map(|doc| {
            doc.iter().map(|token_tree| {
                let str = token_tree.to_string();
                litrs::StringLit::parse(str.clone())
                    .map_or(str, |parsed| parsed.value().to_string())
            })
        })
}

fn xml_escape(value: String) -> String {
    // Most strings have no special characters, so this check helps avoid unnecessary string copying
    if !value.contains(['&', '<', '>', '"', '\'']) {
        return value;
    }

    let mut result = String::with_capacity(value.len());

    for c in value.chars() {
        match c {
            '&' => result.push_str("&amp;"),
            '<' => result.push_str("&lt;"),
            '>' => result.push_str("&gt;"),
            '"' => result.push_str("&quot;"),
            '\'' => result.push_str("&#39;"),
            c => result.push(c),
        }
    }

    result
}

/// Calls [`siphon_docs_from_attributes`] and converts the result to BBCode
/// for Godot's consumption.
fn make_docs_from_attributes(doc: &[Attribute]) -> Option<String> {
    let doc = siphon_docs_from_attributes(doc)
        .collect::<Vec<String>>()
        .join("\n");

    (!doc.is_empty()).then(|| markdown_converter::to_bbcode(&doc))
}

fn make_signal_docs(signal: &SignalDefinition) -> Option<String> {
    let name = &signal.signature.name;
    let params = params(signal.signature.params.iter().filter_map(|(x, _)| match x {
        FnParam::Receiver(_) => None,
        FnParam::Typed(y) => Some((&y.name, &y.ty)),
    }));
    let desc = make_docs_from_attributes(&signal.external_attributes)?;
    Some(format!(
        r#"
<signal name="{name}">
  {params}
  <description>
  {desc}
  </description>
</signal>
"#,
        name = xml_escape(name.to_string()),
        desc = xml_escape(desc),
    ))
}

fn make_constant_docs(constant: &Constant) -> Option<String> {
    let docs = make_docs_from_attributes(&constant.attributes)?;
    let name = constant.name.to_string();
    let value = constant
        .initializer
        .as_ref()
        .map(|x| x.to_token_stream().to_string())
        .unwrap_or_else(|| "null".to_string());

    Some(format!(
        r#"<constant name="{name}" value="{value}">{docs}</constant>"#,
        name = xml_escape(name),
        value = xml_escape(value),
        docs = xml_escape(docs),
    ))
}

pub fn member(member: &Field) -> Option<String> {
    let docs = make_docs_from_attributes(&member.attributes)?;
    let name = &member.name;
    let ty = member.ty.to_token_stream().to_string();
    let default = member.default_val.to_token_stream().to_string();
    Some(format!(
        r#"<member name="{name}" type="{ty}" default="{default}">{docs}</member>"#,
        name = xml_escape(name.to_string()),
        ty = xml_escape(ty),
        default = xml_escape(default),
        docs = xml_escape(docs),
    ))
}

fn params<'a, 'b>(params: impl Iterator<Item = (&'a Ident, &'b TypeExpr)>) -> String {
    let mut output = String::new();
    for (index, (name, ty)) in params.enumerate() {
        output.push_str(&format!(
            r#"<param index="{index}" name="{name}" type="{ty}" />"#,
            name = xml_escape(name.to_string()),
            ty = xml_escape(ty.to_token_stream().to_string()),
        ));
    }
    output
}

pub fn make_virtual_method_docs(method: Function) -> Option<String> {
    let desc = make_docs_from_attributes(&method.attributes)?;
    let name = method.name.to_string();
    let ret = method
        .return_ty
        .map(|x| x.to_token_stream().to_string())
        .unwrap_or_else(|| "void".to_string());

    let params = params(method.params.iter().filter_map(|(x, _)| match x {
        FnParam::Receiver(_) => None,
        FnParam::Typed(y) => Some((&y.name, &y.ty)),
    }));
    Some(format!(
        r#"
<method name="_{name}">
  <return type="{ret}" />
  {params}
  <description>
  {desc}
  </description>
</method>
"#,
        name = xml_escape(name),
        ret = xml_escape(ret),
        desc = xml_escape(desc),
    ))
}

pub fn make_method_docs(method: &FuncDefinition) -> Option<String> {
    let desc = make_docs_from_attributes(&method.external_attributes)?;
    let name = method
        .registered_name
        .clone()
        .unwrap_or_else(|| method.rust_ident().to_string());
    let ret = method.signature_info.ret_type.to_token_stream().to_string();
    let params = params(
        method
            .signature_info
            .param_idents
            .iter()
            .zip(&method.signature_info.param_types),
    );
    Some(format!(
        r#"
<method name="{name}">
  <return type="{ret}" />
  {params}
  <description>
  {desc}
  </description>
</method>
"#,
        name = xml_escape(name),
        ret = xml_escape(ret),
        desc = xml_escape(desc),
    ))
}
