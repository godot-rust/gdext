/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::util::{bail, ident, KvParser};
use crate::ParseResult;
use proc_macro2::{Ident, Punct, TokenStream};
use quote::{format_ident, quote};
use venial::{Declaration, NamedField, Struct, StructFields, TyExpr};

pub fn transform(decl: Declaration) -> ParseResult<TokenStream> {
    let class = decl
        .as_struct()
        .ok_or_else(|| venial::Error::new("Not a valid struct"))?;

    let struct_cfg = parse_struct_attributes(class)?;
    let fields = parse_fields(class)?;

    let base_ty = &struct_cfg.base_ty;
    let class_name = &class.name;
    let class_name_str = class.name.to_string();
    let inherits_macro = format_ident!("inherits_transitive_{}", base_ty);

    let prv = quote! { ::godot::private };
    let deref_impl = make_deref_impl(class_name, &fields);

    let godot_exports_impl = make_exports_impl(class_name, &fields);

    let (godot_init_impl, create_fn);
    if struct_cfg.has_generated_init {
        godot_init_impl = make_godot_init_impl(class_name, fields);
        create_fn = quote! { Some(#prv::callbacks::create::<#class_name>) };
    } else {
        godot_init_impl = TokenStream::new();
        create_fn = quote! { None };
    };

    Ok(quote! {
        impl ::godot::obj::GodotClass for #class_name {
            type Base = ::godot::engine::#base_ty;
            type Declarer = ::godot::obj::dom::UserDomain;
            type Mem = <Self::Base as ::godot::obj::GodotClass>::Mem;

            const CLASS_NAME: &'static str = #class_name_str;
        }

        #godot_init_impl
        #godot_exports_impl
        #deref_impl

        ::godot::sys::plugin_add!(__GODOT_PLUGIN_REGISTRY in #prv; #prv::ClassPlugin {
            class_name: #class_name_str,
            component: #prv::PluginComponent::ClassDef {
                base_class_name: <::godot::engine::#base_ty as ::godot::obj::GodotClass>::CLASS_NAME,
                generated_create_fn: #create_fn,
                free_fn: #prv::callbacks::free::<#class_name>,
            },
        });

        #prv::class_macros::#inherits_macro!(#class_name);
    })
}

/// Returns the name of the base and the default mode
fn parse_struct_attributes(class: &Struct) -> ParseResult<ClassAttributes> {
    let mut base_ty = ident("RefCounted");
    let mut has_generated_init = false;

    // #[func] attribute on struct
    if let Some(mut parser) = KvParser::parse(&class.attributes, "class")? {
        if let Some(base) = parser.handle_ident("base")? {
            base_ty = base;
        }

        if parser.handle_alone("init")? {
            has_generated_init = true;
        }

        parser.finish()?;
    }

    Ok(ClassAttributes {
        base_ty,
        has_generated_init,
    })
}

