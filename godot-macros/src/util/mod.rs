/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

// Note: some code duplication with godot-codegen crate.

use proc_macro2::{Delimiter, Group, Ident, Literal, Punct, Spacing, Span, TokenStream, TokenTree};
use quote::spanned::Spanned;
use quote::{format_ident, quote, ToTokens, TokenStreamExt};

use crate::class::FuncDefinition;
use crate::ParseResult;

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

/// Returns the span of the given tokens.
pub fn span_of<T: Spanned>(tokens: &T) -> Span {
    // Use of private API due to lack of alternative. If this becomes an issue, we'll find another way.
    tokens.__span()
}

pub fn error_fn<T: Spanned>(msg: impl AsRef<str>, tokens: T) -> venial::Error {
    let span = span_of(&tokens);
    venial::Error::new_at_span(span, msg.as_ref())
}

macro_rules! error {
    ($tokens:expr, $format_string:literal $($rest:tt)*) => {
        $crate::util::error_fn(format!($format_string $($rest)*), $tokens)
    }
}

pub(crate) use {bail, error, require_api_version};

/// Keeps all attributes except the one specified (e.g. `"itest"`).
pub fn retain_attributes_except<'a>(
    attributes: &'a [venial::Attribute],
    macro_name: &'a str,
) -> impl Iterator<Item = &'a venial::Attribute> {
    attributes.iter().filter(move |attr| {
        attr.get_single_path_segment()
            .is_none_or(|segment| segment != macro_name)
    })
}

