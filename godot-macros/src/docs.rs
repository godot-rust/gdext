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

/// Returns code containing the doc information of a `#[derive(GodotClass)] struct MyClass` declaration.
pub fn document_struct(
    base: String,
    description: &[venial::Attribute],
    fields: &[Field],
) -> TokenStream {
    let base_escaped = xml_escape(base);
    let Some(desc_escaped) = attribute_docs_to_bbcode(description).map(xml_escape) else {
        return quote! { None };
    };

    let members = fields
        .iter()
        .filter(|field| field.var.is_some() || field.export.is_some())
        .filter_map(format_member_xml)
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

/// Returns code containing the doc information of a `#[godot_api] impl MyClass` declaration.
pub fn document_inherent_impl(
    functions: &[FuncDefinition],
    constants: &[ConstDefinition],
    signals: &[SignalDefinition],
) -> TokenStream {
    let group_xml_block = |s: String, tag: &str| -> String {
        if s.is_empty() {
            s
        } else {
            format!("<{tag}>{s}</{tag}>")
        }
    };

    let signal_xml_elems = signals
        .iter()
        .filter_map(format_signal_xml)
        .collect::<String>();
    let signals_block = group_xml_block(signal_xml_elems, "signals");

    let constant_xml_elems = constants
        .iter()
        .map(|ConstDefinition { raw_constant }| raw_constant)
        .filter_map(format_constant_xml)
        .collect::<String>();
    let constants_block = group_xml_block(constant_xml_elems, "constants");

    let method_xml_elems = functions
        .iter()
        .filter_map(format_method_xml)
        .collect::<String>();

    quote! {
        ::godot::docs::InherentImplDocs {
            methods: #method_xml_elems,
            signals_block: #signals_block,
            constants_block: #constants_block,
        }
    }
}

/// Returns code containing the doc information of a `#[godot_api] impl ITrait for MyClass` declaration.
pub fn document_interface_trait_impl(impl_members: &[venial::ImplMember]) -> TokenStream {
    let interface_methods = impl_members
        .iter()
        .filter_map(|x| match x {
            venial::ImplMember::AssocFunction(f) => Some(f.clone()),
            _ => None,
        })
        .filter_map(format_virtual_method_xml)
        .collect::<String>();

    quote! { #interface_methods }
}

/// `///` is expanded to `#[doc = "…"]`.
///
/// This function goes through and extracts the "…" part.
fn extract_docs_from_attributes(doc: &[venial::Attribute]) -> impl Iterator<Item = String> + '_ {
    doc.iter()
        // Find #[doc].
        .filter(|x| x.get_single_path_segment().is_some_and(|x| x == "doc"))
        // Limit to occurrences with syntax #[doc = "…"].
        .filter_map(|x| match &x.value {
            venial::AttributeValue::Equals(_, doc) => Some(doc),
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
    // Most strings have no special characters, so this check helps avoid unnecessary string copying.
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

/// Calls [`extract_docs_from_attributes`] and converts the result to BBCode for Godot's consumption.
fn attribute_docs_to_bbcode(doc: &[venial::Attribute]) -> Option<String> {
    let doc = extract_docs_from_attributes(doc)
        .collect::<Vec<String>>()
        .join("\n");

    (!doc.is_empty()).then(|| markdown_converter::to_bbcode(&doc))
}

fn format_venial_params_xml(params: &venial::Punctuated<venial::FnParam>) -> String {
    let non_receiver_params = params.iter().filter_map(|(param, _punct)| match param {
        venial::FnParam::Receiver(_) => None,
        venial::FnParam::Typed(p) => Some((&p.name, &p.ty)),
    });

    format_params_xml(non_receiver_params)
}

fn format_signal_xml(signal: &SignalDefinition) -> Option<String> {
    let name = &signal.fn_signature.name;
    let name = xml_escape(name.to_string());

    let params = format_venial_params_xml(&signal.fn_signature.params);

    let desc = attribute_docs_to_bbcode(&signal.external_attributes)?;
    let desc = xml_escape(desc);

    Some(format!(
        r#"
<signal name="{name}">
  {params}
  <description>
  {desc}
  </description>
</signal>
"#
    ))
}

fn format_constant_xml(constant: &venial::Constant) -> Option<String> {
    let docs = attribute_docs_to_bbcode(&constant.attributes)?;
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

pub fn format_member_xml(member: &Field) -> Option<String> {
    let docs = attribute_docs_to_bbcode(&member.attributes)?;
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

fn format_params_xml<'a, 'b>(
    params: impl Iterator<Item = (&'a Ident, &'b venial::TypeExpr)>,
) -> String {
    use std::fmt::Write;

    let mut output = String::new();
    for (index, (name, ty)) in params.enumerate() {
        write!(
            output,
            r#"<param index="{index}" name="{name}" type="{ty}" />"#,
            name = xml_escape(name.to_string()),
            ty = xml_escape(ty.to_token_stream().to_string()),
        )
        .expect("write to string failed");
    }
    output
}

fn format_virtual_method_xml(method: venial::Function) -> Option<String> {
    let desc = attribute_docs_to_bbcode(&method.attributes)?;
    let desc = xml_escape(desc);

    let name = method.name.to_string();
    let name = xml_escape(name);

    let return_ty = method
        .return_ty
        .map(|ty| ty.to_token_stream().to_string())
        .unwrap_or_else(|| "void".to_string());
    let return_ty = xml_escape(return_ty);

    let params = format_venial_params_xml(&method.params);

    Some(format!(
        r#"
<method name="_{name}">
  <return type="{return_ty}" />
  {params}
  <description>
  {desc}
  </description>
</method>
"#
    ))
}

fn format_method_xml(method: &FuncDefinition) -> Option<String> {
    let desc = attribute_docs_to_bbcode(&method.external_attributes)?;
    let desc = xml_escape(desc);

    let name = method
        .registered_name
        .clone()
        .unwrap_or_else(|| method.rust_ident().to_string());
    let name = xml_escape(name);

    let signature = &method.signature_info;

    let return_ty = signature.return_type.to_token_stream().to_string();
    let return_ty = xml_escape(return_ty);

    let param_names_and_types = signature.param_idents.iter().zip(&signature.param_types);
    let params = format_params_xml(param_names_and_types);

    Some(format!(
        r#"
<method name="{name}">
  <return type="{return_ty}" />
  {params}
  <description>
  {desc}
  </description>
</method>
"#
    ))
}
