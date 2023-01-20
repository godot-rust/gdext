/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

// Note: some code duplication with codegen crate

use crate::ParseResult;
use proc_macro2::{Ident, Literal, Span, TokenTree};
use quote::format_ident;
use quote::spanned::Spanned;
use std::collections::HashMap;
use venial::{Error, Function, Impl};

pub fn ident(s: &str) -> Ident {
    format_ident!("{}", s)
}

#[allow(dead_code)]
pub fn strlit(s: &str) -> Literal {
    Literal::string(s)
}

pub fn bail<R, T>(msg: impl AsRef<str>, tokens: T) -> Result<R, Error>
where
    T: Spanned,
{
    Err(Error::new_at_span(tokens.__span(), msg.as_ref()))
}

pub fn reduce_to_signature(function: &Function) -> Function {
    let mut reduced = function.clone();
    reduced.vis_marker = None; // TODO needed?
    reduced.attributes.clear();
    reduced.tk_semicolon = None;
    reduced.body = None;

    reduced
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Key-value parsing of proc attributes

#[derive(Clone, Eq, PartialEq, Debug)]
pub(crate) enum KvValue {
    /// Key only, no value.
    None,

    /// Literal like `"hello"`, `20`, `3.4`.
    /// Unlike the proc macro type, this includes `true` and `false` as well as negative literals `-32`.
    /// Complex expressions are not supported though.
    Lit(String),

    /// Identifier like `hello`.
    Ident(Ident),
}

pub(crate) type KvMap = HashMap<String, KvValue>;

// parses (a="hey", b=342)
pub(crate) fn parse_kv_group(value: &venial::AttributeValue) -> ParseResult<KvMap> {
    // FSM with possible flows:
    //
    //  [start]* ------>  Key*  ----> Equals
    //                    ^  |          |
    //                    |  v          v
    //                   Comma* <----- Value*
    //  [end] <-- *
    #[derive(Copy, Clone, Eq, PartialEq, Debug)]
    enum KvState {
        Start,
        Key,
        Equals,
        Value,
        Comma,
    }

    let mut map: KvMap = HashMap::new();
    let mut state = KvState::Start;
    let mut last_key: Option<String> = None;
    let mut is_negative: bool = false;

    // can't be a closure because closures borrow greedily, and we'd need borrowing only at invocation time (lazy)
    macro_rules! insert_kv {
        ($value:expr) => {
            let key = last_key.take().expect("last_key.take");
            map.insert(key, $value);
        };
    }

    let tokens = value.get_value_tokens();
    //println!("all tokens: {tokens:?}");
    for tk in tokens {
        // Key
        //println!("-- {state:?} -> {tk:?}");

        match state {
            KvState::Start => match tk {
                // key ...
                TokenTree::Ident(ident) => {
                    let key = last_key.replace(ident.to_string());
                    assert!(key.is_none());
                    state = KvState::Key;
                }
                _ => bail("attribute must start with key", tk)?,
            },
            KvState::Key => {
                match tk {
                    TokenTree::Punct(punct) => {
                        if punct.as_char() == '=' {
                            // key = ...
                            state = KvState::Equals;
                        } else if punct.as_char() == ',' {
                            // key, ...
                            insert_kv!(KvValue::None);
                            state = KvState::Comma;
                        } else {
                            bail("key must be followed by either '=' or ','", tk)?;
                        }
                    }
                    _ => {
                        bail("key must be followed by either '=' or ','", tk)?;
                    }
                }
            }
            KvState::Equals => match tk {
                // key = value ...
                TokenTree::Ident(ident) => {
                    let ident_str = ident.to_string();
                    if ident_str == "true" || ident_str == "false" {
                        insert_kv!(KvValue::Lit(ident_str));
                    } else {
                        insert_kv!(KvValue::Ident(ident.clone()));
                    }
                    state = KvState::Value;
                }
                // key = "value" ...
                TokenTree::Literal(lit) => {
                    let prefix = if is_negative { "-" } else { "" };
                    insert_kv!(KvValue::Lit(format!("{prefix}{lit}")));
                    state = KvState::Value;
                }
                // key = - ...
                TokenTree::Punct(punct) if punct.as_char() == '-' => {
                    is_negative = true;
                    // state remains
                }
                _ => bail("'=' sign must be followed by an identifier or literal", tk)?,
            },
            KvState::Value => match tk {
                // key = value, ...
                TokenTree::Punct(punct) => {
                    if punct.as_char() == ',' {
                        state = KvState::Comma;
                    } else {
                        bail("value must be followed by a ','", tk)?;
                    }
                }
                _ => bail("value must be followed by a ','", tk)?,
            },
            KvState::Comma => match tk {
                // , key ...
                TokenTree::Ident(ident) => {
                    let key = last_key.replace(ident.to_string());
                    assert!(key.is_none());
                    is_negative = false;
                    state = KvState::Key;
                }
                _ => bail("',' must be followed by the next key", tk)?,
            },
        }

        //println!("   {state:?} -> {tk:?}");
    }

    // No more tokens, make sure it ends in a valid state
    match state {
        KvState::Key => {
            // Only stored key, not yet added to map
            insert_kv!(KvValue::None);
        }
        KvState::Start | KvState::Value | KvState::Comma => {}
        KvState::Equals => {
            bail("unexpected end of macro attributes", value)?;
        }
    }

    Ok(map)
}

/// At the end of processing a KV map, make sure it runs
/// TODO refactor to a wrapper class and maybe destructor
pub(crate) fn ensure_kv_empty(map: KvMap, span: Span) -> ParseResult<()> {
    if map.is_empty() {
        Ok(())
    } else {
        let msg = &format!("Attribute contains unknown keys: {:?}", map.keys());
        bail(msg, span)
    }
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
        let trait_name = original_impl.trait_ty.as_ref().unwrap(); // unwrap: already checked outside
        if !extract_typename(trait_name).map_or(false, |seg| seg.ident == expected_trait) {
            return bail(
                format!("#[{attr}] for trait impls requires trait to be `{expected_trait}`"),
                original_impl,
            );
        }
    }

    // impl Trait for Self -- validate Self
    if let Some(segment) = extract_typename(&original_impl.self_ty) {
        if segment.generic_args.is_none() {
            Ok(segment.ident)
        } else {
            bail(
                format!("#[{attr}] for does currently not support generic arguments"),
                original_impl,
            )
        }
    } else {
        bail(
            format!("#[{attr}] requires Self type to be a simple path"),
            original_impl,
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

#[cfg(test)]
mod tests {
    use super::*;
    use proc_macro2::TokenStream;
    use quote::quote;

    macro_rules! hash_map {
        (
            $($key:expr => $value:expr),*
            $(,)?
        ) => {
            {
                let mut map = std::collections::HashMap::new();
                $(
                    map.insert($key, $value);
                )*
                map
            }
        };
    }

    fn expect_parsed(input_tokens: TokenStream, output_map: KvMap) {
        let input = quote! {
            #input_tokens
            fn func();
        };
        let decl = venial::parse_declaration(input);

        let attrs = &decl
            .as_ref()
            .expect("decl")
            .as_function()
            .expect("fn")
            .attributes;

        assert_eq!(attrs.len(), 1);
        let attr_value = &attrs[0].value;
        let mut parsed = parse_kv_group(attr_value).expect("parse");

        for (key, value) in output_map {
            assert_eq!(parsed.remove(&key), Some(value));
        }

        assert!(parsed.is_empty(), "Remaining entries in map");
    }

    #[test]
    fn test_parse_kv_just_key() {
        expect_parsed(
            quote! {
                #[attr(just_key)]
            },
            hash_map!(
                "just_key".to_string() => KvValue::None,
            ),
        );
    }

    #[test]
    fn test_parse_kv_key_ident() {
        expect_parsed(
            quote! {
                #[attr(key=value)]
            },
            hash_map!(
                "key".to_string() => KvValue::Ident(ident("value")),
            ),
        );
    }

    #[test]
    fn test_parse_kv_key_lit() {
        expect_parsed(
            quote! {
                #[attr(key="string", pos=32, neg=-32, bool=true, float=3.4)]
            },
            hash_map!(
                "key".to_string() => KvValue::Lit("\"string\"".to_string()),
                "pos".to_string() => KvValue::Lit("32".to_string()),
                "neg".to_string() => KvValue::Lit("-32".to_string()),
                "bool".to_string() => KvValue::Lit("true".to_string()),
                "float".to_string() => KvValue::Lit("3.4".to_string()),
            ),
        );
    }

    #[test]
    fn test_parse_kv_mixed() {
        expect_parsed(
            quote! {
                #[attr(forever, key="string", default=-820, fn=my_function, alone)]
            },
            hash_map!(
                "forever".to_string() => KvValue::None,
                "key".to_string() => KvValue::Lit("\"string\"".to_string()),
                "default".to_string() => KvValue::Lit("-820".to_string()),
                "fn".to_string() => KvValue::Ident(ident("my_function")),
                "alone".to_string() => KvValue::None,
            ),
        );
    }
}

pub(crate) fn path_is_single(path: &Vec<TokenTree>, expected: &str) -> bool {
    path.len() == 1 && path[0].to_string() == expected
}
