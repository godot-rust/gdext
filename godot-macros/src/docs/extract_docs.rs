/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
use proc_macro2::{Ident, TokenStream};
use quote::{quote, ToTokens};

use crate::class::{ConstDefinition, Field, FuncDefinition, SignalDefinition};
use crate::docs::markdown_converter;

#[derive(Default)]
struct XmlParagraphs {
    /// XML content, as BBCode, to be used in `description` tag: `<description>VALUE</description>`.
    description_content: String,
    /// XML attribute, as BBCode: `experimental="EXPERIMENTAL"`.
    /// Contains whole paragraph annotated with an `@experimental` tag.
    experimental_attr: String,
    /// XML attribute, as BBCode: `deprecated="DEPRECATED"`.
    /// Contains whole paragraph annotated with a `@deprecated` tag.
    deprecated_attr: String,
}

pub struct InherentImplXmlDocs {
    pub method_xml_elems: String,
    pub constant_xml_elems: String,
    pub signal_xml_elems: String,
}

/// Returns code containing the doc information of a `#[derive(GodotClass)] struct MyClass` declaration iff class or any of its members is documented.
pub fn document_struct(
    base: String,
    description: &[venial::Attribute],
    fields: &[Field],
) -> TokenStream {
    let XmlParagraphs {
        description_content,
        deprecated_attr,
        experimental_attr,
    } = attribute_docs_to_xml_paragraphs(description).unwrap_or_default();

    let properties = fields
        .iter()
        .filter(|field| field.var.is_some() || field.export.is_some())
        .filter_map(format_member_xml)
        .collect::<String>();

    let base_escaped = xml_escape(base);

    quote! {
        ::godot::docs::StructDocs {
            base: #base_escaped,
            description: #description_content,
            experimental: #experimental_attr,
            deprecated: #deprecated_attr,
            properties: #properties,
        }
    }
}

