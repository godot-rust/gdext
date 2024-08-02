/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use proc_macro2::{Ident, Punct, TokenStream};
use quote::{format_ident, quote};

use crate::class::{
    make_property_impl, make_virtual_callback, BeforeKind, Field, FieldExport, FieldVar, Fields,
    SignatureInfo,
};
use crate::util::{bail, ident, path_ends_with_complex, require_api_version, KvParser};
use crate::{util, ParseResult};

pub fn derive_godot_class(item: venial::Item) -> ParseResult<TokenStream> {
    let class = item
        .as_struct()
        .ok_or_else(|| venial::Error::new("Not a valid struct"))?;

    let named_fields = named_fields(class)?;
    let struct_cfg = parse_struct_attributes(class)?;
    let fields = parse_fields(named_fields, struct_cfg.init_strategy)?;

    let class_name = &class.name;
    let class_name_str: String = struct_cfg
        .rename
        .map_or_else(|| class.name.clone(), |rename| rename)
        .to_string();

    let class_name_cstr = util::c_str(&class_name_str);
    let class_name_obj = util::class_name_obj(class_name);

    let is_editor_plugin = struct_cfg.is_editor_plugin;
    let is_hidden = struct_cfg.is_hidden;
    let base_ty = &struct_cfg.base_ty;
    #[cfg(all(feature = "docs", since_api = "4.3"))]
    let docs = crate::docs::make_definition_docs(
        base_ty.to_string(),
        &class.attributes,
        &fields.all_fields,
    );
    #[cfg(not(all(feature = "docs", since_api = "4.3")))]
    let docs = quote! {};
    let base_class = quote! { ::godot::classes::#base_ty };
    let base_class_name_obj = util::class_name_obj(&base_class);
    let inherits_macro = format_ident!("unsafe_inherits_transitive_{}", base_ty);

    let prv = quote! { ::godot::private };
    let godot_exports_impl = make_property_impl(class_name, &fields);

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
        TokenStream::new()
    };

    let (user_class_impl, has_default_virtual) =
        make_user_class_impl(class_name, struct_cfg.is_tool, &fields.all_fields);

    let mut init_expecter = TokenStream::new();
    let mut godot_init_impl = TokenStream::new();
    let mut create_fn = quote! { None };
    let mut recreate_fn = quote! { None };
    let mut is_instantiable = true;

    match struct_cfg.init_strategy {
        InitStrategy::Generated => {
            godot_init_impl = make_godot_init_impl(class_name, &fields);
            create_fn = quote! { Some(#prv::callbacks::create::<#class_name>) };

            if cfg!(since_api = "4.2") {
                recreate_fn = quote! { Some(#prv::callbacks::recreate::<#class_name>) };
            }
        }
        InitStrategy::UserDefined => {
            let fn_name = format_ident!("class_{}_must_have_an_init_method", class_name);
            init_expecter = quote! {
                #[allow(non_snake_case)]
                fn #fn_name() {
                    fn __type_check<T: ::godot::obj::cap::GodotDefault>() {}

                    __type_check::<#class_name>();
                }
            }
        }
        InitStrategy::Absent => {
            is_instantiable = false;
        }
    };

    let default_get_virtual_fn = if has_default_virtual {
        quote! { Some(#prv::callbacks::default_get_virtual::<#class_name>) }
    } else {
        quote! { None }
    };

    let is_tool = struct_cfg.is_tool;

    Ok(quote! {
        impl ::godot::obj::GodotClass for #class_name {
            type Base = #base_class;

            // Code duplicated in godot-codegen.
            fn class_name() -> ::godot::meta::ClassName {
                use ::godot::meta::ClassName;

                // Optimization note: instead of lazy init, could use separate static which is manually initialized during registration.
                static CLASS_NAME: std::sync::OnceLock<ClassName> = std::sync::OnceLock::new();

                let name: &'static ClassName = CLASS_NAME.get_or_init(|| ClassName::alloc_next(#class_name_cstr));
                *name
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
        #init_expecter

        ::godot::sys::plugin_add!(__GODOT_PLUGIN_REGISTRY in #prv; #prv::ClassPlugin {
            class_name: #class_name_obj,
            item: #prv::PluginItem::Struct {
                base_class_name: #base_class_name_obj,
                generated_create_fn: #create_fn,
                generated_recreate_fn: #recreate_fn,
                register_properties_fn: #prv::ErasedRegisterFn {
                    raw: #prv::callbacks::register_user_properties::<#class_name>,
                },
                free_fn: #prv::callbacks::free::<#class_name>,
                default_get_virtual_fn: #default_get_virtual_fn,
                is_tool: #is_tool,
                is_editor_plugin: #is_editor_plugin,
                is_hidden: #is_hidden,
                is_instantiable: #is_instantiable,
                #docs
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

#[derive(Copy, Clone)]
enum InitStrategy {
    Generated,
    UserDefined,
    Absent,
}

struct ClassAttributes {
    base_ty: Ident,
    init_strategy: InitStrategy,
    is_tool: bool,
    is_editor_plugin: bool,
    is_hidden: bool,
    rename: Option<Ident>,
}

fn make_godot_init_impl(class_name: &Ident, fields: &Fields) -> TokenStream {
    let base_init = if let Some(Field { name, .. }) = &fields.base_field {
        quote! { #name: base, }
    } else {
        TokenStream::new()
    };

    let rest_init = fields.all_fields.iter().map(|field| {
        let field_name = field.name.clone();
        let value_expr = field
            .default
            .clone()
            .unwrap_or_else(|| quote! { ::std::default::Default::default() });

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

fn make_user_class_impl(
    class_name: &Ident,
    is_tool: bool,
    all_fields: &[Field],
) -> (TokenStream, bool) {
    let onready_inits = {
        let mut onready_fields = all_fields
            .iter()
            .filter(|&field| field.is_onready)
            .map(|field| {
                let field = &field.name;
                quote! {
                    ::godot::private::auto_init(&mut self.#field, &base);
                }
            });

        if let Some(first) = onready_fields.next() {
            quote! {
                {
                    let base = <Self as godot::obj::WithBaseField>::to_gd(self).upcast();
                    #first
                    #( #onready_fields )*
                }
            }
        } else {
            TokenStream::new()
        }
    };

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
                #onready_inits
            }

            #default_virtual_fn
        }
    };

    (user_class_impl, default_virtual_fn.is_some())
}

/// Returns the name of the base and the default mode
fn parse_struct_attributes(class: &venial::Struct) -> ParseResult<ClassAttributes> {
    let mut base_ty = ident("RefCounted");
    let mut init_strategy = InitStrategy::UserDefined;
    let mut is_tool = false;
    let mut is_editor_plugin = false;
    let mut is_hidden = false;
    let mut rename: Option<Ident> = None;

    // #[class] attribute on struct
    if let Some(mut parser) = KvParser::parse(&class.attributes, "class")? {
        // #[class(base = Base)]
        if let Some(base) = parser.handle_ident("base")? {
            base_ty = base;
        }

        // #[class(init)], #[class(no_init)]
        match handle_opposite_keys(&mut parser, "init", "class")? {
            Some(true) => init_strategy = InitStrategy::Generated,
            Some(false) => init_strategy = InitStrategy::Absent,
            None => {}
        }

        // #[class(tool)]
        if parser.handle_alone("tool")? {
            is_tool = true;
        }

        // #[class(editor_plugin)]
        if let Some(attr_key) = parser.handle_alone_with_span("editor_plugin")? {
            is_editor_plugin = true;

            // Requires #[class(tool, base=EditorPlugin)].
            // The base=EditorPlugin check should come first to create the best compile errors since it's more complex to resolve.
            // See https://github.com/godot-rust/gdext/pull/773
            if base_ty != ident("EditorPlugin") {
                return bail!(
                    attr_key,
                    "#[class(editor_plugin)] requires additional key-value `base=EditorPlugin`"
                );
            }
            if !is_tool {
                return bail!(
                    attr_key,
                    "#[class(editor_plugin)] requires additional key `tool`"
                );
            }
        }

        // #[class(rename = NewName)]
        rename = parser.handle_ident("rename")?;

        // #[class(hidden)]
        // TODO consider naming this "internal"; godot-cpp uses that terminology:
        // https://github.com/godotengine/godot-cpp/blob/master/include/godot_cpp/core/class_db.hpp#L327
        if let Some(span) = parser.handle_alone_with_span("hidden")? {
            require_api_version!("4.2", span, "#[class(hidden)]")?;
            is_hidden = true;
        }

        parser.finish()?;
    }

    Ok(ClassAttributes {
        base_ty,
        init_strategy,
        is_tool,
        is_editor_plugin,
        is_hidden,
        rename,
    })
}

/// Fetches data for all named fields for a struct.
///
/// Errors if `class` is a tuple struct.
fn named_fields(class: &venial::Struct) -> ParseResult<Vec<(venial::NamedField, Punct)>> {
    // This is separate from parse_fields to improve compile errors.  The errors from here demand larger and more non-local changes from the API
    // user than those from parse_struct_attributes, so this must be run first.
    match &class.fields {
        venial::Fields::Unit => Ok(vec![]),
        venial::Fields::Tuple(_) => bail!(
            &class.fields,
            "#[derive(GodotClass)] not supported for tuple structs",
        )?,
        venial::Fields::Named(fields) => Ok(fields.fields.inner.clone()),
    }
}

/// Returns field names and 1 base field, if available.
fn parse_fields(
    named_fields: Vec<(venial::NamedField, Punct)>,
    init_strategy: InitStrategy,
) -> ParseResult<Fields> {
    let mut all_fields = vec![];
    let mut base_field = Option::<Field>::None;

    // Attributes on struct fields
    for (named_field, _punct) in named_fields {
        let mut is_base = false;
        let mut field = Field::new(&named_field);

        // Base<T> type inference
        if path_ends_with_complex(&field.ty, "Base") {
            is_base = true;
        }

        // OnReady<T> type inference
        if path_ends_with_complex(&field.ty, "OnReady") {
            field.is_onready = true;
        }

        // #[init]
        if let Some(mut parser) = KvParser::parse(&named_field.attributes, "init")? {
            // #[init] on fields is useless if there is no generated constructor.
            if !matches!(init_strategy, InitStrategy::Generated) {
                return bail!(
                    parser.span(),
                    "field attribute #[init] requires struct attribute #[class(init)]"
                );
            }

            // #[init(default = expr)]
            if let Some(default) = parser.handle_expr("default")? {
                field.default = Some(default);
            }

            // #[init(node = "NodePath")]
            if let Some(node_path) = parser.handle_expr("node")? {
                if !field.is_onready {
                    return bail!(
                        parser.span(),
                        "The key `node` in attribute #[init] requires field of type `OnReady<T>`\n\
				         Help: The syntax #[init(node = \"NodePath\")] is equivalent to \
				         #[init(default = OnReady::node(\"NodePath\"))], \
				         which can only be assigned to fields of type `OnReady<T>`"
                    );
                }

                if field.default.is_some() {
                    return bail!(
				        parser.span(),
				        "The key `node` in attribute #[init] is mutually exclusive with the key `default`\n\
				         Help: The syntax #[init(node = \"NodePath\")] is equivalent to \
				         #[init(default = OnReady::node(\"NodePath\"))], \
				         both aren't allowed since they would override each other"
			        );
                }

                field.default = Some(quote! {
                    OnReady::node(#node_path)
                });
            }

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

        // #[hint] to override type inference (must be at the end).
        if let Some(mut parser) = KvParser::parse(&named_field.attributes, "hint")? {
            if let Some(override_base) = handle_opposite_keys(&mut parser, "base", "hint")? {
                is_base = override_base;
            }

            if let Some(override_onready) = handle_opposite_keys(&mut parser, "onready", "hint")? {
                field.is_onready = override_onready;
            }
            parser.finish()?;
        }

        // Extra validation; eventually assign to base_fields or all_fields.
        if is_base {
            if field.is_onready
                || field.var.is_some()
                || field.export.is_some()
                || field.default.is_some()
            {
                return bail!(
                    named_field,
                    "base field cannot have type `OnReady<T>` or attributes #[var], #[export] or #[init]"
                );
            }

            if let Some(prev_base) = base_field.replace(field) {
                // Ensure at most one Base<T>.
                return bail!(
                    // base_field.unwrap().name,
                    named_field,
                    "at most 1 field can have type Base<T>; previous is `{}`",
                    prev_base.name
                );
            }
        } else {
            all_fields.push(field);
        }
    }

    Ok(Fields {
        all_fields,
        base_field,
    })
}

fn handle_opposite_keys(
    parser: &mut KvParser,
    key: &str,
    attribute: &str,
) -> ParseResult<Option<bool>> {
    let antikey = format!("no_{}", key);

    let is_key = parser.handle_alone(key)?;
    let is_no_key = parser.handle_alone(&antikey)?;

    match (is_key, is_no_key) {
        (true, false) => Ok(Some(true)),
        (false, true) => Ok(Some(false)),
        (false, false) => Ok(None),
        (true, true) => bail!(
            parser.span(),
            "#[{attribute}] attribute keys `{key}` and `{antikey}` are mutually exclusive",
        ),
    }
}
