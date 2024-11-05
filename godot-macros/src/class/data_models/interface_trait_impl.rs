/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::class::{into_signature_info, make_virtual_callback, BeforeKind, SignatureInfo};
use crate::{util, ParseResult};

use proc_macro2::TokenStream;
use quote::quote;

/// Codegen for `#[godot_api] impl ISomething for MyType`
pub fn transform_trait_impl(original_impl: venial::Impl) -> ParseResult<TokenStream> {
    let (class_name, trait_path) = util::validate_trait_impl_virtual(&original_impl, "godot_api")?;
    let class_name_obj = util::class_name_obj(&class_name);

    let mut godot_init_impl = TokenStream::new();
    let mut to_string_impl = TokenStream::new();
    let mut register_class_impl = TokenStream::new();
    let mut on_notification_impl = TokenStream::new();
    let mut get_property_impl = TokenStream::new();
    let mut set_property_impl = TokenStream::new();
    let mut get_property_list_impl = TokenStream::new();
    let mut property_get_revert_impl = TokenStream::new();

    let mut register_fn = None;
    let mut create_fn = None;
    let mut recreate_fn = None;
    let mut to_string_fn = None;
    let mut on_notification_fn = None;
    let mut get_property_fn = None;
    let mut set_property_fn = None;
    let mut get_property_list_fn = None;
    let mut free_property_list_fn = None;
    let mut property_get_revert_fn = None;
    let mut property_can_revert_fn = None;

    let mut virtual_methods = vec![];
    let mut virtual_method_cfg_attrs = vec![];
    let mut virtual_method_names = vec![];

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
        let method_name = method.name.to_string();
        match method_name.as_str() {
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

                // Adds a match arm for each implementation of this method, transferring its respective cfg attrs to
                // the corresponding match arm (see explanation for the match after this loop).
                // In principle, the cfg attrs will allow only either 0 or 1 of a function with this name to exist,
                // unless there are duplicate implementations for the same method, which should error anyway.
                // Thus, in any correct program, the match arms (which are, in principle, identical) will be reduced to
                // a single one at most, since we forward the cfg attrs. The idea here is precisely to keep this
                // specific match arm 'alive' if at least one implementation of the method is also kept (hence why all
                // the match arms are identical).
                register_fn = Some(quote! {
                    #register_fn
                    #(#cfg_attrs)*
                    () => Some(#prv::ErasedRegisterFn {
                        raw: #prv::callbacks::register_class_by_builder::<#class_name>
                    }),
                });
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
                create_fn = Some(quote! {
                    #create_fn
                    #(#cfg_attrs)*
                    () => Some(#prv::callbacks::create::<#class_name>),
                });
                if cfg!(since_api = "4.2") {
                    recreate_fn = Some(quote! {
                        #recreate_fn
                        #(#cfg_attrs)*
                        () => Some(#prv::callbacks::recreate::<#class_name>),
                    });
                }
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

                to_string_fn = Some(quote! {
                    #to_string_fn
                    #(#cfg_attrs)*
                    () => Some(#prv::callbacks::to_string::<#class_name>),
                });
            }

            "on_notification" => {
                on_notification_impl = quote! {
                    #on_notification_impl

                    #(#cfg_attrs)*
                    impl ::godot::obj::cap::GodotNotification for #class_name {
                        fn __godot_notification(&mut self, what: i32) {
                            use ::godot::obj::UserClass as _;

                            #[cfg(before_api = "4.3")]
                            if ::godot::private::is_class_inactive(Self::__config().is_tool) {
                                return;
                            }

                            <Self as #trait_path>::on_notification(self, what.into())
                        }
                    }
                };

                on_notification_fn = Some(quote! {
                    #on_notification_fn
                    #(#cfg_attrs)*
                    () => Some(#prv::callbacks::on_notification::<#class_name>),
                });
            }

            "get_property" => {
                get_property_impl = quote! {
                    #(#cfg_attrs)*
                    impl ::godot::obj::cap::GodotGet for #class_name {
                        fn __godot_get_property(&self, property: ::godot::builtin::StringName) -> Option<::godot::builtin::Variant> {
                            use ::godot::obj::UserClass as _;

                            #[cfg(before_api = "4.3")]
                            if ::godot::private::is_class_inactive(Self::__config().is_tool) {
                                return None;
                            }

                            <Self as #trait_path>::get_property(self, property)
                        }
                    }
                };

                get_property_fn = Some(quote! {
                    #(#cfg_attrs)*
                    () => Some(#prv::callbacks::get_property::<#class_name>),
                });
            }

            "set_property" => {
                set_property_impl = quote! {
                    #(#cfg_attrs)*
                    impl ::godot::obj::cap::GodotSet for #class_name {
                        fn __godot_set_property(&mut self, property: ::godot::builtin::StringName, value: ::godot::builtin::Variant) -> bool {
                            use ::godot::obj::UserClass as _;

                            #[cfg(before_api = "4.3")]
                            if ::godot::private::is_class_inactive(Self::__config().is_tool) {
                                return false;
                            }

                            <Self as #trait_path>::set_property(self, property, value)
                        }
                    }
                };

                set_property_fn = Some(quote! {
                    #(#cfg_attrs)*
                    () => Some(#prv::callbacks::set_property::<#class_name>),
                });
            }

            #[cfg(before_api = "4.3")]
            "get_property_list" => {
                get_property_list_impl = quote! {
                    #(#cfg_attrs)*
                    compile_error!("`get_property_list` is only supported for Godot versions of at least 4.3");
                };

                // Set these variables otherwise rust complains that these variables aren't changed in Godot < 4.3.
                get_property_list_fn = None;
                free_property_list_fn = None;
            }

            #[cfg(since_api = "4.3")]
            "get_property_list" => {
                get_property_list_impl = quote! {
                    #(#cfg_attrs)*
                    impl ::godot::obj::cap::GodotGetPropertyList for #class_name {
                        fn __godot_get_property_list(&mut self) -> Vec<::godot::meta::PropertyInfo> {
                            // `get_property_list` is only supported in Godot API >= 4.3. If we add support for `get_property_list` to earlier
                            // versions of Godot then this code is still needed and should be uncommented.
                            //
                            // use ::godot::obj::UserClass as _;
                            //
                            // #[cfg(before_api = "4.3")]
                            // if ::godot::private::is_class_inactive(Self::__config().is_tool) {
                            //     return false;
                            // }

                            <Self as #trait_path>::get_property_list(self)
                        }
                    }
                };

                get_property_list_fn = Some(quote! {
                    #(#cfg_attrs)*
                    () => Some(#prv::callbacks::get_property_list::<#class_name>),
                });
                free_property_list_fn = Some(quote! {
                    #(#cfg_attrs)*
                    () => Some(#prv::callbacks::free_property_list::<#class_name>),
                });
            }

            "property_get_revert" => {
                property_get_revert_impl = quote! {
                    #(#cfg_attrs)*
                    impl ::godot::obj::cap::GodotPropertyGetRevert for #class_name {
                        fn __godot_property_get_revert(&self, property: StringName) -> Option<::godot::builtin::Variant> {
                            use ::godot::obj::UserClass as _;

                            #[cfg(before_api = "4.3")]
                            if ::godot::private::is_class_inactive(Self::__config().is_tool) {
                                return None;
                            }

                            <Self as #trait_path>::property_get_revert(self, property)
                        }
                    }
                };

                property_get_revert_fn = Some(quote! {
                    #(#cfg_attrs)*
                    () => Some(#prv::callbacks::property_get_revert::<#class_name>),
                });

                property_can_revert_fn = Some(quote! {
                    #(#cfg_attrs)*
                    () => Some(#prv::callbacks::property_can_revert::<#class_name>),
                });
            }

            // Other virtual methods, like ready, process etc.
            _ => {
                let method = util::reduce_to_signature(method);

                // Godot-facing name begins with underscore
                //
                // Note: godot-codegen special-cases the virtual
                // method called _init (which exists on a handful of
                // classes, distinct from the default constructor) to
                // init_ext, to avoid Rust-side ambiguity. See
                // godot_codegen::class_generator::virtual_method_name.
                let virtual_method_name = if method_name == "init_ext" {
                    String::from("_init")
                } else {
                    format!("_{method_name}")
                };

                let signature_info = into_signature_info(method, &class_name, false);

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
                virtual_method_cfg_attrs.push(cfg_attrs);
                virtual_method_names.push(virtual_method_name);
                virtual_methods.push((signature_info, before_kind));
            }
        }
    }

    // If there is no ready() method explicitly overridden, we need to add one, to ensure that __before_ready() is called to
    // initialize the OnReady fields.
    if !virtual_methods
        .iter()
        .any(|(sig, _)| sig.method_name == "ready")
    {
        let signature_info = SignatureInfo::fn_ready();

        virtual_method_cfg_attrs.push(vec![]);
        virtual_method_names.push("_ready".to_string());
        virtual_methods.push((signature_info, BeforeKind::OnlyBefore));
    }

    let tool_check = util::make_virtual_tool_check();
    let virtual_method_callbacks: Vec<TokenStream> = virtual_methods
        .into_iter()
        .map(|(signature_info, before_kind)| {
            make_virtual_callback(&class_name, signature_info, before_kind)
        })
        .collect();

    // Use 'match' as a way to only emit 'Some(...)' if the given cfg attrs allow.
    // This permits users to conditionally remove virtual method impls from compilation while also removing their FFI
    // glue which would otherwise make them visible to Godot even if not really implemented.
    // Needs '#[allow(unreachable_patterns)]' to avoid warnings about the last match arm.
    // Also requires '#[allow(clippy::match_single_binding)]' for similar reasons.
    let register_fn = convert_to_match_expression_or_none(register_fn);
    let create_fn = convert_to_match_expression_or_none(create_fn);
    let recreate_fn = convert_to_match_expression_or_none(recreate_fn);
    let to_string_fn = convert_to_match_expression_or_none(to_string_fn);
    let on_notification_fn = convert_to_match_expression_or_none(on_notification_fn);
    let get_property_fn = convert_to_match_expression_or_none(get_property_fn);
    let set_property_fn = convert_to_match_expression_or_none(set_property_fn);
    let get_property_list_fn = convert_to_match_expression_or_none(get_property_list_fn);
    let free_property_list_fn = convert_to_match_expression_or_none(free_property_list_fn);
    let property_get_revert_fn = convert_to_match_expression_or_none(property_get_revert_fn);
    let property_can_revert_fn = convert_to_match_expression_or_none(property_can_revert_fn);

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

        impl ::godot::private::You_forgot_the_attribute__godot_api for #class_name {}

        impl ::godot::obj::cap::ImplementsGodotVirtual for #class_name {
            fn __virtual_call(name: &str) -> ::godot::sys::GDExtensionClassCallVirtual {
                //println!("virtual_call: {}.{}", std::any::type_name::<Self>(), name);
                use ::godot::obj::UserClass as _;
                #tool_check

                match name {
                    #(
                       #(#virtual_method_cfg_attrs)*
                       #virtual_method_names => #virtual_method_callbacks,
                    )*
                    _ => None,
                }
            }
        }

        ::godot::sys::plugin_add!(__GODOT_PLUGIN_REGISTRY in #prv; #prv::ClassPlugin {
            class_name: #class_name_obj,
            item: #prv::PluginItem::ITraitImpl {
                user_register_fn: #register_fn,
                user_create_fn: #create_fn,
                user_recreate_fn: #recreate_fn,
                user_to_string_fn: #to_string_fn,
                user_on_notification_fn: #on_notification_fn,
                user_set_fn: #set_property_fn,
                user_get_fn: #get_property_fn,
                user_get_property_list_fn: #get_property_list_fn,
                user_free_property_list_fn: #free_property_list_fn,
                user_property_get_revert_fn: #property_get_revert_fn,
                user_property_can_revert_fn: #property_can_revert_fn,
                get_virtual_fn: #prv::callbacks::get_virtual::<#class_name>,
                #docs
            },
            init_level: <#class_name as ::godot::obj::GodotClass>::INIT_LEVEL,
        });
    };

    Ok(result)
}

/// Expects either Some(quote! { () => A, () => B, ... }) or None as the 'tokens' parameter.
/// The idea is that the () => ... arms can be annotated by cfg attrs, so, if any of them compiles (and assuming the cfg
/// attrs only allow one arm to 'survive' compilation), their return value (Some(...)) will be prioritized over the
/// 'None' from the catch-all arm at the end. If, however, none of them compile, then None is returned from the last
/// match arm.
fn convert_to_match_expression_or_none(tokens: Option<TokenStream>) -> TokenStream {
    if let Some(tokens) = tokens {
        quote! {
            {
                // When one of the () => ... arms is present, the last arm intentionally won't ever match.
                #[allow(unreachable_patterns)]
                // Don't warn when only _ => None is present as all () => ... arms were removed from compilation.
                #[allow(clippy::match_single_binding)]
                match () {
                    #tokens
                    _ => None,
                }
            }
        }
    } else {
        quote! { None }
    }
}