/// Returns field names and 1 base field, if available
fn parse_fields(class: &Struct) -> ParseResult<Fields> {
    let mut all_field_names = vec![];
    let mut exported_fields = vec![];
    let mut base_field = Option::<Field>::None;

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

    // Attributes on struct fields
    for (field, _punct) in fields {
        let mut is_base = false;

        // #[base]
        if let Some(parser) = KvParser::parse(&field.attributes, "base")? {
            if let Some(prev_base) = base_field {
                bail(
                    format!(
                        "#[base] allowed for at most 1 field, already applied to '{}'",
                        prev_base.name
                    ),
                    parser.span(),
                )?;
            }
            is_base = true;
            base_field = Some(Field::new(&field));
            parser.finish()?;
        }

        // #[export]
        if let Some(mut parser) = KvParser::parse(&field.attributes, "export")? {
            let exported_field = ExportedField::new_from_kv(Field::new(&field), &mut parser)?;
            exported_fields.push(exported_field);
            parser.finish()?;
        }

        // Exported or Rust-only fields
        if !is_base {
            all_field_names.push(field.name.clone())
        }
    }

    Ok(Fields {
        all_field_names,
        base_field,
        exported_fields,
    })
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// General helpers

struct ClassAttributes {
    base_ty: Ident,
    has_generated_init: bool,
}

struct Fields {
    all_field_names: Vec<Ident>,
    base_field: Option<Field>,
    exported_fields: Vec<ExportedField>,
}

struct Field {
    name: Ident,
    _ty: TyExpr,
}

impl Field {
    fn new(field: &NamedField) -> Self {
        Self {
            name: field.name.clone(),
            _ty: field.ty.clone(),
        }
    }
}

struct ExportedField {
    field: Field,
    getter: String,
    setter: String,
    variant_type: String,
}

impl ExportedField {
    pub fn new_from_kv(field: Field, parser: &mut KvParser) -> ParseResult<ExportedField> {
        let getter = parser.handle_lit_required("getter")?;
        let setter = parser.handle_lit_required("setter")?;
        let variant_type = parser.handle_lit_required("variant_type")?;

        Ok(ExportedField {
            field,
            getter,
            setter,
            variant_type,
        })
    }
}

fn make_godot_init_impl(class_name: &Ident, fields: Fields) -> TokenStream {
    let base_init = if let Some(Field { name, .. }) = fields.base_field {
        quote! { #name: base, }
    } else {
        TokenStream::new()
    };

    let rest_init = fields.all_field_names.into_iter().map(|field| {
        quote! { #field: std::default::Default::default(), }
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

fn make_deref_impl(class_name: &Ident, fields: &Fields) -> TokenStream {
    let base_field = if let Some(Field { name, .. }) = &fields.base_field {
        name
    } else {
        return TokenStream::new();
    };

    quote! {
        impl std::ops::Deref for #class_name {
            type Target = <Self as ::godot::obj::GodotClass>::Base;

            fn deref(&self) -> &Self::Target {
                &*self.#base_field
            }
        }
        impl std::ops::DerefMut for #class_name {
            fn deref_mut(&mut self) -> &mut Self::Target {
                &mut *self.#base_field
            }
        }
    }
}

fn make_exports_impl(class_name: &Ident, fields: &Fields) -> TokenStream {
    let export_tokens = fields
        .exported_fields
        .iter()
        .map(|exported_field: &ExportedField| {
            use std::str::FromStr;
            let name = exported_field.field.name.to_string();
            let getter = proc_macro2::Literal::from_str(&exported_field.getter).unwrap();
            let setter = proc_macro2::Literal::from_str(&exported_field.setter).unwrap();
            let vtype = &exported_field.variant_type;
            let variant_type: TokenStream = vtype[1..vtype.len() - 1].parse().unwrap();
            quote! {
                let class_name = ::godot::builtin::StringName::from(#class_name::CLASS_NAME);
                let property_info = ::godot::builtin::meta::PropertyInfo::new(
                    #variant_type,
                    ::godot::builtin::meta::ClassName::of::<#class_name>(),
                    ::godot::builtin::StringName::from(#name),
                );
                let property_info_sys = property_info.property_sys();

                let getter_string_name = ::godot::builtin::StringName::from(#getter);
                let setter_string_name = ::godot::builtin::StringName::from(#setter);
                unsafe {
                    ::godot::sys::interface_fn!(classdb_register_extension_class_property)(
                        ::godot::sys::get_library(),
                        class_name.string_sys(),
                        std::ptr::addr_of!(property_info_sys),
                        setter_string_name.string_sys(),
                        getter_string_name.string_sys(),
                    );
                }
            }
        });
    quote! {
        impl ::godot::obj::cap::ImplementsGodotExports for #class_name {
            fn __register_exports() {
                #(
                    {
                        #export_tokens
                    }
                )*
            }
        }
    }
}
