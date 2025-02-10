/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::class::{into_signature_info, make_virtual_callback, BeforeKind, SignatureInfo};
use crate::util::ident;
use crate::{util, ParseResult};

use proc_macro2::{Ident, TokenStream};
use quote::quote;

/// Codegen for `#[godot_api] impl ISomething for MyType`
pub fn transform_trait_impl(original_impl: venial::Impl) -> ParseResult<TokenStream> {
    let (class_name, trait_path, trait_base_class) =
        util::validate_trait_impl_virtual(&original_impl, "godot_api")?;

    let mut godot_init_impl = TokenStream::new();
    let mut to_string_impl = TokenStream::new();
    let mut register_class_impl = TokenStream::new();
    let mut on_notification_impl = TokenStream::new();
    let mut get_property_impl = TokenStream::new();
    let mut set_property_impl = TokenStream::new();
    let mut get_property_list_impl = TokenStream::new();
    let mut property_get_revert_impl = TokenStream::new();
    let mut validate_property_impl = TokenStream::new();

    let mut modifiers = Vec::new();

    let mut overridden_virtuals = vec![];

    let prv = quote! { ::godot::private };

    #[cfg(all(feature = "register-docs", since_api = "4.3"))]
    let docs = crate::docs::make_virtual_impl_docs(&original_impl.body_items);
    #[cfg(not(all(feature = "register-docs", since_api = "4.3")))]
    let docs = quote! {};

    for item in original_impl.body_items.iter() {
        let method = if let venial::ImplMember::AssocFunction(f) = item {
            f
        } else {
            continue;
        };

        // Transport #[cfg] attributes to the virtual method's FFI glue, to ensure it won't be
        // registered in Godot if conditionally removed from compilation.
        let cfg_attrs = util::extract_cfg_attrs(&method.attributes)
            .into_iter()
            .collect::<Vec<_>>();

        let method_name_str = method.name.to_string();
        match method_name_str.as_str() {
            "register_class" => {
                // Implements the trait once for each implementation of this method, forwarding the cfg attrs of each
                // implementation to the generated trait impl. If the cfg attrs allow for multiple implementations of
                // this method to exist, then Rust will generate an error, so we don't have to worry about the multiple
                // trait implementations actually generating an error, since that can only happen if multiple
                // implementations of the same method are kept by #[cfg] (due to user error).
                // Thus, by implementing the trait once for each possible implementation of this method (depending on
                // what #[cfg] allows), forwarding the cfg attrs, we ensure this trait impl will remain in the code if
                // at least one of the method impls are kept.
                register_class_impl = quote! {
                    #register_class_impl

                    #(#cfg_attrs)*
                    impl ::godot::obj::cap::GodotRegisterClass for #class_name {
                        fn __godot_register_class(builder: &mut ::godot::builder::GodotBuilder<Self>) {
                            <Self as #trait_path>::register_class(builder)
                        }
                    }
                };

                modifiers.push((cfg_attrs, ident("with_register")));
            }

            "init" => {
                godot_init_impl = quote! {
                    #godot_init_impl

                    #(#cfg_attrs)*
                    impl ::godot::obj::cap::GodotDefault for #class_name {
                        fn __godot_user_init(base: ::godot::obj::Base<Self::Base>) -> Self {
                            <Self as #trait_path>::init(base)
                        }
                    }
                };

                modifiers.push((cfg_attrs, ident("with_create")));
            }

            "to_string" => {
                to_string_impl = quote! {
                    #to_string_impl

                    #(#cfg_attrs)*
                    impl ::godot::obj::cap::GodotToString for #class_name {
                        fn __godot_to_string(&self) -> ::godot::builtin::GString {
                            <Self as #trait_path>::to_string(self)
                        }
                    }
                };

                modifiers.push((cfg_attrs, ident("with_string")));
            }

            "on_notification" => {
                let inactive_class_early_return = make_inactive_class_check(TokenStream::new());
                on_notification_impl = quote! {
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

                modifiers.push((cfg_attrs, ident("with_on_notification")));
            }

            "get_property" => {
                let inactive_class_early_return = make_inactive_class_check(quote! { None });
                get_property_impl = quote! {
                    #(#cfg_attrs)*
                    impl ::godot::obj::cap::GodotGet for #class_name {
                        fn __godot_get_property(&self, property: ::godot::builtin::StringName) -> Option<::godot::builtin::Variant> {
                            use ::godot::obj::UserClass as _;

                            #inactive_class_early_return

                            <Self as #trait_path>::get_property(self, property)
                        }
                    }
                };

                modifiers.push((cfg_attrs, ident("with_get_property")));
            }

            "set_property" => {
                let inactive_class_early_return = make_inactive_class_check(quote! { false });
                set_property_impl = quote! {
                    #(#cfg_attrs)*
                    impl ::godot::obj::cap::GodotSet for #class_name {
                        fn __godot_set_property(&mut self, property: ::godot::builtin::StringName, value: ::godot::builtin::Variant) -> bool {
                            use ::godot::obj::UserClass as _;

                            #inactive_class_early_return

                            <Self as #trait_path>::set_property(self, property, value)
                        }
                    }
                };

                modifiers.push((cfg_attrs, ident("with_set_property")));
            }

            #[cfg(before_api = "4.3")]
            "get_property_list" => {
                get_property_list_impl = quote! {
                    #(#cfg_attrs)*
                    compile_error!("`get_property_list` is only supported for Godot versions of at least 4.3");
                };
            }

            #[cfg(since_api = "4.3")]
            "get_property_list" => {
                // `get_property_list` is only supported in Godot API >= 4.3. If we add support for `get_property_list` to earlier
                // versions of Godot then this code is still needed and should be uncommented.
                //
                // let inactive_class_early_return = make_inactive_class_check(false);
                get_property_list_impl = quote! {
                    #(#cfg_attrs)*
                    impl ::godot::obj::cap::GodotGetPropertyList for #class_name {
                        fn __godot_get_property_list(&mut self) -> Vec<::godot::meta::PropertyInfo> {
                            // #inactive_class_early_return

                            <Self as #trait_path>::get_property_list(self)
                        }
                    }
                };

                modifiers.push((cfg_attrs, ident("with_get_property_list")));
            }

            "property_get_revert" => {
                let inactive_class_early_return = make_inactive_class_check(quote! { None });
                property_get_revert_impl = quote! {
                    #(#cfg_attrs)*
                    impl ::godot::obj::cap::GodotPropertyGetRevert for #class_name {
                        fn __godot_property_get_revert(&self, property: StringName) -> Option<::godot::builtin::Variant> {
                            use ::godot::obj::UserClass as _;

                            #inactive_class_early_return

                            <Self as #trait_path>::property_get_revert(self, property)
                        }
                    }
                };

                modifiers.push((cfg_attrs, ident("with_property_get_revert")));
            }

            #[cfg(since_api = "4.2")]
            "validate_property" => {
                let inactive_class_early_return = make_inactive_class_check(TokenStream::new());
                validate_property_impl = quote! {
                    #(#cfg_attrs)*
                    impl ::godot::obj::cap::GodotValidateProperty for #class_name {
                        fn __godot_validate_property(&self, property: &mut ::godot::meta::PropertyInfo) {
                            use ::godot::obj::UserClass as _;

                            #inactive_class_early_return

                            <Self as #trait_path>::validate_property(self, property);
                        }
                    }
                };
                modifiers.push((cfg_attrs, ident("with_validate_property")));
            }

            // Other virtual methods, like ready, process etc.
            method_name_str => {
                #[cfg(since_api = "4.4")]
                let method_name_ident = method.name.clone();
                let method = util::reduce_to_signature(method);

                // Godot-facing name begins with underscore.
                //
                // godot-codegen special-cases the virtual method called _init (which exists on a handful of classes, distinct from the default
                // constructor) to init_ext, to avoid Rust-side ambiguity. See godot_codegen::class_generator::virtual_method_name.
                let virtual_method_name = if method_name_str == "init_ext" {
                    String::from("_init")
                } else {
                    format!("_{method_name_str}")
                };

                let signature_info = into_signature_info(method, &class_name, false);

                // Overridden ready() methods additionally have an additional `__before_ready()` call (for OnReady inits).
                let before_kind = if method_name_str == "ready" {
                    BeforeKind::WithBefore
                } else {
                    BeforeKind::Without
                };

                // Note that, if the same method is implemented multiple times (with different cfg attr combinations),
                // then there will be multiple match arms annotated with the same cfg attr combinations, thus they will
                // be reduced to just one arm (at most, if the implementations aren't all removed from compilation) for
                // each distinct method.
                overridden_virtuals.push(OverriddenVirtualFn {
                    cfg_attrs,
                    method_name: virtual_method_name,
                    // If ever the `I*` verbatim validation is relaxed (it won't work with use-renames or other weird edge cases), the approach
                    // with known_virtual_hashes module could be changed to something like the following (GodotBase = nearest Godot base class):
                    // __get_virtual_hash::<Self::GodotBase>("method")
                    #[cfg(since_api = "4.4")]
                    hash_constant: quote! { hashes::#method_name_ident },
                    signature_info,
                    before_kind,
                    interface_trait: Some(trait_path.clone()),
                });
            }
        }
    }

    // If there is no ready() method explicitly overridden, we need to add one, to ensure that __before_ready() is called to
    // initialize the OnReady fields.
    if is_possibly_node_class(&trait_base_class)
        && !overridden_virtuals
            .iter()
            .any(|v| v.method_name == "_ready")
    {
        let match_arm = OverriddenVirtualFn {
            cfg_attrs: vec![],
            method_name: "_ready".to_string(),
            // Can't use `hashes::ready` here, as the base class might not be `Node` (see above why such a branch is still added).
            #[cfg(since_api = "4.4")]
            hash_constant: quote! { ::godot::sys::known_virtual_hashes::Node::ready },
            signature_info: SignatureInfo::fn_ready(),
            before_kind: BeforeKind::OnlyBefore,
            interface_trait: None,
        };

        overridden_virtuals.push(match_arm);
    }

    let tool_check = util::make_virtual_tool_check();

    let modifications = modifiers.into_iter().map(|(cfg_attrs, modifier)| {
        quote! {
            #(#cfg_attrs)*
            { item = item.#modifier::<#class_name>(); }
        }
    });

    let item_constructor = quote! {
        {
            let mut item = #prv::ITraitImpl::new::<#class_name>(#docs);
            #(#modifications)*
            item
        }
    };

    // See also __default_virtual_call() codegen.
    let (hash_param, hashes_use, match_expr);
    if cfg!(since_api = "4.4") {
        hash_param = quote! { hash: u32, };
        hashes_use =
            quote! { use ::godot::sys::known_virtual_hashes::#trait_base_class as hashes; };
        match_expr = quote! { (name, hash) };
    } else {
        hash_param = TokenStream::new();
        hashes_use = TokenStream::new();
        match_expr = quote! { name };
    };

    let virtual_match_arms = overridden_virtuals
        .iter()
        .map(|v| v.make_match_arm(&class_name));

    let result = quote! {
        #original_impl
        #godot_init_impl
        #to_string_impl
        #on_notification_impl
        #register_class_impl
        #get_property_impl
        #set_property_impl
        #get_property_list_impl
        #property_get_revert_impl
        #validate_property_impl

        impl ::godot::private::You_forgot_the_attribute__godot_api for #class_name {}

        impl ::godot::obj::cap::ImplementsGodotVirtual for #class_name {
            fn __virtual_call(name: &str, #hash_param) -> ::godot::sys::GDExtensionClassCallVirtual {
                //println!("virtual_call: {}.{}", std::any::type_name::<Self>(), name);
                use ::godot::obj::UserClass as _;
                #tool_check

                #hashes_use
                match #match_expr {
                    #( #virtual_match_arms )*
                    _ => None,
                }
            }
        }

        ::godot::sys::plugin_add!(__GODOT_PLUGIN_REGISTRY in #prv; #prv::ClassPlugin::new::<#class_name>(
            #prv::PluginItem::ITraitImpl(#item_constructor)
        ));
    };

    Ok(result)
}

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
struct OverriddenVirtualFn<'a> {
    cfg_attrs: Vec<&'a venial::Attribute>,
    method_name: String,
    #[cfg(since_api = "4.4")]
    hash_constant: TokenStream,
    signature_info: SignatureInfo,
    before_kind: BeforeKind,
    interface_trait: Option<venial::TypeExpr>,
}

impl OverriddenVirtualFn<'_> {
    fn make_match_arm(&self, class_name: &Ident) -> TokenStream {
        let cfg_attrs = self.cfg_attrs.iter();
        let method_name_str = self.method_name.as_str();

        #[cfg(since_api = "4.4")]
        let pattern = {
            let hash_constant = &self.hash_constant;
            quote! { (#method_name_str, #hash_constant) }
        };

        #[cfg(before_api = "4.4")]
        let pattern = method_name_str;

        // Lazily generate code for the actual work (calling user function).
        let method_callback = make_virtual_callback(
            class_name,
            &self.signature_info,
            self.before_kind,
            self.interface_trait.as_ref(),
        );

        quote! {
            #(#cfg_attrs)*
            #pattern => #method_callback,
        }
    }
}

#[cfg(before_api = "4.3")]
fn make_inactive_class_check(return_value: TokenStream) -> TokenStream {
    quote! {
        if ::godot::private::is_class_inactive(Self::__config().is_tool) {
            return #return_value;
        }
    }
}

#[cfg(since_api = "4.3")]
fn make_inactive_class_check(_return_value: TokenStream) -> TokenStream {
    TokenStream::new()
}
