/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use proc_macro2::{Ident, Punct, TokenStream};
use quote::{format_ident, quote, quote_spanned};

use crate::class::{
    make_property_impl, make_virtual_callback, BeforeKind, Field, FieldDefault, FieldExport,
    FieldVar, Fields, SignatureInfo,
};
use crate::util::{bail, error, ident, path_ends_with_complex, require_api_version, KvParser};
use crate::{handle_mutually_exclusive_keys, util, ParseResult};

pub fn derive_godot_class(item: venial::Item) -> ParseResult<TokenStream> {
    let class = item
        .as_struct()
        .ok_or_else(|| venial::Error::new("Not a valid struct"))?;

    let named_fields = named_fields(class)?;
    let mut struct_cfg = parse_struct_attributes(class)?;
    let mut fields = parse_fields(named_fields, struct_cfg.init_strategy)?;
    let is_editor_plugin = struct_cfg.is_editor_plugin();

    let mut deprecations = std::mem::take(&mut struct_cfg.deprecations);
    deprecations.append(&mut fields.deprecations);

    let errors = fields.errors.iter().map(|error| error.to_compile_error());

    let class_name = &class.name;
    let class_name_str: String = struct_cfg
        .rename
        .map_or_else(|| class.name.clone(), |rename| rename)
        .to_string();

    // Determine if we can use ASCII for the class name (in most cases).
    let class_name_allocation = if class_name_str.is_ascii() {
        let c_str = util::c_str(&class_name_str);
        quote! { ClassName::alloc_next_ascii(#c_str) }
    } else {
        quote! { ClassName::alloc_next_unicode(#class_name_str) }
    };

    let class_name_obj = util::class_name_obj(class_name);

    let is_internal = struct_cfg.is_internal;
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

    let godot_withbase_impl = if let Some(Field { name, ty, .. }) = &fields.base_field {
        // Apply the span of the field's type so that errors show up on the field's type.
        quote_spanned! { ty.span()=>
            impl ::godot::obj::WithBaseField for #class_name {
                fn to_gd(&self) -> ::godot::obj::Gd<#class_name> {
                    // By not referencing the base field directly here we ensure that the user only gets one error when the base
                    // field's type is wrong.
                    let base = <#class_name as ::godot::obj::WithBaseField>::base_field(self);
                    base.to_gd().cast()
                }

                fn base_field(&self) -> &::godot::obj::Base<<#class_name as ::godot::obj::GodotClass>::Base> {
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

                let name: &'static ClassName = CLASS_NAME.get_or_init(|| #class_name_allocation);
                *name
            }
        }

        unsafe impl ::godot::obj::Bounds for #class_name {
            type Memory = <<Self as ::godot::obj::GodotClass>::Base as ::godot::obj::Bounds>::Memory;
            type DynMemory = <<Self as ::godot::obj::GodotClass>::Base as ::godot::obj::Bounds>::DynMemory;
            type Declarer = ::godot::obj::bounds::DeclUser;
            type Exportable = <<Self as ::godot::obj::GodotClass>::Base as ::godot::obj::Bounds>::Exportable;
        }

        #godot_init_impl
        #godot_withbase_impl
        #godot_exports_impl
        #user_class_impl
        #init_expecter
        #( #deprecations )*
        #( #errors )*

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
                is_internal: #is_internal,
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
    is_internal: bool,
    rename: Option<Ident>,
    deprecations: Vec<TokenStream>,
}

impl ClassAttributes {
    fn is_editor_plugin(&self) -> bool {
        self.base_ty == ident("EditorPlugin")
    }
}

fn make_godot_init_impl(class_name: &Ident, fields: &Fields) -> TokenStream {
    let base_init = if let Some(Field { name, ty, .. }) = &fields.base_field {
        let base_type =
            quote_spanned! { ty.span()=> <#class_name as ::godot::obj::GodotClass>::Base };
        let base_field_type = quote_spanned! { ty.span()=> ::godot::obj::Base<#base_type> };
        let base = quote_spanned! { ty.span()=>
            <#base_field_type as ::godot::obj::IsBase<#base_type, #ty>>::conv(base)
        };

        quote_spanned! { ty.span()=> #name: #base, }
    } else {
        TokenStream::new()
    };

    let rest_init = fields.all_fields.iter().map(|field| {
        let field_name = field.name.clone();
        let value_expr = field
            .default_val
            .clone()
            .map(|field| field.default_val)
            // Use quote_spanned with the field's span so that errors show up on the field and not the derive macro.
            .unwrap_or_else(|| quote_spanned! { field.span=> ::std::default::Default::default() });

        quote! {#field_name: #value_expr, }
    });

    quote! {
        impl ::godot::obj::cap::GodotDefault for #class_name {
            fn __godot_user_init(base: ::godot::obj::Base<<#class_name as ::godot::obj::GodotClass>::Base>) -> Self {
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
    #[cfg(feature = "codegen-full")]
    let rpc_registrations =
        quote! { ::godot::register::private::auto_register_rpcs::<#class_name>(self); };
    #[cfg(not(feature = "codegen-full"))]
    let rpc_registrations = TokenStream::new();

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
                #rpc_registrations
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
    let mut is_internal = false;
    let mut rename: Option<Ident> = None;
    let mut deprecations = vec![];

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

        // Deprecated #[class(editor_plugin)]
        if let Some(_attr_key) = parser.handle_alone_with_span("editor_plugin")? {
            deprecations.push(quote_spanned! { _attr_key.span()=>
                ::godot::__deprecated::emit_deprecated_warning!(class_editor_plugin);
            });
        }

        // #[class(rename = NewName)]
        rename = parser.handle_ident("rename")?;

        // #[class(internal)]
        // Named "internal" following Godot terminology: https://github.com/godotengine/godot-cpp/blob/master/include/godot_cpp/core/class_db.hpp#L327
        if let Some(span) = parser.handle_alone_with_span("internal")? {
            require_api_version!("4.2", span, "#[class(internal)]")?;
            is_internal = true;
        }

        // Deprecated #[class(hidden)]
        if let Some(ident) = parser.handle_alone_with_span("hidden")? {
            require_api_version!("4.2", &ident, "#[class(hidden)]")?;
            is_internal = true;

            deprecations.push(quote_spanned! { ident.span()=>
                ::godot::__deprecated::emit_deprecated_warning!(class_hidden);
            });
        }

        parser.finish()?;
    }

    post_validate(&base_ty, is_tool)?;

    Ok(ClassAttributes {
        base_ty,
        init_strategy,
        is_tool,
        is_internal,
        rename,
        deprecations,
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
    let mut deprecations = vec![];
    let mut errors = vec![];

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

            // #[init(val = expr)]
            if let Some(default) = parser.handle_expr("val")? {
                field.default_val = Some(FieldDefault {
                    default_val: default,
                    span: parser.span(),
                });
            }

            // Deprecated #[init(default = expr)]
            if let Some(default) = parser.handle_expr("default")? {
                if field.default_val.is_some() {
                    return bail!(
                        parser.span(),
                        "Cannot use both `val` and `default` keys in #[init]; prefer using `val`"
                    );
                }
                field.default_val = Some(FieldDefault {
                    default_val: default,
                    span: parser.span(),
                });
                deprecations.push(quote_spanned! { parser.span()=>
                    ::godot::__deprecated::emit_deprecated_warning!(init_default);
                })
            }

            // #[init(node = "NodePath")]
            if let Some(node_path) = parser.handle_expr("node")? {
                let mut is_well_formed = true;
                if !field.is_onready {
                    is_well_formed = false;
                    errors.push(error!(
                        parser.span(),
                        "The key `node` in attribute #[init] requires field of type `OnReady<T>`\n\
				         Help: The syntax #[init(node = \"NodePath\")] is equivalent to \
				         #[init(val = OnReady::node(\"NodePath\"))], \
				         which can only be assigned to fields of type `OnReady<T>`"
                    ));
                }

                if field.default_val.is_some() {
                    is_well_formed = false;
                    errors.push(error!(
				        parser.span(),
				        "The key `node` in attribute #[init] is mutually exclusive with the key `default`\n\
				         Help: The syntax #[init(node = \"NodePath\")] is equivalent to \
				         #[init(val = OnReady::node(\"NodePath\"))], \
				         both aren't allowed since they would override each other"
			        ));
                }

                let default_val = if is_well_formed {
                    quote! { OnReady::node(#node_path) }
                } else {
                    quote! { todo!() }
                };

                field.default_val = Some(FieldDefault {
                    default_val,
                    span: parser.span(),
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
            if field.is_onready {
                errors.push(error!(
                    field.ty.clone(),
                    "base field cannot have type `OnReady<T>`"
                ));
            }

            if let Some(var) = field.var.as_ref() {
                errors.push(error!(
                    var.span,
                    "base field cannot have the attribute #[var]"
                ));
            }

            if let Some(export) = field.export.as_ref() {
                errors.push(error!(
                    export.span,
                    "base field cannot have the attribute #[export]"
                ));
            }

            if let Some(default_val) = field.default_val.as_ref() {
                errors.push(error!(
                    default_val.span,
                    "base field cannot have the attribute #[init]"
                ));
            }

            if let Some(prev_base) = base_field.replace(field) {
                // Ensure at most one Base<T>.
                errors.push(error!(
                    // base_field.unwrap().name,
                    named_field,
                    "at most 1 field can have type Base<T>; previous is `{}`", prev_base.name
                ));
            }
        } else {
            all_fields.push(field);
        }
    }

    Ok(Fields {
        all_fields,
        base_field,
        deprecations,
        errors,
    })
}

fn handle_opposite_keys(
    parser: &mut KvParser,
    key: &str,
    attribute: &str,
) -> ParseResult<Option<bool>> {
    let antikey = format!("no_{}", key);
    let result = handle_mutually_exclusive_keys(parser, attribute, &[key, &antikey])?;

    if let Some(idx) = result {
        Ok(Some(idx == 0))
    } else {
        Ok(None)
    }
}

/// Checks more logical combinations of attributes.
fn post_validate(base_ty: &Ident, is_tool: bool) -> ParseResult<()> {
    // TODO: this should be delegated to either:
    // a) the type system: have a trait IsTool which is implemented when #[class(tool)] is set.
    //    Then, for certain base classes, require a tool bound (e.g. generate method `fn type_check<T: IsTool>()`).
    //    This would also allow moving the logic to godot-codegen.
    // b) a runtime check in class.rs > register_class_raw() and validate_class_constraints().

    let class_name = base_ty.to_string();

    let is_class_extension = is_class_virtual_extension(&class_name);
    let is_class_editor = is_class_editor_only(&class_name);

    if is_class_extension && !is_tool {
        return bail!(
            base_ty,
            "Base class `{}` is a virtual extension class, which runs in the editor and thus requires #[class(tool)].",
            base_ty
        );
    } else if is_class_editor && !is_tool {
        return bail!(
            base_ty,
            "Base class `{}` is an editor-only class and thus requires #[class(tool)].",
            base_ty
        );
    }

    Ok(())
}

/// Whether a class exists primarily for GDExtension to overload virtual methods.
// See post_validate(). Should be moved to godot-codegen > special_cases.rs.
fn is_class_virtual_extension(godot_class_name: &str) -> bool {
    // Heuristic: suffix, with some exceptions.
    // Generally, a rule might also be "there is another class without that suffix", however that doesn't apply to e.g. OpenXRAPIExtension.

    match godot_class_name {
        "GDExtension" => false,

        _ => godot_class_name.ends_with("Extension"),
    }
}

/// Whether a class exists primarily as a plugin for the editor.
// See post_validate(). Should be moved to godot-codegen > special_cases.rs.
// TODO: This information is available in extension_api.json under classes.*.api_type and should be taken from there.
fn is_class_editor_only(godot_class_name: &str) -> bool {
    match godot_class_name {
        "FileSystemDock" | "ScriptCreateDialog" | "ScriptEditor" | "ScriptEditorBase" => true,
        _ => {
            (godot_class_name.starts_with("ResourceImporter")
                && godot_class_name != "ResourceImporter")
                || godot_class_name.starts_with("Editor")
        }
    }
}
