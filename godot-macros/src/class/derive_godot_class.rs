/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use proc_macro2::{Ident, Punct, TokenStream};
use quote::{format_ident, quote};
use venial::{Declaration, NamedField, Struct, StructFields};

use crate::class::{
    make_property_impl, make_virtual_callback, BeforeKind, Field, FieldExport, FieldVar, Fields,
    SignatureInfo,
};
use crate::util::{bail, ident, path_ends_with_complex, KvParser};
use crate::{util, ParseResult};

pub fn derive_godot_class(decl: Declaration) -> ParseResult<TokenStream> {
    let class = decl
        .as_struct()
        .ok_or_else(|| venial::Error::new("Not a valid struct"))?;

    let struct_cfg = parse_struct_attributes(class)?;
    let fields = parse_fields(class)?;

    let class_name = &class.name;
    let class_name_str: String = struct_cfg
        .rename
        .map_or_else(|| class.name.clone(), |rename| rename)
        .to_string();
    let class_name_cstr = util::cstr_u8_slice(&class_name_str);
    let class_name_obj = util::class_name_obj(class_name);

    let base_ty = &struct_cfg.base_ty;
    let base_class = quote! { ::godot::engine::#base_ty };
    let base_class_name_obj = util::class_name_obj(&base_class);
    let inherits_macro = format_ident!("inherits_transitive_{}", base_ty);

    let prv = quote! { ::godot::private };
    let godot_exports_impl = make_property_impl(class_name, &fields);

    let editor_plugin = if struct_cfg.is_editor_plugin {
        quote! {
            ::godot::sys::plugin_add!(__GODOT_PLUGIN_REGISTRY in #prv; #prv::ClassPlugin {
                class_name: #class_name_obj,
                component: #prv::PluginComponent::EditorPlugin,
                init_level: <#class_name as ::godot::obj::GodotClass>::INIT_LEVEL,
            });

            const _: () = #prv::is_editor_plugin::<#class_name>();
        }
    } else {
        quote! {}
    };

    let hidden = if struct_cfg.is_hidden {
        quote! {
            ::godot::sys::plugin_add!(__GODOT_PLUGIN_REGISTRY in #prv; #prv::ClassPlugin {
                class_name: #class_name_obj,
                component: #prv::PluginComponent::Unexposed,
                init_level: <#class_name as ::godot::obj::GodotClass>::INIT_LEVEL,
            });
        }
    } else {
        quote! {}
    };

    let godot_withbase_impl = if let Some(Field { name, .. }) = &fields.base_field {
        quote! {
            impl ::godot::obj::WithBaseField for #class_name {
                fn to_gd(&self) -> ::godot::obj::Gd<Self> {
                    self.#name.to_gd().cast()
                }

                fn base_field(&self) -> &::godot::obj::Base<<Self as ::godot::obj::GodotClass>::Base> {
                    &self.#name
                }
            }
        }
    } else {
        quote! {}
    };

    let (user_class_impl, has_default_virtual) =
        make_user_class_impl(class_name, struct_cfg.is_tool, &fields.all_fields);

    let (godot_init_impl, create_fn, recreate_fn);
    if struct_cfg.has_generated_init {
        godot_init_impl = make_godot_init_impl(class_name, fields);
        create_fn = quote! { Some(#prv::callbacks::create::<#class_name>) };
        if cfg!(since_api = "4.2") {
            recreate_fn = quote! { Some(#prv::callbacks::recreate::<#class_name>) };
        } else {
            recreate_fn = quote! { None };
        }
    } else {
        godot_init_impl = TokenStream::new();
        create_fn = quote! { None };
        recreate_fn = quote! { None };
    };

    let default_get_virtual_fn = if has_default_virtual {
        quote! { Some(#prv::callbacks::default_get_virtual::<#class_name>) }
    } else {
        quote! { None }
    };

    Ok(quote! {
        impl ::godot::obj::GodotClass for #class_name {
            type Base = #base_class;

            fn class_name() -> ::godot::builtin::meta::ClassName {
                ::godot::builtin::meta::ClassName::from_ascii_cstr(#class_name_cstr)
            }
        }

        unsafe impl ::godot::obj::Bounds for #class_name {
            type Memory = <<Self as ::godot::obj::GodotClass>::Base as ::godot::obj::Bounds>::Memory;
            type DynMemory = <<Self as ::godot::obj::GodotClass>::Base as ::godot::obj::Bounds>::DynMemory;
            type Declarer = ::godot::obj::bounds::DeclUser;
        }

        #godot_init_impl
        #godot_withbase_impl
        #godot_exports_impl
        #user_class_impl

        ::godot::sys::plugin_add!(__GODOT_PLUGIN_REGISTRY in #prv; #prv::ClassPlugin {
            class_name: #class_name_obj,
            component: #prv::PluginComponent::ClassDef {
                base_class_name: #base_class_name_obj,
                generated_create_fn: #create_fn,
                generated_recreate_fn: #recreate_fn,
                register_properties_fn: #prv::ErasedRegisterFn {
                    raw: #prv::callbacks::register_user_properties::<#class_name>,
                },
                free_fn: #prv::callbacks::free::<#class_name>,
                default_get_virtual_fn: #default_get_virtual_fn,
            },
            init_level: {
                let level = <#class_name as ::godot::obj::GodotClass>::INIT_LEVEL;
                let base_level = <#base_class as ::godot::obj::GodotClass>::INIT_LEVEL;

                // Sanity check for init levels. Note that this does not cover cases where GodotClass is manually defined;
                // might make sense to add a run-time check during class registration.
                assert!(
                    level >= base_level,
                    "Class `{class}` has init level `{level:?}`, but its base class has init level `{base_level:?}`.\n\
                    A class cannot be registered before its base class.",
                    class = #class_name_str,
                );

                level
            }
        });

        #editor_plugin
        #hidden

        #prv::class_macros::#inherits_macro!(#class_name);
    })
}

fn make_user_class_impl(
    class_name: &Ident,
    is_tool: bool,
    all_fields: &[Field],
) -> (TokenStream, bool) {
    let onready_field_inits = all_fields
        .iter()
        .filter(|&field| field.is_onready)
        .map(|field| {
            let field = &field.name;
            quote! {
                ::godot::private::auto_init(&mut self.#field);
            }
        });

    let default_virtual_fn = if all_fields.iter().any(|field| field.is_onready) {
        let tool_check = util::make_virtual_tool_check();
        let signature_info = SignatureInfo::fn_ready();

        let callback = make_virtual_callback(class_name, signature_info, BeforeKind::OnlyBefore);
        let default_virtual_fn = quote! {
            fn __default_virtual_call(name: &str) -> ::godot::sys::GDExtensionClassCallVirtual {
                use ::godot::obj::UserClass as _;
                #tool_check

                if name == "_ready" {
                    #callback
                } else {
                    None
                }
            }
        };
        Some(default_virtual_fn)
    } else {
        None
    };

    let user_class_impl = quote! {
        impl ::godot::obj::UserClass for #class_name {
            fn __config() -> ::godot::private::ClassConfig {
                ::godot::private::ClassConfig {
                    is_tool: #is_tool,
                }
            }

            fn __before_ready(&mut self) {
                #( #onready_field_inits )*
            }

            #default_virtual_fn
        }
    };

    (user_class_impl, default_virtual_fn.is_some())
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
    let mut is_editor_plugin = false;
    let mut is_hidden = false;
    let mut rename: Option<Ident> = None;

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

        // TODO: better error message when using in Godot 4.0
        if parser.handle_alone_ident("editor_plugin")?.is_some() {
            is_editor_plugin = true;
        }

        if parser.handle_alone("hide")? {
            is_hidden = true;
        }

        rename = parser.handle_ident("rename")?;

        parser.finish()?;
    }

    Ok(ClassAttributes {
        base_ty,
        has_generated_init,
        is_tool,
        is_editor_plugin,
        is_hidden,
        rename,
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
        let mut field = Field::new(&named_field);

        // #[base]
        if let Some(parser) = KvParser::parse(&named_field.attributes, "base")? {
            if let Some(prev_base) = base_field.as_ref() {
                return bail!(
                    parser.span(),
                    "#[base] allowed for at most 1 field, already applied to `{}`",
                    prev_base.name
                );
            }
            is_base = true;
            parser.finish()?;
        }

        // OnReady<T> type inference
        if path_ends_with_complex(&field.ty, "OnReady") {
            field.is_onready = true;
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

    // TODO detect #[base] based on type instead of attribute
    // Edge cases (type aliases, user types with same name, ...) could be handled with #[hint(base)] or #[hint(no_base)].

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
    is_editor_plugin: bool,
    is_hidden: bool,
    rename: Option<Ident>,
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
        impl ::godot::obj::cap::GodotDefault for #class_name {
            fn __godot_user_init(base: ::godot::obj::Base<Self::Base>) -> Self {
                Self {
                    #( #rest_init )*
                    #base_init
                }
            }
        }
    }
}
