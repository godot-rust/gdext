/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use proc_macro2::{Delimiter, Group, Ident, TokenStream};
use quote::{quote, ToTokens};

use crate::class::data_models::func::extract_gd_self;
use crate::class::{into_signature_info, make_virtual_callback, BeforeKind, SignatureInfo};
use crate::util::{bail, ident, KvParser};
use crate::{util, ParseResult};

/// Codegen for `#[godot_api] impl ISomething for MyType`.
pub fn transform_trait_impl(mut original_impl: venial::Impl) -> ParseResult<TokenStream> {
    let (class_name, trait_path, trait_base_class) =
        util::validate_trait_impl_virtual(&original_impl, "godot_api")?;

    let prv = quote! { ::godot::private };

    let register_docs = crate::docs::make_interface_impl_docs_registration(
        &original_impl.body_items,
        &class_name,
        &prv,
    );

    let mut decls = IDecls::default();
    let mut removed_methods_idx = Vec::new();
    for (index, item) in original_impl.body_items.iter_mut().enumerate() {
        let method = if let venial::ImplMember::AssocFunction(f) = item {
            f
        } else {
            continue;
        };

        let is_gd_self = is_gd_self(&method.attributes)?;
        // Methods with gd_self will be rewritten outside trait.
        if is_gd_self {
            removed_methods_idx.push(index);
        }

        // Transport #[cfg] attributes to the virtual method's FFI glue, to ensure it won't be
        // registered in Godot if conditionally removed from compilation.
        let cfg_attrs = util::extract_cfg_attrs(&method.attributes)
            .into_iter()
            .collect::<Vec<_>>();

        let method_name_str = method.name.to_string();
        match method_name_str.as_str() {
            "register_class" => {
                validate_not_gd_self(is_gd_self, method)?;
                handle_register_class(&class_name, &trait_path, cfg_attrs, &mut decls);
            }
            "init" => {
                validate_not_gd_self(is_gd_self, method)?;
                handle_init(&class_name, &trait_path, cfg_attrs, &mut decls);
            }
            "to_string" => {
                handle_to_string(&class_name, &trait_path, cfg_attrs, &mut decls, is_gd_self);
            }
            "on_notification" => {
                // POSTINIT notification can't be handled with the gd_self receiver
                // since object will not be yet constructed.
                validate_not_gd_self(is_gd_self, method)?;
                handle_on_notification(&class_name, &trait_path, cfg_attrs, &mut decls);
            }
            "get_property" => {
                handle_get_property(&class_name, &trait_path, cfg_attrs, &mut decls, is_gd_self);
            }
            "set_property" => {
                handle_set_property(&class_name, &trait_path, cfg_attrs, &mut decls, is_gd_self);
            }
            "validate_property" => {
                handle_validate_property(
                    &class_name,
                    &trait_path,
                    cfg_attrs,
                    &mut decls,
                    is_gd_self,
                );
            }
            "get_property_list" => {
                handle_get_property_list(
                    &class_name,
                    &trait_path,
                    cfg_attrs,
                    &mut decls,
                    is_gd_self,
                );
            }
            "property_get_revert" => {
                handle_property_get_revert(
                    &class_name,
                    &trait_path,
                    cfg_attrs,
                    &mut decls,
                    is_gd_self,
                );
            }
            regular_virtual_fn => {
                // Break borrow chain to allow handle_regular_virtual_fn() to mutably borrow `method` and modify `original_impl` through it.
                // let cfg_attrs = cfg_attrs.iter().cloned().collect()

                // All the non-special engine ones: ready(), process(), etc.
                // Can modify original_impl, concretely the fn body for f64->f32 conversions.
                let changed_function = handle_regular_virtual_fn(
                    &class_name,
                    &trait_path,
                    method,
                    regular_virtual_fn,
                    cfg_attrs,
                    &mut decls,
                    is_gd_self,
                )?;

                // If the function is modified (e.g. process() declared with f32), apply changes here.
                // Borrow-checker: we cannot reassign whole function due to shared borrow on `method.attributes`.
                // Thus, separately update signature + body when needed.
                if let Some((new_params, new_body)) = changed_function {
                    method.params = new_params;
                    method.body = Some(new_body);
                    //panic!("modify params: {}", method.params.to_token_stream().to_string());
                }
            }
        }
    }

    // If there is no ready() method explicitly overridden, we need to add one, to ensure that __before_ready() is called to
    // initialize the OnReady fields.
    if is_possibly_node_class(&trait_base_class)
        && !decls
            .overridden_virtuals
            .iter()
            .any(|v| v.rust_method_name == "_ready")
    {
        let match_arm = OverriddenVirtualFn {
            cfg_attrs: vec![],
            rust_method_name: "_ready".to_string(),
            // Can't use `virtuals::ready` here, as the base class might not be `Node` (see above why such a branch is still added).
            godot_name_hash_constant: quote! { ::godot::private::virtuals::Node::ready },
            signature_info: SignatureInfo::fn_ready(),
            before_kind: BeforeKind::OnlyBefore,
            interface_trait: None,
        };

        decls.overridden_virtuals.push(match_arm);
    }

    let tool_check = util::make_virtual_tool_check();

    let modifications = decls.modifiers.drain(..).map(|(cfg_attrs, modifier)| {
        quote! {
            #(#cfg_attrs)*
            { item = item.#modifier::<#class_name>(); }
        }
    });

    let item_constructor = quote! {
        {
            let mut item = #prv::ITraitImpl::new::<#class_name>();
            #(#modifications)*
            item
        }
    };

    // See also __default_virtual_call() codegen.
    let (hash_param, match_expr);
    if cfg!(since_api = "4.4") {
        hash_param = quote! { hash: u32, };
        match_expr = quote! { (name, hash) };
    } else {
        hash_param = TokenStream::new();
        match_expr = quote! { name };
    };

    let virtual_match_arms = decls
        .overridden_virtuals
        .iter()
        .map(|v| v.make_match_arm(&class_name, &trait_base_class));

    let mut result = quote! {
        // #original_impl and gd_self_impls are inserted below.
        #decls

        impl ::godot::private::You_forgot_the_attribute__godot_api for #class_name {}

        impl ::godot::obj::cap::ImplementsGodotVirtual for #class_name {
            fn __virtual_call(name: &str, #hash_param) -> ::godot::sys::GDExtensionClassCallVirtual {
                //println!("virtual_call: {}.{}", std::any::type_name::<Self>(), name);
                use ::godot::obj::UserClass as _;
                use ::godot::private::virtuals::#trait_base_class as virtuals;
                #tool_check

                match #match_expr {
                    #( #virtual_match_arms )*
                    _ => None,
                }
            }
        }

        ::godot::sys::plugin_add!(#prv::__GODOT_PLUGIN_REGISTRY; #prv::ClassPlugin::new::<#class_name>(
            #prv::PluginItem::ITraitImpl(#item_constructor)
        ));

        #register_docs
    };

    // #decls still holds a mutable borrow to `original_impl`, so we mutate && append it afterwards.

    let mut gd_self_decls = Vec::new();
    for index in removed_methods_idx.into_iter().rev() {
        let venial::ImplMember::AssocFunction(mut method) = original_impl.body_items.remove(index)
        else {
            unreachable!("We made sure that it is a function earlier.")
        };

        method.attributes.retain(util::is_cfg_or_cfg_attr);

        gd_self_decls.push(method);
    }

    let gd_self_decl = quote! {
        #[allow(clippy::wrong_self_convention)]
        impl #class_name {
            #( #gd_self_decls )*
        }
    };

    gd_self_decl.to_tokens(&mut result);
    original_impl.to_tokens(&mut result);

    Ok(result)
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Handlers for individual symbols in #[godot_api].

