// Note: some code duplication with codegen crate

use crate::ParseResult;
use proc_macro2::{Ident, Literal, Span, TokenTree};
use quote::spanned::Spanned;
use quote::{format_ident, quote};
use std::collections::HashMap;
use venial::{Error, Function};

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
    reduced.attributes.clear();
    reduced.tk_semicolon = None;

    reduced
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Key-value parsing of proc attributes

#[derive(Clone, Eq, PartialEq, Debug)]
pub(crate) enum KvValue {
    None,
    Str(String),
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

    // can't be a closure because closures borrow greedy, and we'd need borrowing only at invocation time (lazy)
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
                    insert_kv!(KvValue::Ident(ident.clone()));
                    state = KvState::Value;
                }
                // key = "value" ...
                TokenTree::Literal(lit) => {
                    insert_kv!(KvValue::Str(lit.to_string()));
                    state = KvState::Value;
                } // TODO non-string literals
                _ => bail("'=' sign must be followed by a value", tk)?,
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
                    state = KvState::Key;
                }
                _ => bail("',' must be followed by the next key", tk)?,
            },
        }

        //println!("   {state:?} -> {tk:?}");
    }

    // No more tokens, make sure it ends in a valid state
    match state {
        KvState::Start | KvState::Key | KvState::Value | KvState::Comma => {}
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

// ----------------------------------------------------------------------------------------------------------------------------------------------

#[test]
fn test_parse_kv_none() {
    let input = quote! {
        #[attr(key=value)]
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

    let parsed = parse_kv_group(attr_value).expect("parse");

    dbg!(&parsed);

    assert_eq!(
        parsed.get("key"),
        Some(&KvValue::Ident(Ident::new("value", Span::call_site())))
    );
}
