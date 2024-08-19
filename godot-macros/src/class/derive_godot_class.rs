/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use proc_macro2::{Ident, Punct, Span, TokenStream};
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
    let var_fields = fields.vars();

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
    let script = struct_cfg.script;
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
    
    let script_impl = if script {
        make_script_impl(class_name, class_name_str.clone(), base_ty, var_fields)?
    } else {
        quote!{}
    };

    let mut init_expecter = TokenStream::new();
    let mut godot_init_impl = TokenStream::new();
    let mut create_fn = quote! { None };
    let mut recreate_fn = quote! { None };
    let mut is_instantiable = true;
    let deprecations = &fields.deprecations;

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
            type Exportable = <<Self as ::godot::obj::GodotClass>::Base as ::godot::obj::Bounds>::Exportable;
        }

        #script_impl
        #godot_init_impl
        #godot_withbase_impl
        #godot_exports_impl
        #user_class_impl
        #init_expecter
        #( #deprecations )*

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

#[derive(Copy, Clone, PartialEq)]
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
    script: bool
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
            .default_val
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

/// Creates the script instance struct and implements [crate::obj::script::ScriptInstance]
/// and [crate::classes::IScriptExtension] on their appropriate types.
fn make_script_impl(class_name: &Ident, class_name_str: String, base_ty: &Ident, var_fields: Vec<&Field>) -> ParseResult<TokenStream> {
    let instance_class_name_str = format!("{}Instance", class_name_str);
    let instance_class_name = Ident::new(&instance_class_name_str, Span::call_site());

    // turn var_fields into a useful property list

    // for get_script_property_list (Array<Dictionary>)
    let mut prop_list = quote! {};
    for prop in &var_fields {
        let name = format!("{}", prop.name);
        let f_type = if let Some(path) = prop.ty.as_path() {
            path
        } else {
            return Err(venial::Error::new(format!("The property {} is not a valid type", name)));
        };

        // Check if it is a Gd<>
        // Left empty if it is a builtin type
        let mut prop_class_name = String::new();
        let mut gd_builtin_type = quote! { r#""type": 0"# };
        if format!("{}", f_type.segments[0].ident) == "godot" {
            if format!("{}", f_type.segments[1].ident) == "obj" {
                if format!("{}", f_type.segments[2].ident) == "Gd" {
                    if let Some(generic) = &f_type.segments[2].generic_args {
                        if let Some((generic, _)) = generic.args.first() {
                            if let venial::GenericArg::TypeOrConst { expr } = generic {
                                if let Some(path) = expr.as_path() {
                                    // The contained generic cannot contain a generic as that's
                                    // not supported by godot yet, so the last argument is guaranteed the actual type
                                    prop_class_name = format!("{}", path.segments[path.segments.len() - 1].ident);
                                    // 24 = Object
                                    gd_builtin_type = quote! { r#""type": 24"# };
                                } else {
                                    return Err(venial::Error::new(format!("The property {} is not a valid type", name)));
                                };
                            }
                        }
                    } else {
                        let ident: proc_macro2::Ident = f_type.segments[2].ident.clone();

                        return bail! {
                            ident,
                            "One of the properties is a godot::obj::Gd with no generic argument. This should be impossible."
                        };
                    }
                }
            } else if format!("{}", f_type.segments[1].ident) == "builtin" {
                gd_builtin_type = quote! { "type": ::godot::builtin::Variant::from(#f_type::new()).get_type() as u8 };
            }
        }

        // determine meta info about the property, e.g. exported
        // TODO 

        prop_list = quote!{
            #prop_list,
            ::godot::builtin::dict! {
                "name": ::godot::builtin::GString::from(#name),
                "class_name": ::godot::builtin::StringName::from(#prop_class_name),
                "type": 0,
                "hint": 0,
                "hint_string": ::godot::builtin::GString::new(),
                "usage": 0
            }
        };
    }

    // All into one Array
    prop_list = quote! {
        ::godot::builtin::array! [#prop_list]
    };

    // create a match for get_property
    let mut get_property_match = quote! {};
    for prop in &var_fields {
        let name_str = format!("{}", prop.name);
        let name = prop.name.clone();

        get_property_match = quote!{
            ::godot::builtin::StringName::from(#name_str) => Some(self.#name),
        };
    }

    get_property_match = quote! {
        match property {
            #get_property_match
            _ => None
        }
    };

    return Ok(quote! {
        use ::godot::classes::IScriptExtension;
        #[godot_api]
        impl ::godot::classes::IScriptExtension for #class_name {
            fn editor_can_reload_from_file(&mut self) -> bool {
                // The script is compiled along with the GDExtension, and reloaded with it.
                false
            }
            fn get_method_info(&self, method: StringName) -> Dictionary {
                todo!()
            }

            fn can_instantiate(&self) -> bool {
                true
            }

            fn get_base_script(&self) -> Option<::godot::obj::Gd<::godot::classes::Script>> {
                Some(self.base().clone().upcast())
            }

            fn get_global_name(&self) -> ::godot::builtin::StringName {
                ::godot::builtin::StringName::from(#class_name_str)
            }

            fn inherits_script(&self, script: ::godot::obj::Gd<::godot::classes::Script>) -> bool {
                // unwrap safe: get_base_script always succeeds
                if self.get_base_script().unwrap() == script {
                    return true;
                } else {
                    return false;
                }
            }

            fn get_instance_base_type(&self) -> StringName {
                ::godot::builtin::StringName::from(c"ScriptExtension")
            }

            unsafe fn instance_create(&self, for_object: ::godot::obj::Gd<::godot::classes::Object>) -> *mut ::std::ffi::c_void {
                let inst: #instance_class_name = #instance_class_name::from(self);
                let for_dcast: ::godot::obj::Gd<::godot::classes::ScriptExtension> = for_object.cast();
                ::godot::obj::script::create_script_instance(inst, for_dcast)
            }

            unsafe fn placeholder_instance_create(&self, _for_object: ::godot::obj::Gd<::godot::classes::Object>) -> *mut ::std::ffi::c_void {
                unreachable!("{} is not a placeholder!", #class_name_str);
            }

            fn instance_has(&self, object: ::godot::obj::Gd<::godot::classes::Object>) -> bool {
                return match object.get_script().try_to::<::godot::obj::Gd<Self>>() {
                    Ok(_) => true,
                    Err(_) => false
                };
            }

            fn has_source_code(&self) -> bool {
                false
            }

            fn get_source_code(&self) -> ::godot::builtin::GString {
                // has_source_code returns false, should never be called
                unreachable!("{} has no source code!", #class_name_str);
            }

            fn set_source_code(&mut self, code: ::godot::builtin::GString) {
                // has_source_code returns false, should never be called
                unreachable!("{} has no source code!", #class_name_str);
            }

            fn reload(&mut self, keep_state: bool) -> ::godot::global::Error {
                // editor_can_reload_from_file returns false, should never be called
                unreachable!("{} should never be reloaded!", #class_name_str);
            }

            fn get_documentation(&self) -> ::godot::builtin::Array<::godot::builtin::Dictionary> {
                todo!("Parse custom methods for #[script(doc = blah)]")
            }

            fn has_method(&self, method: ::godot::builtin::StringName) -> bool {
                todo!("Custom methods")
            }

            fn has_static_method(&self, method: ::godot::builtin::StringName) -> bool {
                todo!("Custom methods")
            }

            fn is_tool(&self) -> bool {
                true
            }

            fn is_valid(&self) -> bool {
                true
            }

            fn get_language(&self) -> Option<::godot::obj::Gd<::godot::classes::ScriptLanguage>> {
                todo!()
            }

            fn has_script_signal(&self, signal: ::godot::builtin::StringName) -> bool {
                todo!()
            }

            fn get_script_method_list(&self) -> ::godot::builtin::Array<::godot::builtin::Dictionary> {
                todo!()
            }

            fn has_property_default_value(&self, property: ::godot::builtin::StringName) -> bool {
                todo!("What is this?")
            }

            fn get_property(&self, property: ::godot::builtin::StringName) -> Option<::godot::builtin::Variant> {
                return #get_property_match
            }

            fn update_exports(&mut self) {
                todo!()
            }

            fn get_script_property_list(&self) -> ::godot::builtin::Array<::godot::builtin::Dictionary> {
                #prop_list
            }

            fn get_member_line(&self, member: ::godot::builtin::StringName) -> i32 {
                // Script is compiled Rust, should never be called
                unreachable!("{} has no source code!", #class_name_str);
            }

            fn get_constants(&self) -> ::godot::builtin::Dictionary {
                todo!()
            }

            fn get_members(&self) -> ::godot::builtin::Array<::godot::builtin::StringName> {
                todo!()
            }

            fn is_placeholder_fallback_enabled(&self) -> bool {
                false
            }

            fn get_rpc_config(&self) -> ::godot::builtin::Variant {
                todo!()
            }

            fn get_script_signal_list(&self) -> ::godot::builtin::Array<::godot::builtin::Dictionary> {
                todo!()
            }

            fn get_property_default_value(&self, property: ::godot::builtin::StringName) -> ::godot::builtin::Variant {
                todo!()
            }
        }

        struct #instance_class_name {
            script: ::godot::obj::Gd<::godot::classes::Script>
        }

        impl ::std::convert::From<&#class_name> for #instance_class_name {
            fn from(value: &#class_name) -> Self {
                let gd_cast: ::godot::obj::Gd<::godot::classes::Script> = ::godot::obj::WithBaseField::to_gd(value).upcast();

                #instance_class_name {
                    script: gd_cast
                }
            }
        }

        impl ::godot::obj::script::ScriptInstance for #instance_class_name {
            type Base = #base_ty;

            fn class_name(&self) -> ::godot::builtin::GString {
                ::godot::builtin::GString::from(#class_name_str)
            }

            fn set_property(this: ::godot::obj::script::SiMut<Self>, name: ::godot::builtin::StringName, value: &::godot::builtin::Variant) -> bool {
                false
            }

            fn get_property(&self, name: ::godot::builtin::StringName) -> Option<::godot::builtin::Variant> {
                todo!()
            }

            fn get_property_list(&self) -> Vec<::godot::meta::PropertyInfo> {
                todo!()
            }

            fn get_method_list(&self) -> Vec<::godot::meta::MethodInfo> {
                todo!()
            }

            fn call(
                this: ::godot::obj::script::SiMut<Self>,
                method: ::godot::builtin::StringName,
                args: &[&::godot::builtin::Variant],
            ) -> Result<::godot::builtin::Variant, ::godot::sys::GDExtensionCallErrorType> {
                todo!()
            }

            fn is_placeholder(&self) -> bool {
                false
            }

            fn has_method(&self, method: ::godot::builtin::StringName) -> bool {
                todo!()
            }

            fn get_script(&self) -> &::godot::obj::Gd<::godot::classes::Script> {
                &self.script
            }

            fn get_property_type(&self, name: ::godot::builtin::StringName) -> VariantType {
                todo!()
            }

            fn to_string(&self) -> ::godot::builtin::GString {
                ::godot::builtin::GString::from(#instance_class_name_str)
            }

            fn get_property_state(&self) -> Vec<(::godot::builtin::StringName, ::godot::builtin::Variant)> {
                todo!()
            }

            fn get_language(&self) -> ::godot::obj::Gd<::godot::classes::ScriptLanguage> {
                todo!()
            }

            fn on_refcount_decremented(&self) -> bool {
                false
            }

            fn on_refcount_incremented(&self) {}

            fn property_get_fallback(&self, name: ::godot::builtin::StringName) -> Option<::godot::builtin::Variant> {
                todo!()
            }

            fn property_set_fallback(this: ::godot::obj::script::SiMut<Self>, name: ::godot::builtin::StringName, value: &::godot::builtin::Variant) -> bool {
                todo!()
            }
        }
    });
}

/// Returns the name of the base and the default mode
fn parse_struct_attributes(class: &venial::Struct) -> ParseResult<ClassAttributes> {
    let mut base_ty = ident("RefCounted");
    let mut init_strategy = InitStrategy::UserDefined;
    let mut is_tool = false;
    let mut is_editor_plugin = false;
    let mut is_hidden = false;
    let mut rename: Option<Ident> = None;
    let mut script = false;

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

        // #[class(script)]
        if let Some(attr_key) = parser.handle_alone_with_span("script")? {
            script = true;

            if base_ty != ident("ScriptExtension") {
                return bail!(
                    attr_key,
                    "#[class(script)] requires additional key-value base=ScriptExtension"
                );
            }

            if !is_tool {
                return bail!(
                    attr_key,
                    "#[class(script)] requires additional key `tool`"
                );
            }

            if init_strategy != InitStrategy::Generated {
                return bail!(
                    attr_key,
                    "#[class(script)] requires additional key `init`"
                );
            }
        }

        parser.finish()?;
    }

    post_validate(&base_ty, is_tool, is_editor_plugin)?;

    Ok(ClassAttributes {
        base_ty,
        init_strategy,
        is_tool,
        is_editor_plugin,
        is_hidden,
        rename,
        script
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
                field.default_val = Some(default);
            }

            // Deprecated #[init(default = expr)]
            if let Some(default) = parser.handle_expr("default")? {
                if field.default_val.is_some() {
                    return bail!(
                        parser.span(),
                        "Cannot use both `val` and `default` keys in #[init]; prefer using `val`"
                    );
                }
                field.default_val = Some(default);
                deprecations.push(quote! {
                    ::godot::__deprecated::emit_deprecated_warning!(init_default);
                })
            }

            // #[init(node = "NodePath")]
            if let Some(node_path) = parser.handle_expr("node")? {
                if !field.is_onready {
                    return bail!(
                        parser.span(),
                        "The key `node` in attribute #[init] requires field of type `OnReady<T>`\n\
				         Help: The syntax #[init(node = \"NodePath\")] is equivalent to \
				         #[init(val = OnReady::node(\"NodePath\"))], \
				         which can only be assigned to fields of type `OnReady<T>`"
                    );
                }

                if field.default_val.is_some() {
                    return bail!(
				        parser.span(),
				        "The key `node` in attribute #[init] is mutually exclusive with the key `default`\n\
				         Help: The syntax #[init(node = \"NodePath\")] is equivalent to \
				         #[init(val = OnReady::node(\"NodePath\"))], \
				         both aren't allowed since they would override each other"
			        );
                }

                field.default_val = Some(quote! {
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
                || field.default_val.is_some()
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
        deprecations,
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

/// Checks more logical combinations of attributes.
fn post_validate(base_ty: &Ident, is_tool: bool, is_editor_plugin: bool) -> ParseResult<()> {
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
    } else if class_name == "EditorPlugin" && !is_editor_plugin {
        return bail!(
            base_ty,
            "Classes extending `{}` require #[class(editor_plugin)] to get registered as a plugin in the editor. See: https://godot-rust.github.io/book/recipes/editor-plugin/index.html", 
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
