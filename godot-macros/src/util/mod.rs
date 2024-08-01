/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

// Note: some code duplication with godot-codegen crate.

use crate::ParseResult;
use proc_macro2::{Delimiter, Group, Ident, Literal, TokenStream, TokenTree};
use quote::spanned::Spanned;
use quote::{format_ident, quote, ToTokens, TokenStreamExt};

mod kv_parser;
mod list_parser;

pub(crate) use kv_parser::KvParser;
pub(crate) use list_parser::ListParser;

pub fn ident(s: &str) -> Ident {
    format_ident!("{}", s)
}

pub fn c_str(string: &str) -> Literal {
    let c_string = std::ffi::CString::new(string).expect("CString::new() failed");
    Literal::c_string(&c_string)
}

pub fn class_name_obj(class: &impl ToTokens) -> TokenStream {
    let class = class.to_token_stream();
    quote! { <#class as ::godot::obj::GodotClass>::class_name() }
}

pub fn property_variant_type(property_type: &impl ToTokens) -> TokenStream {
    let property_type = property_type.to_token_stream();
    quote! { <<#property_type as ::godot::meta::GodotConvert>::Via as ::godot::meta::GodotType>::Ffi::variant_type() }
}

pub fn property_variant_class_name(property_type: &impl ToTokens) -> TokenStream {
    let property_type = property_type.to_token_stream();
    quote! { <<#property_type as ::godot::meta::GodotConvert>::Via as ::godot::meta::GodotType>::class_name() }
}

pub fn bail_fn<R, T>(msg: impl AsRef<str>, tokens: T) -> ParseResult<R>
where
    T: Spanned,
{
    // TODO: using T: Spanned often only highlights the first tokens of the symbol, e.g. #[attr] in a function.
    // Could use function.name; possibly our own trait to get a more meaningful span... or change upstream in venial.

    Err(error_fn(msg, tokens))
}

macro_rules! bail {
    ($tokens:expr, $format_string:literal $($rest:tt)*) => {
        $crate::util::bail_fn(format!($format_string $($rest)*), $tokens)
    }
}

macro_rules! require_api_version {
    ($min_version:literal, $span:expr, $attribute:literal) => {
        if !cfg!(since_api = $min_version) {
            bail!(
                $span,
                "{} requires at least Godot API version {}",
                $attribute,
                $min_version
            )
        } else {
            Ok(())
        }
    };
}

pub fn error_fn<T>(msg: impl AsRef<str>, tokens: T) -> venial::Error
where
    T: Spanned,
{
    venial::Error::new_at_span(tokens.__span(), msg.as_ref())
}

macro_rules! error {
    ($tokens:expr, $format_string:literal $($rest:tt)*) => {
        $crate::util::error_fn(format!($format_string $($rest)*), $tokens)
    }
}

pub(crate) use bail;
pub(crate) use error;
pub(crate) use require_api_version;

pub fn reduce_to_signature(function: &venial::Function) -> venial::Function {
    let mut reduced = function.clone();
    reduced.vis_marker = None; // TODO needed?
    reduced.attributes.clear();
    reduced.tk_semicolon = None;
    reduced.body = None;

    reduced
}

pub fn parse_signature(mut signature: TokenStream) -> venial::Function {
    // Signature needs {} body to be parseable by venial
    signature.append(TokenTree::Group(Group::new(
        Delimiter::Brace,
        TokenStream::new(),
    )));

    let function_item = venial::parse_item(signature)
        .unwrap()
        .as_function()
        .unwrap()
        .clone();

    reduce_to_signature(&function_item)
}

/// Returns a type expression that can be used as a `VarcallSignatureTuple`.
pub fn make_signature_tuple_type(
    ret_type: &TokenStream,
    param_types: &[venial::TypeExpr],
) -> TokenStream {
    quote::quote! {
        (#ret_type, #(#param_types),*)
    }
}

fn is_punct(tt: &TokenTree, c: char) -> bool {
    match tt {
        TokenTree::Punct(punct) => punct.as_char() == c,
        _ => false,
    }
}

fn delimiter_opening_char(delimiter: Delimiter) -> char {
    match delimiter {
        Delimiter::Parenthesis => '(',
        Delimiter::Brace => '{',
        Delimiter::Bracket => '[',
        Delimiter::None => 'Ø',
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Validation for trait/impl

/// Given an impl block for a trait, returns whether that is an impl for a trait with the given name.
///
/// That is, if `name` is `"MyTrait"`, then this function returns true if and only if `original_impl` is a
/// declaration of the form `impl MyTrait for SomeType`. The type `SomeType` is irrelevant in this example.
pub(crate) fn is_impl_named(original_impl: &venial::Impl, name: &str) -> bool {
    let trait_name = original_impl.trait_ty.as_ref().unwrap(); // unwrap: already checked outside
    extract_typename(trait_name).map_or(false, |seg| seg.ident == name)
}

/// Validates either:
/// a) the declaration is `impl Trait for SomeType`, if `expected_trait` is `Some("Trait")`
/// b) the declaration is `impl SomeType`, if `expected_trait` is `None`
pub(crate) fn validate_impl(
    original_impl: &venial::Impl,
    expected_trait: Option<&str>,
    attr: &str,
) -> ParseResult<Ident> {
    if let Some(expected_trait) = expected_trait {
        // impl Trait for Self -- validate Trait
        if !is_impl_named(original_impl, expected_trait) {
            return bail!(
                original_impl,
                "#[{attr}] for trait impls requires trait to be `{expected_trait}`",
            );
        }
    }

    // impl Trait for Self -- validate Self
    validate_self(original_impl, attr)
}

/// Validates that the declaration is the of the form `impl Trait for SomeType`, where the name
/// of `Trait` begins with `I`.
pub(crate) fn validate_trait_impl_virtual<'a>(
    original_impl: &'a venial::Impl,
    attr: &str,
) -> ParseResult<(Ident, &'a venial::TypeExpr)> {
    let trait_name = original_impl.trait_ty.as_ref().unwrap(); // unwrap: already checked outside
    let typename = extract_typename(trait_name);

    // Validate trait
    if !typename
        .as_ref()
        .map_or(false, |seg| seg.ident.to_string().starts_with('I'))
    {
        return bail!(
            original_impl,
            "#[{attr}] for trait impls requires a virtual method trait (trait name should start with 'I')",
        );
    }

    // Validate self
    validate_self(original_impl, attr).map(|class_name| {
        // let trait_name = typename.unwrap(); // unwrap: already checked in 'Validate trait'
        (class_name, trait_name)
    })
}

fn validate_self(original_impl: &venial::Impl, attr: &str) -> ParseResult<Ident> {
    if let Some(segment) = extract_typename(&original_impl.self_ty) {
        if segment.generic_args.is_none() {
            Ok(segment.ident)
        } else {
            bail!(
                original_impl,
                "#[{attr}] for does currently not support generic arguments",
            )
        }
    } else {
        bail!(
            original_impl,
            "#[{attr}] requires Self type to be a simple path",
        )
    }
}

/// Gets the right-most type name in the path.
fn extract_typename(ty: &venial::TypeExpr) -> Option<venial::PathSegment> {
    match ty.as_path() {
        Some(mut path) => path.segments.pop(),
        _ => None,
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

pub(crate) fn path_is_single(path: &[TokenTree], expected: &str) -> bool {
    path.len() == 1 && path[0].to_string() == expected
}

pub(crate) fn path_ends_with(path: &[TokenTree], expected: &str) -> bool {
    // Could also use TypeExpr::as_path(), or fn below this one.
    path.last()
        .map(|last| last.to_string() == expected)
        .unwrap_or(false)
}

pub(crate) fn path_ends_with_complex(path: &venial::TypeExpr, expected: &str) -> bool {
    path.as_path()
        .map(|path| {
            path.segments
                .last()
                .map_or(false, |seg| seg.ident == expected)
        })
        .unwrap_or(false)
}

pub(crate) fn extract_cfg_attrs(
    attrs: &[venial::Attribute],
) -> impl IntoIterator<Item = &venial::Attribute> {
    attrs.iter().filter(|attr| {
        attr.get_single_path_segment()
            .map_or(false, |name| name == "cfg")
    })
}

#[cfg(before_api = "4.3")]
pub fn make_virtual_tool_check() -> TokenStream {
    quote! {
        if ::godot::private::is_class_inactive(Self::__config().is_tool) {
            return None;
        }
    }
}

#[cfg(since_api = "4.3")]
pub fn make_virtual_tool_check() -> TokenStream {
    TokenStream::new()
}

// This function is duplicated in godot-codegen\src\util.rs
#[rustfmt::skip]
pub fn safe_ident(s: &str) -> Ident {
    // See also: https://doc.rust-lang.org/reference/keywords.html
    match s {
        // Lexer
        | "as" | "break" | "const" | "continue" | "crate" | "else" | "enum" | "extern" | "false" | "fn" | "for" | "if"
        | "impl" | "in" | "let" | "loop" | "match" | "mod" | "move" | "mut" | "pub" | "ref" | "return" | "self" | "Self"
        | "static" | "struct" | "super" | "trait" | "true" | "type" | "unsafe" | "use" | "where" | "while"

        // Lexer 2018+
        | "async" | "await" | "dyn"

        // Reserved
        | "abstract" | "become" | "box" | "do" | "final" | "macro" | "override" | "priv" | "typeof" | "unsized" | "virtual" | "yield"

        // Reserved 2018+
        | "try"
           => format_ident!("{}_", s),

         _ => ident(s)
    }
}
