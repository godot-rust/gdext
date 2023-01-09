mod all;
mod any;
mod error;
mod list;
mod not;
mod option;
mod predicate;

use crate::ParseResult;
use all::*;
use any::*;
use error::*;
use list::*;
use not::*;
use option::*;
use predicate::*;
use proc_macro2::{Delimiter, Ident, Punct, Spacing, Span, TokenStream, TokenTree};
use quote::ToTokens;
use std::str::FromStr;
use venial::{Attribute, AttributeValue, Declaration, GroupSpan, ImplMember, StructFields};

trait GodotConditionalCompilation {
    fn should_compile(&self) -> bool;
}

pub fn should_compile(ts: proc_macro2::TokenStream) -> Result<bool, venial::Error> {
    let predicate = GodotConfigurationPredicate::try_from(ts)?;
    Ok(predicate.should_compile())
}

pub fn transform(meta: TokenStream, input: TokenStream) -> ParseResult<TokenStream> {
    let mut decl = venial::parse_declaration(input)?;
    match &decl {
        Declaration::Struct(_) => {
            decorate_block_attributes(&meta, &mut decl);
            decorate_field_attributes(&meta, &mut decl);
        }
        Declaration::Impl(impl_decl) => {
            // Dump GodotExt all together as this can not be implemented without the derive, luckily
            // its unlikely you will want this when turning off godot-rust/gdextension
            if let Some(path) = impl_decl.trait_ty.as_ref() {
                if &path.to_token_stream().to_string() == "GodotExt" {
                    return Ok(TokenStream::new());
                }
            }
            decorate_block_attributes(&meta, &mut decl);
            decorate_methods(&meta, &mut decl);
        }
        _ => {}
    }
    Ok(decl.to_token_stream())
}

/// Creates a new `#[cfg(...)]` attribute that with the provided contents from `#[godot_attr(...)]`
fn create_cfg(cfg_inner: &TokenStream) -> Attribute {
    let tk_hash = Punct::new('#', Spacing::Alone);

    let tk_brackets = GroupSpan {
        delimiter: Delimiter::Parenthesis,
        span: Span::call_site(),
    };

    let path = vec![TokenTree::Ident(Ident::new("cfg", Span::call_site()))];

    let value = AttributeValue::Group(
        GroupSpan {
            delimiter: Delimiter::Parenthesis,
            span: Span::call_site(),
        },
        cfg_inner.clone().into_iter().collect(),
    );

    Attribute {
        tk_hash,
        tk_bang: None,
        tk_brackets,
        path,
        value,
    }
}

/// Produces a `#[cfg_attr(..., ...)]` attribute that with the provided contents from
/// `#[godot_attr(...)]` for whatever attribute is provided
fn create_cfg_attr(cfg_inner: &TokenStream, old_attr: &Attribute) -> Attribute {
    let mut tk_hash = Punct::new('#', Spacing::Alone);
    tk_hash.set_span(old_attr.tk_hash.span());

    let tk_brackets = GroupSpan {
        delimiter: Delimiter::Bracket,
        span: Span::call_site(),
    };

    let path = vec![TokenTree::Ident(Ident::new(
        "cfg_attr",
        // old_attr.get_single_path_segment().unwrap().span(),
        Span::call_site(),
    ))];

    // ToDo: Make the value creation not awful
    let new_attribute_values = format!(
        "{}, {}",
        cfg_inner.to_string(),
        old_attr.path.iter().map(|path| path.to_string()).collect::<Vec<_>>().join(", ")
    );
    let ts = TokenStream::from_str(&new_attribute_values).unwrap();
    let tokens: Vec<_> = ts.into_iter().collect();
    let value = AttributeValue::Group(
        GroupSpan {
            delimiter: Delimiter::Parenthesis,
            span: Span::call_site(),
        },
        tokens,
    );

    let attribute = Attribute {
        tk_hash,
        tk_bang: None,
        tk_brackets,
        path,
        value,
    };
    dbg!(&attribute.to_token_stream().to_string());
    attribute
}

