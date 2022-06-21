use crate::util::ident;
use proc_macro2::{Ident, TokenStream, TokenTree};
use quote::spanned::Spanned;
use quote::{quote, ToTokens, TokenStreamExt};
use std::collections::HashMap;
use venial::{AttributeValue, Error};

pub fn derive_godot_class(input: TokenStream) -> Result<TokenStream, Error> {
    let decl = venial::parse_declaration(input)?;

    let class = decl.as_struct().ok_or(Error::new("Not a valid struct"))?;

    let mut godot_attr = None;

    for attr in class.attributes.iter() {
        let path = &attr.path;
        if path.len() == 1 || path[0].to_string() == "godot" {
            if godot_attr.is_some() {
                bail("Only one #[godot] attribute per struct allowed", attr)?;
            }

            let map = parse_kv_group(&attr.value)?;
            godot_attr = Some((attr.__span(), map));
        }
    }

    let mut base = ident("RefCounted");
    if let Some((span, mut map)) = godot_attr {
        if let Some(kv_value) = map.remove("base") {
            if let KvValue::Ident(override_base) = kv_value {
                base = override_base;
            } else {
                bail("Invalid 'base' value", span)?;
            }
        }
    }

    let class_name = &class.name;
    let class_name_str = class.name.to_string();
    //let fields = class.field_tokens().to_token_stream();

    let result = quote! {
        impl gdext_class::traits::GodotClass for #class_name {
            type Base = gdext_class::api::#base;
            // type Declarer = marker::UserClass;
            // type Mem = mem::ManualMemory;

            fn class_name() -> String {
                #class_name_str.to_string()
            }
        }
        // impl GodotExtensionClass for #class_name {
        //     fn virtual_call(_name: &str) -> sys::GDNativeExtensionClassCallVirtual {
        //         todo!()
        //     }
        //     fn register_methods() {}
        // }
        // impl DefaultConstructible for ObjPayload {
        //     fn construct(_base: sys::GDNativeObjectPtr) -> Self {
        //         #class_name { }
        //     }
        // }
    };

    Ok(result)
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
enum KvToken {
    KeyOrEnd,
    Equals,
    Value,
    Comma,
}

#[derive(Clone, Eq, PartialEq, Debug)]
enum KvValue {
    None,
    Str(String),
    Ident(Ident),
}

// parses (a="hey", b=342)
fn parse_kv_group(value: &AttributeValue) -> Result<HashMap<String, KvValue>, Error> {
    let tokens = value.get_value_tokens();

    let mut map = HashMap::new();

    let mut expect_next = KvToken::KeyOrEnd;
    let mut last_key = None;

    // FSM with possible flows:
    //
    //  [start]  ------> KeyOrEnd ----> Equals
    //                 /  ^  |           |
    //                /   |  v           v
    //   [end]  <----´-- Comma <------ Value
    //
    println!("all tokens: {tokens:?}");
    for tk in tokens {
        // Key
        println!("-- {tk:?} @ {expect_next:?}");
        match tk {
            TokenTree::Group(group) => {
                unimplemented!("Only key=value syntax supported")
            }
            TokenTree::Ident(ident) => {
                if expect_next == KvToken::KeyOrEnd {
                    last_key = Some(ident);
                    expect_next = KvToken::Equals;
                } else if expect_next == KvToken::Value {
                    map.insert(last_key.unwrap().to_string(), KvValue::Ident(ident.clone()));
                    expect_next = KvToken::Comma;
                    last_key = None;
                }
            }
            TokenTree::Punct(punct) => {
                if expect_next == KvToken::Equals && punct.as_char() == '=' {
                    expect_next = KvToken::Value;
                } else if expect_next == KvToken::Comma && punct.as_char() == ',' {
                    expect_next = KvToken::KeyOrEnd;
                } else {
                    bail(
                        &format!(
                            "Unexpected punctuation token '{}' in macro attributes",
                            punct.as_char()
                        ),
                        punct,
                    )?;
                }
            }
            TokenTree::Literal(lit) => {
                if expect_next == KvToken::Value {
                    map.insert(last_key.unwrap().to_string(), KvValue::Str(lit.to_string()));
                    expect_next = KvToken::Comma;
                    last_key = None;
                } else {
                    bail("Unexpected literal in macro attributes", lit)?;
                }
            }
        }
        println!("   {tk:?} @ {expect_next:?}");
    }
    if expect_next != KvToken::KeyOrEnd && expect_next != KvToken::Comma {
        bail("Macro attributes ended unexpectedly", value)?;
    }

    Ok(map)
}

fn bail<R, T>(msg: &str, tokens: T) -> Result<R, Error>
where
    T: Spanned,
{
    Err(Error::new_at_span(tokens.__span(), msg))
}