/// Returns code containing the doc information of a `#[godot_api] impl MyClass` declaration.
pub fn document_inherent_impl(
    functions: &[FuncDefinition],
    constants: &[ConstDefinition],
    signals: &[SignalDefinition],
) -> InherentImplXmlDocs {
    let signal_xml_elems = signals
        .iter()
        .filter_map(format_signal_xml)
        .collect::<String>();

    let constant_xml_elems = constants
        .iter()
        .map(|ConstDefinition { raw_constant }| raw_constant)
        .filter_map(format_constant_xml)
        .collect::<String>();

    let method_xml_elems = functions
        .iter()
        .filter_map(format_method_xml)
        .collect::<String>();

    InherentImplXmlDocs {
        method_xml_elems,
        constant_xml_elems,
        signal_xml_elems,
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
                litrs::StringLit::try_from(token_tree)
                    .map_or_else(|_| token_tree.to_string(), |parsed| parsed.into_value())
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

/// Extracts docs from attributes and groups them in three Strings:
/// user documentation content and paragraphs annotated with `@deprecated` or `@experimental` tags.
fn docs_with_attributes(doc: &[venial::Attribute]) -> (String, String, String) {
    let (mut docs, mut deprecated, mut experimental) =
        (String::new(), String::new(), String::new());

    // Allows to compare the current bucket (the one we put current paragraph in) with docs one.
    let docs_bucket = std::ptr::from_ref(&docs);
    let mut current_bucket: &mut String = &mut docs;

    for line in extract_docs_from_attributes(doc) {
        let trimmed = line.trim_start();

        // End of the paragraph (`#[doc=""]` or `///`) .
        if trimmed.is_empty() {
            // Switch back from attribute docs to user docs when paragraph ends.
            // Don't double newlines after XML attribute tags descriptions.
            if !std::ptr::eq(current_bucket, docs_bucket) {
                current_bucket = &mut docs;
            } else {
                current_bucket.push('\n');
            }
            continue;
        }

        // Check for `/// @deprecated` ... or `/// @experimental`
        if trimmed.starts_with("@deprecated") {
            current_bucket = &mut deprecated;
            current_bucket.push_str(trimmed.trim_start_matches("@deprecated"));
        } else if trimmed.starts_with("@experimental") {
            current_bucket = &mut experimental;
            current_bucket.push_str(trimmed.trim_start_matches("@experimental"));
        } else {
            current_bucket.push_str(&line);
            current_bucket.push('\n');
        }
    }

    (docs, deprecated, experimental)
}

/// Converts attribute docs to form suitable for Godot's consumption.
///
/// See also: [`XmlParagraphs`].
fn attribute_docs_to_xml_paragraphs(doc: &[venial::Attribute]) -> Option<XmlParagraphs> {
    let (docs, deprecated, experimental) = docs_with_attributes(doc);

    if docs.is_empty() && deprecated.is_empty() && experimental.is_empty() {
        return None;
    }

    let to_bbcode: fn(String) -> Option<String> =
        |piece| (!piece.is_empty()).then(|| markdown_converter::to_bbcode(&piece));

    let to_xml_attribute: fn(String, &str) -> String =
        // Mind the whitespace before XML attribute declaration.
        |description, attribute| format!(" {attribute}=\"{description}\"");

    Some(XmlParagraphs {
        description_content: to_bbcode(docs).map(xml_escape).unwrap_or_default(),
        deprecated_attr: to_bbcode(deprecated)
            .map(xml_escape)
            .map(|s| to_xml_attribute(s, "deprecated"))
            .unwrap_or_default(),
        experimental_attr: to_bbcode(experimental)
            .map(xml_escape)
            .map(|s| to_xml_attribute(s, "experimental"))
            .unwrap_or_default(),
    })
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

    let XmlParagraphs {
        description_content,
        deprecated_attr,
        experimental_attr,
    } = attribute_docs_to_xml_paragraphs(&signal.external_attributes)?;

    Some(format!(
        r#"
<signal name="{name}"{deprecated_attr}{experimental_attr}>
  {params}
  <description>
  {description_content}
  </description>
</signal>
"#
    ))
}

fn format_constant_xml(constant: &venial::Constant) -> Option<String> {
    let XmlParagraphs {
        description_content,
        deprecated_attr,
        experimental_attr,
    } = attribute_docs_to_xml_paragraphs(&constant.attributes)?;

    let name = constant.name.to_string();
    let value = constant
        .initializer
        .as_ref()
        .map(|x| x.to_token_stream().to_string())
        .unwrap_or_else(|| "null".to_string());

    Some(format!(
        r#"<constant name="{name}" value="{value}"{deprecated_attr}{experimental_attr}>{description_content}</constant>"#,
        name = xml_escape(name),
        value = xml_escape(value),
    ))
}

pub fn format_member_xml(member: &Field) -> Option<String> {
    let XmlParagraphs {
        description_content,
        deprecated_attr,
        experimental_attr,
    } = attribute_docs_to_xml_paragraphs(&member.attributes)?;
    let name = &member.name;
    let ty = member.ty.to_token_stream().to_string();
    let default = member.default_val.to_token_stream().to_string();

    Some(format!(
        r#"<member name="{name}" type="{ty}" default="{default}"{deprecated_attr}{experimental_attr}>{description_content}</member>"#,
        name = xml_escape(name.to_string()),
        ty = xml_escape(ty),
        default = xml_escape(default),
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
    let XmlParagraphs {
        description_content,
        deprecated_attr,
        experimental_attr,
    } = attribute_docs_to_xml_paragraphs(&method.attributes)?;

    if !deprecated_attr.is_empty() || !experimental_attr.is_empty() {
        panic!("Virtual methods can't be documented as neither `@experimental` nor `@deprecated`.");
    }

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
  {description_content}
  </description>
</method>
"#
    ))
}

fn format_method_xml(method: &FuncDefinition) -> Option<String> {
    let XmlParagraphs {
        description_content,
        deprecated_attr,
        experimental_attr,
    } = attribute_docs_to_xml_paragraphs(&method.external_attributes)?;

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
<method name="{name}"{deprecated_attr}{experimental_attr}>
  <return type="{return_ty}" />
  {params}
  <description>
  {description_content}
  </description>
</method>
"#
    ))
}
