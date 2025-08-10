/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use proc_macro2::{Ident, Punct, TokenStream};
use quote::{format_ident, quote, quote_spanned};
use venial::Error;

use crate::class::data_models::fields::{named_fields, Fields};
use crate::class::data_models::group_export::FieldGroup;
use crate::class::{
    make_property_impl, make_virtual_callback, BeforeKind, Field, FieldCond, FieldDefault,
    FieldExport, FieldVar, GetterSetter, SignatureInfo,
};
use crate::util::{
    bail, error, format_funcs_collection_struct, ident, path_ends_with_complex,
    require_api_version, KvParser,
};
use crate::{handle_mutually_exclusive_keys, util, ParseResult};

pub fn derive_godot_class(item: venial::Item) -> ParseResult<TokenStream> {
    let class = item.as_struct().ok_or_else(|| {
        util::error_fn(
            "#[derive(GodotClass)] is only allowed on structs",
            item.name(),
        )
    })?;

    if class.generic_params.is_some() {
        return bail!(
            &class.generic_params,
            "#[derive(GodotClass)] does not support lifetimes or generic parameters",
        );
    }

    let mut modifiers = Vec::new();
    let named_fields = named_fields(class, "#[derive(GodotClass)]")?;
    let mut struct_cfg = parse_struct_attributes(class)?;
    let mut fields = parse_fields(named_fields, struct_cfg.init_strategy)?;

    if struct_cfg.is_editor_plugin() {
        modifiers.push(quote! { with_editor_plugin })
    }

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

    if struct_cfg.is_internal {
        modifiers.push(quote! { with_internal })
    }
    let base_ty = &struct_cfg.base_ty;
    #[cfg(all(feature = "register-docs", since_api = "4.3"))]
    let docs =
        crate::docs::document_struct(base_ty.to_string(), &class.attributes, &fields.all_fields);
    #[cfg(not(all(feature = "register-docs", since_api = "4.3")))]
    let docs = quote! {};
    let base_class = quote! { ::godot::classes::#base_ty };

    // Use this name because when typing a non-existent class, users will be met with the following error:
    //    could not find `inherit_from_OS__ensure_class_exists` in `class_macros`.
    let inherits_macro_ident = format_ident!("inherit_from_{}__ensure_class_exists", base_ty);

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

                    base.__constructed_gd().cast()
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
    let mut is_instantiable = true;

    match struct_cfg.init_strategy {
        InitStrategy::Generated => {
            godot_init_impl = make_godot_init_impl(class_name, &fields);
            modifiers.push(quote! { with_generated::<#class_name> });
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

            // Workaround for https://github.com/godot-rust/gdext/issues/874 before Godot 4.5.
            #[cfg(before_api = "4.5")]
            modifiers.push(quote! { with_generated_no_default::<#class_name> });
        }
    };
    if is_instantiable {
        modifiers.push(quote! { with_instantiable });
    }

    if has_default_virtual {
        modifiers.push(quote! { with_default_get_virtual_fn::<#class_name> });
    }

    if struct_cfg.is_tool {
        modifiers.push(quote! { with_tool })
    }

    // Declares a "funcs collection" struct that, for holds a constant for each #[func].
    // That constant maps the Rust name (constant ident) to the Godot registered name (string value).
    let funcs_collection_struct_name = format_funcs_collection_struct(class_name);
    let funcs_collection_struct = quote! {
        #[doc(hidden)]
        #[allow(non_camel_case_types)]
        pub struct #funcs_collection_struct_name {}
    };

    // Note: one limitation is that macros don't work for `impl nested::MyClass` blocks.
    let visibility_macro = make_visibility_macro(class_name, class.vis_marker.as_ref());
    let base_field_macro = make_base_field_macro(class_name, fields.base_field.is_some());
    let deny_manual_init_macro = make_deny_manual_init_macro(class_name, struct_cfg.init_strategy);

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

        #funcs_collection_struct
        #godot_init_impl
        #godot_withbase_impl
        #godot_exports_impl
        #user_class_impl
        #init_expecter
        #visibility_macro
        #base_field_macro
        #deny_manual_init_macro
        #( #deprecations )*
        #( #errors )*

        ::godot::sys::plugin_add!(#prv::__GODOT_PLUGIN_REGISTRY; #prv::ClassPlugin::new::<#class_name>(
            #prv::PluginItem::Struct(
                #prv::Struct::new::<#class_name>(#docs)#(.#modifiers())*
            )
        ));

        #prv::class_macros::#inherits_macro_ident!(#class_name);
    })
}

/// Generates code for a decl-macro, which takes any item and prepends it with the visibility marker of the class.
///
/// Used to access the visibility of the class in other proc-macros like `#[godot_api]`.
fn make_visibility_macro(
    class_name: &Ident,
    vis_marker: Option<&venial::VisMarker>,
) -> TokenStream {
    let macro_name = util::format_class_visibility_macro(class_name);

    quote! {
        macro_rules! #macro_name {
            (
                $( #[$meta:meta] )*
                struct $( $tt:tt )+
            ) => {
                $( #[$meta] )*
                #vis_marker struct $( $tt )+
            };

            // Can be expanded to `fn` etc. if needed.
        }
    }
}

/// Generates code for a decl-macro, which evaluates to nothing if the class has no base field.
fn make_base_field_macro(class_name: &Ident, has_base_field: bool) -> TokenStream {
    let macro_name = util::format_class_base_field_macro(class_name);

    if has_base_field {
        quote! {
            macro_rules! #macro_name {
                ( $( $tt:tt )* ) => { $( $tt )* };
            }
        }
    } else {
        quote! {
            macro_rules! #macro_name {
                ( $( $tt:tt )* ) => {};
            }
        }
    }
}

/// Generates code for a decl-macro that prevents manual `init()` for incompatible init strategies.
fn make_deny_manual_init_macro(class_name: &Ident, init_strategy: InitStrategy) -> TokenStream {
    let macro_name = util::format_class_deny_manual_init_macro(class_name);

    let class_attr = match init_strategy {
        InitStrategy::Absent => "#[class(no_init)]",
        InitStrategy::Generated => "#[class(init)]",
        InitStrategy::UserDefined => {
            // For classes that expect manual init, do nothing.
            return quote! {
                macro_rules! #macro_name {
                    () => {};
                }
            };
        }
    };

    let error_message =
        format!("Class `{class_name}` is marked with {class_attr} but provides an init() method.");

    quote! {
        macro_rules! #macro_name {
            () => {
                compile_error!(#error_message);
            };
        }
    }
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
        quote_spanned! { ty.span()=> #name: base, }
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

        quote! { #field_name: #value_expr, }
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

fn make_onready_init(all_fields: &[Field]) -> TokenStream {
    let onready_fields = all_fields
        .iter()
        .filter(|&field| field.is_onready)
        .map(|field| {
            let field = &field.name;
            quote! {
                ::godot::private::auto_init(&mut self.#field, &base);
            }
        })
        .collect::<Vec<_>>();

    if !onready_fields.is_empty() {
        quote! {
            {
                let base = <Self as ::godot::obj::WithBaseField>::to_gd(self).upcast();
                #( #onready_fields )*
            }
        }
    } else {
        TokenStream::new()
    }
}

fn make_oneditor_panic_inits(class_name: &Ident, all_fields: &[Field]) -> TokenStream {
    // Despite its name OnEditor shouldn't panic in the editor for tool classes.
    let is_in_editor = quote! { ::godot::classes::Engine::singleton().is_editor_hint() };

    let are_all_oneditor_fields_valid = quote! { are_all_oneditor_fields_valid };

    // Informs the user which fields haven't been set, instead of panicking on the very first one. Useful for debugging.
    let on_editor_fields_checks = all_fields
        .iter()
        .filter(|&field| field.is_oneditor)
        .map(|field| {
            let field = &field.name;
            let warning_message =
                format! { "godot-rust: OnEditor field {field} hasn't been initialized."};

            quote! {
                if this.#field.is_invalid() {
                    ::godot::global::godot_warn!(#warning_message);
                    #are_all_oneditor_fields_valid = false;
                }
            }
        })
        .collect::<Vec<_>>();

    if !on_editor_fields_checks.is_empty() {
        quote! {
            fn __are_oneditor_fields_initalized(this: &#class_name) -> bool {
                // Early return for `#[class(tool)]`.
                if #is_in_editor {
                    return true;
                }

                let mut #are_all_oneditor_fields_valid: bool = true;

                #( #on_editor_fields_checks )*

                #are_all_oneditor_fields_valid
            }

            if !__are_oneditor_fields_initalized(&self) {
                panic!("OnEditor fields must be properly initialized before ready.")
            }
        }
    } else {
        TokenStream::new()
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

    let onready_inits = make_onready_init(all_fields);

    let oneditor_panic_inits = make_oneditor_panic_inits(class_name, all_fields);

    let run_before_ready = !onready_inits.is_empty() || !oneditor_panic_inits.is_empty();

    let default_virtual_fn = if run_before_ready {
        let tool_check = util::make_virtual_tool_check();
        let signature_info = SignatureInfo::fn_ready();

        let callback =
            make_virtual_callback(class_name, &signature_info, BeforeKind::OnlyBefore, None);

        // See also __virtual_call() codegen.
        // This doesn't explicitly check if the base class inherits from Node (and thus has `_ready`), but the derive-macro already does
        // this for the `OnReady` field declaration.
        let (hash_param, matches_ready_hash);
        if cfg!(since_api = "4.4") {
            hash_param = quote! { hash: u32, };
            matches_ready_hash = quote! {
                (name, hash) == ::godot::sys::godot_virtual_consts::Node::ready
            };
        } else {
            hash_param = TokenStream::new();
            matches_ready_hash = quote! { name == "_ready" }
        }

        let default_virtual_fn = quote! {
            fn __default_virtual_call(
                name: &str,
                #hash_param
            ) -> ::godot::sys::GDExtensionClassCallVirtual {
                use ::godot::obj::UserClass as _;
                #tool_check

                if #matches_ready_hash {
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
            #[doc(hidden)]
            fn __config() -> ::godot::private::ClassConfig {
                ::godot::private::ClassConfig {
                    is_tool: #is_tool,
                }
            }

            #[doc(hidden)]
            fn __before_ready(&mut self) {
                #oneditor_panic_inits
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

        // OnEditor<T> type inference
        if path_ends_with_complex(&field.ty, "OnEditor") {
            field.is_oneditor = true;
        }

        // PhantomVar<T> type inference
        if path_ends_with_complex(&field.ty, "PhantomVar") {
            field.is_phantomvar = true;
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

            // #[init(val = EXPR)]
            if let Some(default) = parser.handle_expr("val")? {
                field.default_val = Some(FieldDefault {
                    default_val: default,
                    span: parser.span(),
                });
            }

            // Deprecated #[init(default = expr)]
            if let Some((key, default)) = parser.handle_expr_with_key("default")? {
                if field.default_val.is_some() {
                    return bail!(
                        key,
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

            // #[init(node = "PATH")]
            if let Some(node_path) = parser.handle_expr("node")? {
                field.set_default_val_if(
                    || quote! { OnReady::from_node(#node_path) },
                    FieldCond::IsOnReady,
                    &parser,
                    &mut errors,
                );
            }

            // #[init(load = "PATH")]
            if let Some(resource_path) = parser.handle_expr("load")? {
                field.set_default_val_if(
                    || quote! { OnReady::from_loaded(#resource_path) },
                    FieldCond::IsOnReady,
                    &parser,
                    &mut errors,
                );
            }

            // #[init(sentinel = EXPR)]
            if let Some(sentinel_value) = parser.handle_expr("sentinel")? {
                field.set_default_val_if(
                    || quote! { OnEditor::from_sentinel(#sentinel_value) },
                    FieldCond::IsOnEditor,
                    &parser,
                    &mut errors,
                );
            }

            parser.finish()?;
        }

        // #[export]
        if let Some(mut parser) = KvParser::parse(&named_field.attributes, "export")? {
            let export = FieldExport::new_from_kv(&mut parser)?;
            field.export = Some(export);
            parser.finish()?;
        }

        // #[export_group(name = ..., prefix = ...)]
        if let Some(mut parser) = KvParser::parse(&named_field.attributes, "export_group")? {
            let group = FieldGroup::new_from_kv(&mut parser)?;
            field.group = Some(group);
            parser.finish()?;
        }

        // #[export_subgroup(name = ..., prefix = ...)]
        if let Some(mut parser) = KvParser::parse(&named_field.attributes, "export_subgroup")? {
            let subgroup = FieldGroup::new_from_kv(&mut parser)?;
            field.subgroup = Some(subgroup);
            parser.finish()?;
        }

        // #[var]
        if let Some(mut parser) = KvParser::parse(&named_field.attributes, "var")? {
            let mut var = FieldVar::new_from_kv(&mut parser)?;
            if !field.is_phantomvar {
                var.default_to_generated_getter_setter();
            }
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

            // Not yet implemented for OnEditor.

            parser.finish()?;
        }

        // Extra validation; eventually assign to base_fields or all_fields.
        if is_base {
            validate_base_field(&field, &mut errors);

            if let Some(prev_base) = base_field.replace(field) {
                // Ensure at most one Base<T>.
                errors.push(error!(
                    // base_field.unwrap().name,
                    named_field,
                    "at most 1 field can have type Base<T>; previous is `{}`", prev_base.name
                ));
            }
        } else {
            if field.is_phantomvar {
                validate_phantomvar_field(&field, &mut errors);
            }

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

fn validate_base_field(field: &Field, errors: &mut Vec<Error>) {
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
}

fn validate_phantomvar_field(field: &Field, errors: &mut Vec<Error>) {
    let Some(field_var) = &field.var else {
        errors.push(error!(
            field.span,
            "PhantomVar<T> field is useless without attribute #[var]"
        ));
        return;
    };

    // For now, we do not support write-only properties. Godot does not fully support them either; it silently returns null
    // when the property is being read. This is probably because the editor needs to be able to read exported properties,
    // to show them in the inspector and serialize them to disk.
    // See also this discussion:
    // https://github.com/godot-rust/gdext/pull/1261#discussion_r2255335223
    match field_var.getter {
        GetterSetter::Omitted => {
            errors.push(error!(
                field_var.span,
                "PhantomVar<T> requires a custom getter"
            ));
        }
        GetterSetter::Generated => {
            errors.push(error!(
                field_var.span,
                "PhantomVar<T> stores no data, so it cannot use an autogenerated getter"
            ));
        }
        GetterSetter::Custom(_) => {}
    }

    // The setter may either be custom or omitted.
    match field_var.setter {
        GetterSetter::Omitted => {}
        GetterSetter::Generated => {
            errors.push(error!(
                field_var.span,
                "PhantomVar<T> stores no data, so it cannot use an autogenerated setter"
            ));
        }
        GetterSetter::Custom(_) => {}
    }
}

fn handle_opposite_keys(
    parser: &mut KvParser,
    key: &str,
    attribute: &str,
) -> ParseResult<Option<bool>> {
    let antikey = format!("no_{key}");
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
