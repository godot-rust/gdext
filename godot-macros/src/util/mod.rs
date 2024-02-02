/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

// Note: some code duplication with codegen crate

use crate::ParseResult;
use proc_macro2::{Delimiter, Group, Ident, Literal, TokenStream, TokenTree};
use quote::spanned::Spanned;
use quote::{format_ident, quote, ToTokens, TokenStreamExt};
use venial::{Error, Function, GenericParamList, Impl, TyExpr, WhereClause};

mod kv_parser;
mod list_parser;

pub(crate) use kv_parser::has_attr;
pub(crate) use kv_parser::KvParser;
pub(crate) use list_parser::ListParser;

pub fn ident(s: &str) -> Ident {
    format_ident!("{}", s)
}

pub fn cstr_u8_slice(string: &str) -> Literal {
    Literal::byte_string(format!("{string}\0").as_bytes())
}

pub fn class_name_obj(class: &impl ToTokens) -> TokenStream {
    let class = class.to_token_stream();
    quote! { <#class as ::godot::obj::GodotClass>::class_name() }
}

pub fn property_variant_type(property_type: &impl ToTokens) -> TokenStream {
    let property_type = property_type.to_token_stream();
    quote! { <<<#property_type as ::godot::register::property::Var>::Intermediate as ::godot::builtin::meta::GodotConvert>::Via as ::godot::builtin::meta::GodotType>::Ffi::variant_type() }
}

pub fn property_variant_class_name(property_type: &impl ToTokens) -> TokenStream {
    let property_type = property_type.to_token_stream();
    quote! { <<<#property_type as ::godot::register::property::Var>::Intermediate as ::godot::builtin::meta::GodotConvert>::Via as ::godot::builtin::meta::GodotType>::class_name() }
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

pub(crate) use bail;

pub fn error_fn<T>(msg: impl AsRef<str>, tokens: T) -> Error
where
    T: Spanned,
{
    Error::new_at_span(tokens.__span(), msg.as_ref())
}

macro_rules! error {
    ($tokens:expr, $format_string:literal $($rest:tt)*) => {
        $crate::util::error_fn(format!($format_string $($rest)*), $tokens)
    }
}

pub(crate) use error;

pub fn reduce_to_signature(function: &Function) -> Function {
    let mut reduced = function.clone();
    reduced.vis_marker = None; // TODO needed?
    reduced.attributes.clear();
    reduced.tk_semicolon = None;
    reduced.body = None;

    reduced
}

pub fn parse_signature(mut signature: TokenStream) -> Function {
    // Signature needs {} body to be parseable by venial
    signature.append(TokenTree::Group(Group::new(
        Delimiter::Brace,
        TokenStream::new(),
    )));

    let method_declaration = venial::parse_declaration(signature)
        .unwrap()
        .as_function()
        .unwrap()
        .clone();

    reduce_to_signature(&method_declaration)
}

/// Returns a type expression that can be used as a `VarcallSignatureTuple`.
pub fn make_signature_tuple_type(
    class_name: &Ident,
    ret_type: &TokenStream,
    param_types: &[TyExpr],
) -> TokenStream {
    let param_types: Vec<TyExpr> = param_types
        .iter()
        .map(|ty| {
            let tokens = ty
                .tokens
                .iter()
                .flat_map(|tt| tt.into_token_stream())
                .map(|tt| map_self_to_class_name(tt, class_name))
                .collect();
            TyExpr { tokens }
        })
        .collect();

    let ret_type: TokenStream = ret_type
        .clone()
        .into_iter()
        .map(|tt| map_self_to_class_name(tt, class_name))
        .collect();

    quote::quote! {
        (#ret_type, #(#param_types),*)
    }
}

/// Maps each usage of `Self` to the struct it's referencing,
/// since `Self` can't be used inside nested functions.
fn map_self_to_class_name(tokens: TokenTree, class_name: &Ident) -> TokenTree {
    match tokens {
        TokenTree::Group(group) => {
            let stream: proc_macro2::TokenStream = group
                .stream()
                .into_iter()
                .map(|tt| map_self_to_class_name(tt, class_name))
                .collect();
            TokenTree::Group(Group::new(group.delimiter(), stream))
        }
        TokenTree::Ident(ident) if ident == "Self" => TokenTree::Ident(class_name.clone()),
        tt => tt,
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
pub(crate) fn is_impl_named(original_impl: &Impl, name: &str) -> bool {
    let trait_name = original_impl.trait_ty.as_ref().unwrap(); // unwrap: already checked outside
    extract_typename(trait_name).map_or(false, |seg| seg.ident == name)
}

/// Validates either:
/// a) the declaration is `impl Trait for SomeType`, if `expected_trait` is `Some("Trait")`
/// b) the declaration is `impl SomeType`, if `expected_trait` is `None`
pub(crate) fn validate_impl(
    original_impl: &Impl,
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

/// Validates that the declaration is the of the form `impl Trait for
/// SomeType`, where the name of `Trait` ends in `Virtual`.
pub(crate) fn validate_trait_impl_virtual(
    original_impl: &Impl,
    attr: &str,
) -> ParseResult<(Ident, Ident)> {
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
        let trait_name = typename.unwrap(); // unwrap: already checked in 'Validate trait'
        (class_name, trait_name.ident)
    })
}

fn validate_self(original_impl: &Impl, attr: &str) -> ParseResult<Ident> {
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

/// Gets the right-most type name in the path
fn extract_typename(ty: &venial::TyExpr) -> Option<venial::PathSegment> {
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
    // Could also use TyExpr::as_path(), or fn below this one.
    path.last()
        .map(|last| last.to_string() == expected)
        .unwrap_or(false)
}

pub(crate) fn path_ends_with_complex(path: &venial::TyExpr, expected: &str) -> bool {
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

pub(crate) struct DeclInfo {
    pub where_: Option<WhereClause>,
    pub generic_params: Option<GenericParamList>,
    pub name: Ident,
    pub name_string: String,
}

pub(crate) fn decl_get_info(decl: &venial::Declaration) -> DeclInfo {
    let (where_, generic_params, name, name_string) = match decl {
        venial::Declaration::Struct(struct_) => (
            struct_.where_clause.clone(),
            struct_.generic_params.clone(),
            struct_.name.clone(),
            struct_.name.to_string(),
        ),
        venial::Declaration::Enum(enum_) => (
            enum_.where_clause.clone(),
            enum_.generic_params.clone(),
            enum_.name.clone(),
            enum_.name.to_string(),
        ),
        _ => {
            panic!("only enums and structs are supported at the moment")
        }
    };

    DeclInfo {
        where_,
        generic_params,
        name,
        name_string,
    }
}

pub fn make_virtual_tool_check() -> TokenStream {
    quote! {
        if ::godot::private::is_class_inactive(Self::__config().is_tool) {
            return None;
        }
    }
}