fn handle_register_class<'a>(
    class_name: &Ident,
    trait_path: &venial::TypeExpr,
    cfg_attrs: Vec<&'a venial::Attribute>,
    decls: &mut IDecls<'a>,
) {
    let IDecls {
        register_class_impl,
        ..
    } = decls;

    // Implements the trait once for each implementation of this method, forwarding the cfg attrs of each
    // implementation to the generated trait impl. If the cfg attrs allow for multiple implementations of
    // this method to exist, then Rust will generate an error, so we don't have to worry about the multiple
    // trait implementations actually generating an error, since that can only happen if multiple
    // implementations of the same method are kept by #[cfg] (due to user error).
    // Thus, by implementing the trait once for each possible implementation of this method (depending on
    // what #[cfg] allows), forwarding the cfg attrs, we ensure this trait impl will remain in the code if
    // at least one of the method impls are kept.
    *register_class_impl = quote! {
        #register_class_impl

        #(#cfg_attrs)*
        impl ::godot::obj::cap::GodotRegisterClass for #class_name {
            fn __godot_register_class(builder: &mut ::godot::builder::GodotBuilder<Self>) {
                <Self as #trait_path>::register_class(builder)
            }
        }
    };

    decls.add_modifier(cfg_attrs, "with_register");
}

fn handle_init<'a>(
    class_name: &Ident,
    trait_path: &venial::TypeExpr,
    cfg_attrs: Vec<&'a venial::Attribute>,
    decls: &mut IDecls<'a>,
) {
    let IDecls {
        godot_init_impl, ..
    } = decls;

    // If #[class(init)] or #[class(no_init)] is provided, deny overriding manual init().
    let deny_manual_init_macro = util::format_class_deny_manual_init_macro(class_name);

    *godot_init_impl = quote! {
        #godot_init_impl
        #deny_manual_init_macro!();

        #(#cfg_attrs)*
        impl ::godot::obj::cap::GodotDefault for #class_name {
            fn __godot_user_init(base: ::godot::obj::Base<Self::Base>) -> Self {
                <Self as #trait_path>::init(base)
            }
        }
    };

    decls.add_modifier(cfg_attrs, "with_create");
}