fn decorate_attributes(
    cfg_inner: &TokenStream,
    attrs: &mut [Attribute],
) {
    const GODOT_CLASS_ATTRS: [&str; 7] =
        ["class", "property", "export", "base", "signal", "godot_api", "func"];
    for attr in attrs {
        let Some(path) = attr.get_single_path_segment().map(|path| path.to_string()) else {
            continue;
        };
        if GODOT_CLASS_ATTRS.contains(&path.as_str()) {
            let new_attr = create_cfg_attr(cfg_inner, &attr);
            let _old_attr = std::mem::replace(attr, new_attr);
        }
    }
}

/// This function replaces gdextension attributes on structs and impls with a cfg_attr wrapper
/// For example,
///
/// ```norun rust
/// #[godot_cfg(not(test))]
/// #[derive(GodotClass, Debug))]
/// #[class(base = Node))]
/// pub struct MyGodotClass {
///     ...
/// }
/// ```
/// desugars to:
/// ```norun rust
/// #[derive(GodotClass, Debug))]
/// #[cfg_attr(not(test), class(base = Node))]
/// pub struct MyGodotClass {
///     ...
/// }
/// ```
/// ToDo: prevent GodotClass from being derived
fn decorate_block_attributes(cfg_inner: &TokenStream, decl: &mut Declaration) {
    decorate_attributes(cfg_inner, decl.attributes_mut());
    //strip GodotClass
    if let Some(derive) = decl.attributes_mut().iter_mut().find(|attr| attr.get_single_path_segment().map(|path| path == "derive").unwrap_or(false)) {
        if let AttributeValue::Group(_span, tokens) = &mut derive.value {
            tokens.retain(|token| &token.to_string() != "GodotClass");
        }
    }
}

/// This function replaces gdextension attributes on fields with a cfg_attr wrapper
/// For example,
///
/// ```norun rust
/// #[godot_cfg(not(test))]
/// pub struct MyGodotClass {
///     #[base]
///     base: Base<Node>,
/// }
/// ```
/// desugars to:
/// ```norun rust
/// pub struct MyGodotClass {
///     #[cfg_attr(not(test), base]
///     base: Base<Node>,
/// }
/// ```
fn decorate_field_attributes(cfg_inner: &TokenStream, decl: &mut Declaration) {
    let Declaration::Struct(struct_decl) = decl else {
        return;
    };

    let StructFields::Named(fields) = &mut struct_decl.fields else {
        return;
    };

    for (ref mut field, _) in &mut fields.fields.inner {
        decorate_attributes(cfg_inner, &mut field.attributes)
    }
}

/// This function replaces gdextension attributes on fields with a cfg_attr wrapper
/// For example,
///
/// ```norun rust
/// #[godot_cfg(not(test))]
/// pub struct MyGodotClass {
///     #[base]
///     base: Base<Node>,
/// }
/// ```
/// desugars to:
/// ```norun rust
/// pub struct MyGodotClass {
///     #[cfg_attr(not(test), base]
///     base: Base<Node>,
/// }
/// ```
fn decorate_methods(cfg_inner: &TokenStream, decl: &mut Declaration) {
    // ToDo: Signals are not provided as valid Rust so should be completely removed.
    const GODOT_CLASS_FIELDS: [&str; 1] = ["signal"];

    let Declaration::Impl(impl_decl) = decl else {
        return;
    };

    for member in impl_decl.body_items.iter_mut() {
        let ImplMember::Method(method) = member else {
            continue;
        };

        decorate_attributes(cfg_inner, &mut method.attributes);
        // Remove fields that can not be compiled
        method.attributes.retain(|attr| {
            attr.get_single_path_segment()
                .map(|path| !GODOT_CLASS_FIELDS.contains(&path.to_string().as_str()))
                .unwrap_or(true)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proc_macro2::TokenStream;
    use std::str::FromStr;

    #[test]
    fn test_should_compile() {
        let ts = TokenStream::from_str("any(all(test, not(doctest)), doctest)").unwrap();
        assert!(should_compile(ts).unwrap());
    }
}