pub fn reduce_to_signature(function: &venial::Function) -> venial::Function {
    let mut reduced = function.clone();
    reduced.vis_marker = None; // retained outside in the case of #[signal].
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

/// Returns a type expression that can be used as a `ParamTuple`.
pub fn make_signature_param_type(param_types: &[venial::TypeExpr]) -> TokenStream {
    quote::quote! {
        (#(#param_types,)*)
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
    extract_typename(trait_name).is_some_and(|seg| seg.ident == name)
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

/// Validates that the declaration is the of the form `impl Trait for SomeType`, where the name of `Trait` begins with `I`.
///
/// Returns `(class_name, trait_path, trait_base_class)`, e.g. `(MyClass, godot::prelude::INode3D, Node3D)`.
pub(crate) fn validate_trait_impl_virtual(
    original_impl: &venial::Impl,
    attr: &str,
) -> ParseResult<(Ident, venial::TypeExpr, Ident)> {
    let trait_name = original_impl.trait_ty.as_ref().unwrap(); // unwrap: already checked outside
    let typename = extract_typename(trait_name);

    // Validate trait
    let Some(base_class) = typename
        .as_ref()
        .and_then(|seg| seg.ident.to_string().strip_prefix('I').map(ident))
    else {
        return bail!(
            original_impl,
            "#[{attr}] for trait impls requires a virtual method trait (trait name should start with 'I')",
        );
    };

    // Validate self
    validate_self(original_impl, attr).map(|class_name| {
        // let trait_name = typename.unwrap(); // unwrap: already checked in 'Validate trait'
        (class_name, trait_name.clone(), base_class)
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
pub(crate) fn extract_typename(ty: &venial::TypeExpr) -> Option<venial::PathSegment> {
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
    path.last().is_some_and(|last| last.to_string() == expected)
}

pub(crate) fn path_ends_with_complex(path: &venial::TypeExpr, expected: &str) -> bool {
    path.as_path().is_some_and(|path| {
        path.segments
            .last()
            .is_some_and(|seg| seg.ident == expected)
    })
}

pub(crate) fn extract_cfg_attrs(
    attrs: &[venial::Attribute],
) -> impl IntoIterator<Item = &venial::Attribute> {
    attrs.iter().filter(|attr| {
        let Some(attr_name) = attr.get_single_path_segment() else {
            return false;
        };

        // #[cfg(condition)]
        if attr_name == "cfg" {
            return true;
        }

        // #[cfg_attr(condition, attributes...)]. Multiple attributes can be seperated by comma.
        if attr_name == "cfg_attr" && attr.value.to_token_stream().to_string().contains("cfg(") {
            return true;
        }

        false
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

// ----------------------------------------------------------------------------------------------------------------------------------------------

/// Parses a `meta` TokenStream, that is, the tokens in parameter position of a proc-macro (between the braces).
/// Because venial can't actually parse a meta item directly, this is done by reconstructing the full macro attribute on top of some content and then parsing *that*.
pub fn venial_parse_meta(
    meta: &TokenStream,
    self_name: Ident,
    content: &TokenStream,
) -> Result<venial::Item, venial::Error> {
    // Hack because venial doesn't support direct meta parsing yet
    let input = quote! {
        #[#self_name(#meta)]
        #content
    };

    venial::parse_item(input)
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

// util functions for handling #[func]s and #[var(get=f, set=f)]

pub fn make_funcs_collection_constants(
    funcs: &[FuncDefinition],
    class_name: &Ident,
) -> Vec<TokenStream> {
    funcs
        .iter()
        .map(|func| {
            // The constant needs the same #[cfg] attribute(s) as the function, so that it is only active if the function is also active.
            let cfg_attributes = extract_cfg_attrs(&func.external_attributes)
                .into_iter()
                .collect::<Vec<_>>();

            make_funcs_collection_constant(
                class_name,
                &func.signature_info.method_name,
                func.registered_name.as_ref(),
                &cfg_attributes,
            )
        })
        .collect()
}

/// Returns a `const` declaration for the funcs collection struct.
///
/// User-defined functions can be renamed with `#[func(rename=new_name)]`. To be able to access the renamed function name from another macro,
/// a constant is used as indirection.
pub fn make_funcs_collection_constant(
    class_name: &Ident,
    func_name: &Ident,
    registered_name: Option<&String>,
    attributes: &[&venial::Attribute],
) -> TokenStream {
    let const_name = format_funcs_collection_constant(class_name, func_name);
    let const_value = match &registered_name {
        Some(renamed) => renamed.to_string(),
        None => func_name.to_string(),
    };

    quote! {
        #(#attributes)*
        #[doc(hidden)]
        #[allow(non_upper_case_globals)]
        pub const #const_name: &str  = #const_value;
    }
}

/// Converts `path::class` to `path::new_class`.
pub fn replace_class_in_path(path: venial::Path, new_class: Ident) -> venial::Path {
    match path.segments.as_slice() {
        // Can't happen, you have at least one segment (the class name).
        [] => unreachable!("empty path"),

        [_single] => venial::Path {
            segments: vec![venial::PathSegment {
                ident: new_class,
                generic_args: None,
                tk_separator_colons: None,
            }],
        },

        [path @ .., _last] => {
            let mut segments = vec![];
            segments.extend(path.iter().cloned());
            segments.push(venial::PathSegment {
                ident: new_class,
                generic_args: None,
                tk_separator_colons: Some([
                    Punct::new(':', Spacing::Joint),
                    Punct::new(':', Spacing::Alone),
                ]),
            });
            venial::Path { segments }
        }
    }
}

/// Returns the name of the constant inside the func "collection" struct.
pub fn format_funcs_collection_constant(_class_name: &Ident, func_name: &Ident) -> Ident {
    format_ident!("{func_name}")
}

/// Returns the name of the struct used as collection for all function name constants.
pub fn format_funcs_collection_struct(class_name: &Ident) -> Ident {
    format_ident!("__godot_{class_name}_Funcs")
}

/// Returns the name of the macro used to communicate the `struct` (class) visibility to other symbols.
pub fn format_class_visibility_macro(class_name: &Ident) -> Ident {
    format_ident!("__godot_{class_name}_vis_macro")
}

/// Returns the name of the macro used to communicate whether the `struct` (class) contains a base field.
pub fn format_class_base_field_macro(class_name: &Ident) -> Ident {
    format_ident!("__godot_{class_name}_has_base_field_macro")
}

/// Returns the name of the macro used to deny manual `init()` for incompatible init strategies.
pub fn format_class_deny_manual_init_macro(class_name: &Ident) -> Ident {
    format_ident!("__deny_manual_init_{class_name}")
}