fn handle_to_string<'a>(
    class_name: &Ident,
    trait_path: &venial::TypeExpr,
    cfg_attrs: Vec<&'a venial::Attribute>,
    decls: &mut IDecls<'a>,
    is_gd_self: bool,
) {
    let IDecls { to_string_impl, .. } = decls;

    let (receiver_path, type_decl, receiver_call) =
        make_inner_virtual_method_call(is_gd_self, false, trait_path);

    *to_string_impl = quote! {

        #(#cfg_attrs)*
        impl ::godot::obj::cap::GodotToString for #class_name {
            type Recv = #receiver_path;

            fn __godot_to_string(mut this: ::godot::private::VirtualMethodReceiver<Self>) -> ::godot::builtin::GString {

                #type_decl::to_string(#receiver_call)
            }
        }
    };

    decls.add_modifier(cfg_attrs, "with_string");
}

fn handle_on_notification<'a>(
    class_name: &Ident,
    trait_path: &venial::TypeExpr,
    cfg_attrs: Vec<&'a venial::Attribute>,
    decls: &mut IDecls<'a>,
) {
    let IDecls {
        on_notification_impl,
        ..
    } = decls;

    let inactive_class_early_return = make_inactive_class_check(TokenStream::new());
    *on_notification_impl = quote! {
        #on_notification_impl

        #(#cfg_attrs)*
        impl ::godot::obj::cap::GodotNotification for #class_name {
            fn __godot_notification(&mut self, what: i32) {
                use ::godot::obj::UserClass as _;

                #inactive_class_early_return

                <Self as #trait_path>::on_notification(self, what.into())
            }
        }
    };

    decls.add_modifier(cfg_attrs, "with_on_notification");
}

fn handle_get_property<'a>(
    class_name: &Ident,
    trait_path: &venial::TypeExpr,
    cfg_attrs: Vec<&'a venial::Attribute>,
    decls: &mut IDecls<'a>,
    is_gd_self: bool,
) {
    let IDecls {
        get_property_impl, ..
    } = decls;

    let (receiver_path, type_decl, receiver_call) =
        make_inner_virtual_method_call(is_gd_self, false, trait_path);

    let inactive_class_early_return = make_inactive_class_check(quote! { None });
    *get_property_impl = quote! {
        #(#cfg_attrs)*
        impl ::godot::obj::cap::GodotGet for #class_name {
            type Recv = #receiver_path;

            fn __godot_get_property(mut this: ::godot::private::VirtualMethodReceiver<Self>, property: ::godot::builtin::StringName) -> Option<::godot::builtin::Variant> {
                use ::godot::obj::UserClass as _;

                #inactive_class_early_return

                #type_decl::get_property(#receiver_call, property)
            }
        }
    };

    decls.add_modifier(cfg_attrs, "with_get_property");
}

