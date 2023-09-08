/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use proc_macro2::{Delimiter, Group, Ident, Punct, TokenStream, TokenTree};
use quote::{format_ident, quote};
use venial::{Declaration, NamedField, Struct, StructFields};

use crate::class::{make_property_impl, Field, FieldExport, FieldVar, Fields};
use crate::util::{bail, ident, KvParser};
use crate::{util, ParseResult};

pub fn derive_godot_class(decl: Declaration) -> ParseResult<TokenStream> {
    let class = decl
        .as_struct()
        .ok_or_else(|| venial::Error::new("Not a valid struct"))?;

    let struct_cfg = parse_struct_attributes(class)?;
    let mut fields = parse_fields(class)?;
    fields.all_fields.extend(struct_cfg.standlone_properties);

    let class_name = &class.name;
    let class_name_str = class.name.to_string();
    let class_name_cstr = util::cstr_u8_slice(&class_name_str);
    let class_name_obj = util::class_name_obj(class_name);

    let base_ty = &struct_cfg.base_ty;
    let base_class = quote! { ::godot::engine::#base_ty };
    let base_class_name_obj = util::class_name_obj(&base_class);
    let inherits_macro = format_ident!("inherits_transitive_{}", base_ty);

    let prv = quote! { ::godot::private };
    let godot_exports_impl = make_property_impl(class_name, &fields);

    let (godot_init_impl, create_fn);
    if struct_cfg.has_generated_init {
        godot_init_impl = make_godot_init_impl(class_name, fields);
        create_fn = quote! { Some(#prv::callbacks::create::<#class_name>) };
    } else {
        godot_init_impl = TokenStream::new();
        create_fn = quote! { None };
    };

    let config_impl = make_config_impl(class_name, struct_cfg.is_tool);

    Ok(quote! {
        unsafe impl ::godot::obj::GodotClass for #class_name {
            type Base = #base_class;
            type Declarer = ::godot::obj::dom::UserDomain;
            type Mem = <Self::Base as ::godot::obj::GodotClass>::Mem;

            fn class_name() -> ::godot::builtin::meta::ClassName {
                ::godot::builtin::meta::ClassName::from_ascii_cstr(#class_name_cstr)
            }
        }

        #godot_init_impl
        #godot_exports_impl
        #config_impl

        ::godot::sys::plugin_add!(__GODOT_PLUGIN_REGISTRY in #prv; #prv::ClassPlugin {
            class_name: #class_name_obj,
            component: #prv::PluginComponent::ClassDef {
                base_class_name: #base_class_name_obj,
                generated_create_fn: #create_fn,
                free_fn: #prv::callbacks::free::<#class_name>,
            },
        });

        #prv::class_macros::#inherits_macro!(#class_name);
    })
}

/// Checks at compile time that a function with the given name exists on `Self`.
#[must_use]
pub fn make_existence_check(ident: &Ident) -> TokenStream {
    quote! {
        #[allow(path_statements)]
        Self::#ident;
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Implementation

/// Returns the name of the base and the default mode
fn parse_struct_attributes(class: &Struct) -> ParseResult<ClassAttributes> {
    let mut base_ty = ident("RefCounted");
    let mut has_generated_init = false;
    let mut is_tool = false;
    let mut standlone_properties = vec![];

    // #[class] attribute on struct
    if let Some(mut parser) = KvParser::parse(&class.attributes, "class")? {
        if let Some(base) = parser.handle_ident("base")? {
            base_ty = base;
        }

        if parser.handle_alone("init")? {
            has_generated_init = true;
        }

        if parser.handle_alone("tool")? {
            is_tool = true;
        }

        parser.finish()?;
    }

    // #[property] attributes on struct
    for mut parser in KvParser::parse_many(&class.attributes, "property")? {
        let name = parser.handle_expr_required("name")?.to_string();
        let ty = parser.handle_expr_required("type")?;

        let field_var = FieldVar::new_from_kv(&mut parser)?;
        if field_var.getter.is_omitted() && field_var.setter.is_omitted() {
            bail!(
                parser.span(),
                "#[property] must define at least 1 getter or setter"
            )?;
        }
        if field_var.getter.is_generated() || field_var.setter.is_generated() {
            bail!(
                parser.span(),
                "#[property] does not support generated getters and setters"
            )?;
        }

        let mut field = Field::new(
            ident(name.as_str()),
            venial::TyExpr {
                tokens: vec![TokenTree::Group(Group::new(Delimiter::None, ty))],
            },
        );
        field.var = Some(field_var);

        standlone_properties.push(field);

        parser.finish()?;
    }

    Ok(ClassAttributes {
        base_ty,
        has_generated_init,
        is_tool,
        standlone_properties,
    })
}

/// Returns field names and 1 base field, if available
fn parse_fields(class: &Struct) -> ParseResult<Fields> {
    let mut all_fields = vec![];
    let mut base_field = Option::<Field>::None;

    let named_fields: Vec<(NamedField, Punct)> = match &class.fields {
        StructFields::Unit => {
            vec![]
        }
        StructFields::Tuple(_) => bail!(
            &class.fields,
            "#[derive(GodotClass)] not supported for tuple structs",
        )?,
        StructFields::Named(fields) => fields.fields.inner.clone(),
    };

    // Attributes on struct fields
    for (named_field, _punct) in named_fields {
        let mut is_base = false;
        let mut field = Field::from_named_field(&named_field);

        // #[base]
        if let Some(parser) = KvParser::parse(&named_field.attributes, "base")? {
            if let Some(prev_base) = base_field.as_ref() {
                bail!(
                    parser.span(),
                    "#[base] allowed for at most 1 field, already applied to `{}`",
                    prev_base.name
                )?;
            }
            is_base = true;
            parser.finish()?;
        }

        // #[init]
        if let Some(mut parser) = KvParser::parse(&named_field.attributes, "init")? {
            let default = parser.handle_expr("default")?;
            field.default = default;
            parser.finish()?;
        }

        // #[export]
        if let Some(mut parser) = KvParser::parse(&named_field.attributes, "export")? {
            let export = FieldExport::new_from_kv(&mut parser)?;
            field.export = Some(export);
            parser.finish()?;
        }

        // #[var]
        if let Some(mut parser) = KvParser::parse(&named_field.attributes, "var")? {
            let var = FieldVar::new_from_kv(&mut parser)?;
            field.var = Some(var);
            parser.finish()?;
        }

        // Exported or Rust-only fields
        if is_base {
            base_field = Some(field);
        } else {
            all_fields.push(field);
        }
    }

    Ok(Fields {
        all_fields,
        base_field,
    })
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// General helpers

struct ClassAttributes {
    base_ty: Ident,
    has_generated_init: bool,
    is_tool: bool,
    standlone_properties: Vec<Field>,
}

fn make_godot_init_impl(class_name: &Ident, fields: Fields) -> TokenStream {
    let base_init = if let Some(Field { name, .. }) = fields.base_field {
        quote! { #name: base, }
    } else {
        TokenStream::new()
    };

    let rest_init = fields.all_fields.into_iter().map(|field| {
        let field_name = field.name;
        let value_expr = match field.default {
            None => quote! { ::std::default::Default::default() },
            Some(default) => default,
        };
        quote! { #field_name: #value_expr, }
    });

    quote! {
        impl ::godot::obj::cap::GodotInit for #class_name {
            fn __godot_init(base: ::godot::obj::Base<Self::Base>) -> Self {
                Self {
                    #( #rest_init )*
                    #base_init
                }
            }
        }
    }
}

fn make_config_impl(class_name: &Ident, is_tool: bool) -> TokenStream {
    quote! {
        impl #class_name {
            #[doc(hidden)]
            pub fn __config() -> ::godot::private::ClassConfig {
                ::godot::private::ClassConfig {
                    is_tool: #is_tool,
                }
            }
        }
    }
}
