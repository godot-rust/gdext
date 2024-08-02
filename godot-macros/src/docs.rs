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
    (|| {
        let desc = make_docs_from_attributes(description)?;
        let members = members
            .into_iter()
            .filter(|x| x.var.is_some() | x.export.is_some())
            .filter_map(member)
            .collect::<String>();
        Some(quote! {
            docs: ::godot::docs::StructDocs {
                base: #base,
                description: #desc,
                members: #members,
            }.into()
        })
    })()
    .unwrap_or(quote! { docs: None })
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
            docs: ::godot::docs::InherentImplDocs {
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

    quote! { virtual_method_docs: #virtual_methods, }
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
            doc.into_iter().map(|x| {
                x.to_string()
                    .trim_start_matches('r')
                    .trim_start_matches('#')
                    .trim_start_matches('"')
                    .trim_end_matches('#')
                    .trim_end_matches('"')
                    .to_string()
            })
        })
}

/// Calls [`siphon_docs_from_attributes`] and converts the result to BBCode
/// for Godot's consumption.
fn make_docs_from_attributes(doc: &[Attribute]) -> Option<String> {
    let doc = siphon_docs_from_attributes(doc)
        .collect::<Vec<_>>()
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
"#
    ))
}

fn make_constant_docs(constant: &Constant) -> Option<String> {
    let docs = make_docs_from_attributes(&constant.attributes)?;
    let name = constant.name.to_string();
    let value = constant
        .initializer
        .as_ref()
        .map(|x| x.to_token_stream().to_string())
        .unwrap_or("null".into());
    Some(format!(
        r#"<constant name="{name}" value="{value}">{docs}</constant>"#
    ))
}

pub fn member(member: &Field) -> Option<String> {
    let docs = make_docs_from_attributes(&member.attributes)?;
    let name = &member.name;
    let ty = member.ty.to_token_stream().to_string();
    let default = member.default.to_token_stream().to_string();
    Some(format!(
        r#"<member name="{name}" type="{ty}" default="{default}">{docs}</member>"#
    ))
}

fn params<'a, 'b>(params: impl Iterator<Item = (&'a Ident, &'b TypeExpr)>) -> String {
    let mut output = String::new();
    for (index, (name, ty)) in params.enumerate() {
        output.push_str(&format!(
            r#"<param index="{index}" name="{name}" type="{ty}" />"#,
            ty = ty.to_token_stream()
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
        .unwrap_or("void".into());
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
"#
    ))
}

pub fn make_method_docs(method: &FuncDefinition) -> Option<String> {
    let desc = make_docs_from_attributes(&method.external_attributes)?;
    let name = method
        .rename
        .clone()
        .unwrap_or_else(|| method.signature_info.method_name.to_string());
    let ret = method.signature_info.ret_type.to_token_stream();
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
"#
    ))
}