fn handle_set_property<'a>(
    class_name: &Ident,
    trait_path: &venial::TypeExpr,
    cfg_attrs: Vec<&'a venial::Attribute>,
    decls: &mut IDecls<'a>,
    is_gd_self: bool,
) {
    let IDecls {
        set_property_impl, ..
    } = decls;

    let (receiver_path, type_decl, receiver_call) =
        make_inner_virtual_method_call(is_gd_self, true, trait_path);

    let inactive_class_early_return = make_inactive_class_check(quote! { false });
    *set_property_impl = quote! {
        #(#cfg_attrs)*
        impl ::godot::obj::cap::GodotSet for #class_name {
            type Recv = #receiver_path;

            fn __godot_set_property(mut this: ::godot::private::VirtualMethodReceiver<Self>, property: ::godot::builtin::StringName, value: ::godot::builtin::Variant) -> bool {
                use ::godot::obj::UserClass as _;

                #inactive_class_early_return

                #type_decl::set_property(#receiver_call, property, value)
            }
        }
    };

    decls.add_modifier(cfg_attrs, "with_set_property");
}

fn handle_validate_property<'a>(
    class_name: &Ident,
    trait_path: &venial::TypeExpr,
    cfg_attrs: Vec<&'a venial::Attribute>,
    decls: &mut IDecls<'a>,
    is_gd_self: bool,
) {
    let IDecls {
        validate_property_impl,
        ..
    } = decls;

    let (receiver_path, type_decl, receiver_call) =
        make_inner_virtual_method_call(is_gd_self, false, trait_path);

    let inactive_class_early_return = make_inactive_class_check(TokenStream::new());
    *validate_property_impl = quote! {
        #(#cfg_attrs)*
        impl ::godot::obj::cap::GodotValidateProperty for #class_name {
            type Recv = #receiver_path;

            fn __godot_validate_property(mut this: ::godot::private::VirtualMethodReceiver<Self>, property: &mut ::godot::meta::PropertyInfo) {
                use ::godot::obj::UserClass as _;

                #inactive_class_early_return
                #type_decl::validate_property(#receiver_call, property);
            }
        }
    };

    decls.add_modifier(cfg_attrs, "with_validate_property");
}

#[cfg(before_api = "4.3")] #[cfg_attr(published_docs, doc(cfg(before_api = "4.3")))]
fn handle_get_property_list<'a>(
    _class_name: &Ident,
    _trait_path: &venial::TypeExpr,
    cfg_attrs: Vec<&'a venial::Attribute>,
    decls: &mut IDecls<'a>,
    _is_gd_self: bool,
) {
    decls.get_property_list_impl = quote! {
        #(#cfg_attrs)*
        compile_error!("`get_property_list` is only supported for Godot versions of at least 4.3");
    };
}

#[cfg(since_api = "4.3")] #[cfg_attr(published_docs, doc(cfg(since_api = "4.3")))]
fn handle_get_property_list<'a>(
    class_name: &Ident,
    trait_path: &venial::TypeExpr,
    cfg_attrs: Vec<&'a venial::Attribute>,
    decls: &mut IDecls<'a>,
    is_gd_self: bool,
) {
    let IDecls {
        get_property_list_impl,
        ..
    } = decls;

    let (receiver_path, type_decl, receiver_call) =
        make_inner_virtual_method_call(is_gd_self, true, trait_path);

    // `get_property_list` is only supported in Godot API >= 4.3. If we add support for `get_property_list` to earlier
    // versions of Godot then this code is still needed and should be uncommented.
    //
    // let inactive_class_early_return = make_inactive_class_check(false);
    *get_property_list_impl = quote! {
        #(#cfg_attrs)*
        impl ::godot::obj::cap::GodotGetPropertyList for #class_name {
            type Recv = #receiver_path;

            fn __godot_get_property_list(mut this: ::godot::private::VirtualMethodReceiver<Self>) -> Vec<::godot::meta::PropertyInfo> {
                // #inactive_class_early_return
                #type_decl::get_property_list(#receiver_call)
            }
        }
    };

    decls.add_modifier(cfg_attrs, "with_get_property_list");
}

fn handle_property_get_revert<'a>(
    class_name: &Ident,
    trait_path: &venial::TypeExpr,
    cfg_attrs: Vec<&'a venial::Attribute>,
    decls: &mut IDecls<'a>,
    is_gd_self: bool,
) {
    let IDecls {
        property_get_revert_impl,
        ..
    } = decls;

    let (receiver_path, type_decl, receiver_call) =
        make_inner_virtual_method_call(is_gd_self, false, trait_path);

    let inactive_class_early_return = make_inactive_class_check(quote! { None });
    *property_get_revert_impl = quote! {
        #(#cfg_attrs)*
        impl ::godot::obj::cap::GodotPropertyGetRevert for #class_name {
            type Recv = #receiver_path;

            fn __godot_property_get_revert(this: ::godot::private::VirtualMethodReceiver<Self>, property: StringName) -> Option<::godot::builtin::Variant> {
                use ::godot::obj::UserClass as _;

                #inactive_class_early_return

                #type_decl::property_get_revert(#receiver_call, property)
            }
        }
    };

    decls.add_modifier(cfg_attrs, "with_property_get_revert");
}

