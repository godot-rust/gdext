use crate::util::ident;
use proc_macro2::{Ident, Punct, TokenStream, TokenTree};
use quote::spanned::Spanned;
use quote::{quote, ToTokens, TokenStreamExt};
use std::collections::HashMap;
use std::mem;
use venial::{AttributeValue, Error, NamedField, StructFields, TyExpr};

struct ExportedField {
    name: Ident,
    ty: TyExpr,
}
impl ExportedField {
    fn new(field: &NamedField) -> Self {
        Self {
            name: field.name.clone(),
            ty: field.ty.clone(),
        }
    }
}

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

    let fields: Vec<(NamedField, Punct)> = match &class.fields {
        StructFields::Unit => {
            vec![]
        }
        StructFields::Tuple(_) => bail(
            "#[derive(GodotClass)] not supported for tuple structs",
            &class.fields,
        )?,
        StructFields::Named(fields) => fields.fields.inner.clone(),
    };

    let mut all_field_names = vec![];
    let mut exported_fields = vec![];
    let mut base_field = Option::<ExportedField>::None;

    for (mut field, _punct) in fields {
        for mut attr in field.attributes.iter() {
            if let Some(path) = attr.get_single_path_segment() {
                let mut is_base = false;
                if path.to_string() == "base" {
                    is_base = true;
                    if let Some(prev_base) = base_field {
                        bail(
                            &format!(
                                "#[base] allowed for at most 1 field, already applied to '{}'",
                                prev_base.name
                            ),
                            attr,
                        )?;
                    }
                    base_field = Some(ExportedField::new(&field))
                } else if path.to_string() == "export" {
                    exported_fields.push(ExportedField::new(&field))
                }

                if !is_base {
                    all_field_names.push(field.name.clone())
                }
            }
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
    let default = create_default(class_name, base_field, all_field_names);
    //let fields = class.field_tokens().to_token_stream();

    let result = quote! {
        impl gdext_class::traits::GodotClass for #class_name {
            type Base = gdext_class::api::#base;
            type Declarer = gdext_class::marker::UserClass;
            type Mem = Self::Base::Mem;

            fn class_name() -> String {
                #class_name_str.to_string()
            }
        }
        #default
        // impl GodotExtensionClass for #class_name {
        //     fn virtual_call(_name: &str) -> sys::GDNativeExtensionClassCallVirtual {
        //         todo!()
        //     }
        //     fn register_methods() {}
        // }

    };

    Ok(result)
}

fn create_default(
    class_name: &Ident,
    base_field: Option<ExportedField>,
    all_field_names: Vec<Ident>,
) -> TokenStream {
    let base_init = if let Some(ExportedField { name, .. }) = base_field {
        quote! { #name: base, }
    } else {
        TokenStream::new()
    };

    let rest_init = all_field_names.into_iter().map(|field| {
        quote! { #field: std::default::Default::default(), }
    });

    quote! {
        impl gdext_class::traits::GodotDefault for #class_name {
            fn construct(base: gdext_class::Obj<Self::Base>) -> Self {
                Self {
                    #( #rest_init )*
                    #base_init
                }
            }
        }
    }
}

#[derive(Clone, Eq, PartialEq, Debug)]
enum KvValue {
    None,
    Str(String),
    Ident(Ident),
}

// parses (a="hey", b=342)
fn parse_kv_group(value: &AttributeValue) -> Result<HashMap<String, KvValue>, Error> {
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

    let mut map: HashMap<String, KvValue> = HashMap::new();
    let mut state = KvState::Start;
    let mut last_key: Option<String> = None;

    // can't be a closure because closures borrow greedy, and we'd need borrowing only at invocation time (lazy)
    macro_rules! insert_kv {
        ($value:expr) => {
            let key = last_key.take().unwrap();
            map.insert(key, $value);
        };
    }

    let tokens = value.get_value_tokens();
    println!("all tokens: {tokens:?}");
    for tk in tokens {
        // Key
        println!("-- {state:?} -> {tk:?}");

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

        println!("   {state:?} -> {tk:?}");
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

fn bail<R, T>(msg: &str, tokens: T) -> Result<R, Error>
where
    T: Spanned,
{
    Err(Error::new_at_span(tokens.__span(), msg))
}