fn handle_regular_virtual_fn<'a>(
    class_name: &Ident,
    trait_path: &venial::TypeExpr,
    original_method: &venial::Function,
    method_name: &str,
    cfg_attrs: Vec<&'a venial::Attribute>,
    decls: &mut IDecls<'a>,
    has_gd_self: bool,
) -> ParseResult<Option<(venial::Punctuated<venial::FnParam>, Group)>> {
    let method_name_ident = original_method.name.clone();
    let mut method = util::reduce_to_signature(original_method);

    if has_gd_self {
        extract_gd_self(&mut method, &original_method.name)?;
    }

    // Godot-facing name begins with underscore.
    //
    // godot-codegen special-cases the virtual method called _init (which exists on a handful of classes, distinct from the default
    // constructor) to init_ext, to avoid Rust-side ambiguity. See godot_codegen::class_generator::virtual_method_name.
    let virtual_method_name = if method_name == "init_ext" {
        String::from("_init")
    } else {
        format!("_{method_name}")
    };

    let signature_info = into_signature_info(method, class_name, has_gd_self);

    let mut updated_function = None;
    // If there was a signature change (e.g. f32 -> f64 in process/physics_process), apply to new function tokens.
    if !signature_info.modified_param_types.is_empty() {
        let mut param_name = None;

        let mut new_params = original_method.params.clone();
        for (index, new_ty) in signature_info.modified_param_types.iter() {
            let venial::FnParam::Typed(typed) = &mut new_params.inner[*index].0 else {
                panic!("unexpected parameter type: {new_params:?}");
            };

            typed.ty = new_ty.clone();
            param_name = Some(typed.name.clone());
        }

        let original_body = &original_method.body;
        let param_name = param_name.expect("parameter had no name");

        // Currently hardcoded to f32/f64 exchange; can be generalized if needed.
        let body_code = quote! {
            let #param_name = #param_name as f32;
            #original_body
        };

        let wrapping_body = Group::new(Delimiter::Brace, body_code);

        updated_function = Some((new_params, wrapping_body));
    }

    // Overridden ready() methods additionally have an additional `__before_ready()` call (for OnReady inits).
    let before_kind = if method_name == "ready" {
        BeforeKind::WithBefore
    } else {
        BeforeKind::Without
    };

    // Note that, if the same method is implemented multiple times (with different cfg attr combinations),
    // then there will be multiple match arms annotated with the same cfg attr combinations, thus they will
    // be reduced to just one arm (at most, if the implementations aren't all removed from compilation) for
    // each distinct method.
    decls.overridden_virtuals.push(OverriddenVirtualFn {
        cfg_attrs,
        rust_method_name: virtual_method_name,
        // If ever the `I*` verbatim validation is relaxed (it won't work with use-renames or other weird edge cases), the approach
        // with godot::private::virtuals module could be changed to something like the following (GodotBase = nearest Godot base class):
        // __get_virtual_hash::<Self::GodotBase>("method")
        godot_name_hash_constant: quote! { virtuals::#method_name_ident },
        signature_info,
        before_kind,
        interface_trait: Some(trait_path.clone()),
    });

    Ok(updated_function)
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Rest of implementation

/// Returns `false` if the given class does definitely not inherit `Node`, `true` otherwise.
///
/// `#[godot_api]` has currently no way of checking base class at macro-resolve time, so the `_ready` branch is unconditionally
/// added, even for classes that don't inherit from `Node`. As a best-effort, we exclude some very common non-Node classes explicitly, to
/// generate less useless code.
fn is_possibly_node_class(trait_base_class: &Ident) -> bool {
    !matches!(
        trait_base_class.to_string().as_str(), //.
        "Object"
            | "MainLoop"
            | "RefCounted"
            | "Resource"
            | "ResourceLoader"
            | "ResourceSaver"
            | "SceneTree"
            | "Script"
            | "ScriptExtension"
    )
}

#[cfg(before_api = "4.3")] #[cfg_attr(published_docs, doc(cfg(before_api = "4.3")))]
fn make_inactive_class_check(return_value: TokenStream) -> TokenStream {
    quote! {
        if ::godot::private::is_class_inactive(Self::__config().is_tool) {
            return #return_value;
        }
    }
}

#[cfg(since_api = "4.3")] #[cfg_attr(published_docs, doc(cfg(since_api = "4.3")))]
fn make_inactive_class_check(_return_value: TokenStream) -> TokenStream {
    TokenStream::new()
}

fn make_inner_virtual_method_call(
    is_gd_self: bool,
    is_mut: bool,
    trait_path: &venial::TypeExpr,
) -> (TokenStream, TokenStream, TokenStream) {
    match (is_gd_self, is_mut) {
        (false, true) => (
            quote! {::godot::private::RecvMut},
            quote! {<Self as #trait_path>},
            quote! {&mut *this.recv_self_mut()},
        ),
        (false, false) => (
            quote! {::godot::private::RecvRef},
            quote! {<Self as #trait_path>},
            quote! {& *this.recv_self()},
        ),
        (true, _) => (
            quote! {::godot::private::RecvGdSelf},
            quote! {Self},
            quote! {this.recv_gd()},
        ),
    }
}

fn is_gd_self(attributes: &[venial::Attribute]) -> ParseResult<bool> {
    if let Some(mut parser) = KvParser::parse(attributes, "func")? {
        let has_gd_self = parser.handle_alone("gd_self")?;
        parser.finish()?;
        Ok(has_gd_self)
    } else {
        Ok(false)
    }
}

fn validate_not_gd_self(is_gd_self: bool, method: &venial::Function) -> ParseResult<()> {
    if is_gd_self {
        bail!(
            &method,
            "Method {} can't be used with #[func(gd_self)].",
            method.name
        )
    } else {
        Ok(())
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

struct OverriddenVirtualFn<'a> {
    cfg_attrs: Vec<&'a venial::Attribute>,
    rust_method_name: String,
    /// Path to a pre-defined constant storing a `("_virtual_func", 123456789)` tuple with name and hash of the virtual function.
    ///
    /// Before Godot 4.4, this just stores the name `"_virtual_func"`.
    godot_name_hash_constant: TokenStream,
    signature_info: SignatureInfo,
    before_kind: BeforeKind,
    interface_trait: Option<venial::TypeExpr>,
}

impl OverriddenVirtualFn<'_> {
    fn make_match_arm(&self, class_name: &Ident, trait_base_class: &Ident) -> TokenStream {
        let cfg_attrs = self.cfg_attrs.iter();
        let godot_name_hash_constant = &self.godot_name_hash_constant;

        // Lazily generate code for the actual work (calling user function).
        let method_callback = make_virtual_callback(
            class_name,
            trait_base_class,
            &self.signature_info,
            self.before_kind,
            self.interface_trait.as_ref(),
        );

        quote! {
            #(#cfg_attrs)*
            #godot_name_hash_constant => #method_callback,
        }
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

/// Accumulates various symbols defined inside a `#[godot_api]` macro.
#[derive(Default)]
struct IDecls<'a> {
    godot_init_impl: TokenStream,
    to_string_impl: TokenStream,
    register_class_impl: TokenStream,
    on_notification_impl: TokenStream,
    get_property_impl: TokenStream,
    set_property_impl: TokenStream,
    get_property_list_impl: TokenStream,
    property_get_revert_impl: TokenStream,
    validate_property_impl: TokenStream,

    modifiers: Vec<(Vec<&'a venial::Attribute>, Ident)>,
    overridden_virtuals: Vec<OverriddenVirtualFn<'a>>,
}

impl<'a> IDecls<'a> {
    fn add_modifier(&mut self, cfg_attrs: Vec<&'a venial::Attribute>, modifier: &str) {
        self.modifiers.push((cfg_attrs, ident(modifier)));
    }
}

impl ToTokens for IDecls<'_> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.godot_init_impl.to_tokens(tokens);
        self.to_string_impl.to_tokens(tokens);
        self.on_notification_impl.to_tokens(tokens);
        self.register_class_impl.to_tokens(tokens);
        self.get_property_impl.to_tokens(tokens);
        self.set_property_impl.to_tokens(tokens);
        self.get_property_list_impl.to_tokens(tokens);
        self.property_get_revert_impl.to_tokens(tokens);
        self.validate_property_impl.to_tokens(tokens);
    }
}
