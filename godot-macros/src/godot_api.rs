/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::util;
use crate::util::bail;
use proc_macro2::{Ident, TokenStream};
use quote::quote;
use venial::{AttributeValue, Declaration, Error, FnParam, Function, Impl, ImplMember, TyExpr};

pub fn transform(input_decl: Declaration) -> Result<TokenStream, Error> {
    let decl = match input_decl {
        Declaration::Impl(decl) => decl,
        _ => bail(
            "#[godot_api] can only be applied on impl blocks",
            input_decl,
        )?,
    };

    if decl.impl_generic_params.is_some() {
        bail(
            "#[godot_api] currently does not support generic parameters",
            &decl,
        )?;
    }

    if decl.self_ty.as_path().is_none() {
        return bail("invalid Self type for #[godot_api] impl", decl);
    };

    if decl.trait_ty.is_some() {
        transform_trait_impl(decl)
    } else {
        transform_inherent_impl(decl)
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

/// Attribute for user-declared function
enum BoundAttrType {
    Func(AttributeValue),
    Signal(AttributeValue),
}

struct BoundAttr {
    attr_name: Ident,
    index: usize,
    ty: BoundAttrType,
}

impl BoundAttr {
    fn bail<R>(self, msg: &str, method: &Function) -> Result<R, Error> {
        bail(format!("#[{}]: {}", self.attr_name, msg), &method.name)
    }
}

/// Codegen for `#[godot_api] impl MyType`
fn transform_inherent_impl(mut decl: Impl) -> Result<TokenStream, Error> {
    let class_name = util::validate_impl(&decl, None, "godot_api")?;
    let class_name_str = class_name.to_string();

    let (funcs, signals) = process_godot_fns(&mut decl)?;

    let mut signal_name_strs: Vec<String> = Vec::new();
    let mut signal_parameters_count: Vec<i64> = Vec::new();
    let mut signal_parameters: Vec<TokenStream> = Vec::new();

    for signature in signals {
        let mut param_types: Vec<TyExpr> = Vec::new();
        let mut param_names: Vec<Ident> = Vec::new();

        for param in signature.params.inner {
            match &param.0 {
                FnParam::Typed(param) => {
                    param_types.push(param.ty.clone());
                    param_names.push(param.name.clone());
                }
                FnParam::Receiver(_) => {}
            };
        }

        signal_name_strs.push(signature.name.to_string());
        signal_parameters_count.push(param_names.len() as i64);
        signal_parameters.push(
            quote! {
                ::godot::private::gdext_get_arguments_info!(((), #(#param_types ),*), #(#param_names, )*).as_ptr()
            },
        );
    }

    let prv = quote! { ::godot::private };

    let result = quote! {
        #decl

        impl ::godot::obj::cap::ImplementsGodotApi for #class_name {
            fn __register_methods() {
                #(
                    ::godot::private::gdext_register_method!(#class_name, #funcs);
                )*

                unsafe {
                    let class_name = ::godot::builtin::StringName::from(#class_name_str);
                    use ::godot::sys;

                    #(
                        let parameters = #signal_parameters;
                        let signal_name = ::godot::builtin::StringName::from(#signal_name_strs);

                        sys::interface_fn!(classdb_register_extension_class_signal)(
                            sys::get_library(),
                            class_name.string_sys(),
                            signal_name.string_sys(),
                            parameters,
                            sys::GDExtensionInt::from(#signal_parameters_count),
                        );
                    )*
                }
            }
        }

        ::godot::sys::plugin_add!(__GODOT_PLUGIN_REGISTRY in #prv; #prv::ClassPlugin {
            class_name: #class_name_str,
            component: #prv::PluginComponent::UserMethodBinds {
                generated_register_fn: #prv::ErasedRegisterFn {
                    raw: #prv::callbacks::register_user_binds::<#class_name>,
                },
            },
        });
    };

    Ok(result)
}

fn process_godot_fns(decl: &mut Impl) -> Result<(Vec<Function>, Vec<Function>), Error> {
    let mut func_signatures = vec![];
    let mut signal_signatures = vec![];

    let mut removed_indexes = vec![];
    for (index, item) in decl.body_items.iter_mut().enumerate() {
        let method = if let ImplMember::Method(method) = item {
            method
        } else {
            continue;
        };

        if let Some(attr) = extract_attributes(method)? {
            // Remaining code no longer has attribute -- rest stays
            method.attributes.remove(attr.index);

            if method.qualifiers.tk_default.is_some()
                || method.qualifiers.tk_const.is_some()
                || method.qualifiers.tk_async.is_some()
                || method.qualifiers.tk_unsafe.is_some()
                || method.qualifiers.tk_extern.is_some()
                || method.qualifiers.extern_abi.is_some()
            {
                return attr.bail("fn qualifiers are not allowed", method);
            }

            if method.generic_params.is_some() {
                return attr.bail("generic fn parameters are not supported", method);
            }

            match attr.ty {
                BoundAttrType::Func(_attr) => {
                    // Signatures are the same thing without body
                    let sig = util::reduce_to_signature(method);
                    func_signatures.push(sig);
                }
                BoundAttrType::Signal(ref _attr_val) => {
                    if method.return_ty.is_some() {
                        return attr.bail("return types are not supported", method);
                    }
                    let sig = util::reduce_to_signature(method);

                    signal_signatures.push(sig.clone());
                    removed_indexes.push(index);
                }
            }
        }
    }

    // Remove some elements (e.g. signals) from impl.
    // O(n^2); alternative: retain(), but elements themselves don't have the necessary information.
    for index in removed_indexes.into_iter().rev() {
        decl.body_items.remove(index);
    }

    Ok((func_signatures, signal_signatures))
}

fn extract_attributes(method: &Function) -> Result<Option<BoundAttr>, Error> {
    let mut found = None;
    for (index, attr) in method.attributes.iter().enumerate() {
        let attr_name = attr
            .get_single_path_segment()
            .expect("get_single_path_segment");

        // Note: can't use match without constructing new string, because ident
        let new_found = if attr_name == "func" {
            Some(BoundAttr {
                attr_name: attr_name.clone(),
                index,
                ty: BoundAttrType::Func(attr.value.clone()),
            })
        } else if attr_name == "signal" {
            // TODO once parameters are supported, this should probably be moved to the struct definition
            // E.g. a zero-sized type Signal<(i32, String)> with a provided emit(i32, String) method
            // This could even be made public (callable on the struct obj itself)
            Some(BoundAttr {
                attr_name: attr_name.clone(),
                index,
                ty: BoundAttrType::Signal(attr.value.clone()),
            })
        } else {
            None
        };

        // Validate at most 1 attribute
        if found.is_some() && new_found.is_some() {
            bail(
                "at most one #[func] or #[signal] attribute per method allowed",
                &method.name,
            )?;
        }

        found = new_found;
    }

    Ok(found)
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

/// Codegen for `#[godot_api] impl GodotExt for MyType`
fn transform_trait_impl(original_impl: Impl) -> Result<TokenStream, Error> {
    let (class_name, trait_name) = util::validate_trait_impl_virtual(&original_impl, "godot_api")?;
    let class_name_str = class_name.to_string();

    let mut godot_init_impl = TokenStream::new();
    let mut to_string_impl = TokenStream::new();
    let mut register_class_impl = TokenStream::new();
    let mut on_notification_impl = TokenStream::new();

    let mut register_fn = quote! { None };
    let mut create_fn = quote! { None };
    let mut to_string_fn = quote! { None };
    let mut on_notification_fn = quote! { None };

    let mut virtual_methods = vec![];
    let mut virtual_method_names = vec![];

    let prv = quote! { ::godot::private };

    for item in original_impl.body_items.iter() {
        let method = if let ImplMember::Method(f) = item {
            f
        } else {
            continue;
        };

        let method_name = method.name.to_string();
        match method_name.as_str() {
            "register_class" => {
                register_class_impl = quote! {
                    impl ::godot::obj::cap::GodotRegisterClass for #class_name {
                        fn __godot_register_class(builder: &mut ::godot::builder::GodotBuilder<Self>) {
                            <Self as #trait_name>::register_class(builder)
                        }
                    }
                };

                register_fn = quote! { Some(#prv::ErasedRegisterFn {
                    raw: #prv::callbacks::register_class_by_builder::<#class_name>
                }) };
            }

            "init" => {
                godot_init_impl = quote! {
                    impl ::godot::obj::cap::GodotInit for #class_name {
                        fn __godot_init(base: ::godot::obj::Base<Self::Base>) -> Self {
                            <Self as #trait_name>::init(base)
                        }
                    }
                };
                create_fn = quote! { Some(#prv::callbacks::create::<#class_name>) };
            }

            "to_string" => {
                to_string_impl = quote! {
                    impl ::godot::obj::cap::GodotToString for #class_name {
                        fn __godot_to_string(&self) -> ::godot::builtin::GodotString {
                            <Self as #trait_name>::to_string(self)
                        }
                    }
                };

                to_string_fn = quote! { Some(#prv::callbacks::to_string::<#class_name>) };
            }

            "on_notification" => {
                on_notification_impl = quote! {
                    impl ::godot::obj::cap::GodotNotification for #class_name {
                        fn __godot_notification(&mut self, what: i32) {
                            <Self as #trait_name>::on_notification(self, what.into())
                        }
                    }
                };

                on_notification_fn = quote! {
                    Some(#prv::callbacks::on_notification::<#class_name>)
                };
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
                virtual_method_names.push(virtual_method_name);
                virtual_methods.push(method);
            }
        }
    }

    let result = quote! {
        #original_impl
        #godot_init_impl
        #to_string_impl
        #on_notification_impl
        #register_class_impl

        impl ::godot::private::You_forgot_the_attribute__godot_api for #class_name {}

        impl ::godot::obj::cap::ImplementsGodotVirtual for #class_name {
            fn __virtual_call(name: &str) -> ::godot::sys::GDExtensionClassCallVirtual {
                //println!("virtual_call: {}.{}", std::any::type_name::<Self>(), name);

                match name {
                    #(
                       #virtual_method_names => #prv::gdext_virtual_method_callback!(#class_name, #virtual_methods),
                    )*
                    _ => None,
                }
            }
        }

        ::godot::sys::plugin_add!(__GODOT_PLUGIN_REGISTRY in #prv; #prv::ClassPlugin {
            class_name: #class_name_str,
            component: #prv::PluginComponent::UserVirtuals {
                user_register_fn: #register_fn,
                user_create_fn: #create_fn,
                user_to_string_fn: #to_string_fn,
                user_on_notification_fn: #on_notification_fn,
                get_virtual_fn: #prv::callbacks::get_virtual::<#class_name>,
            },
        });
    };

    Ok(result)
}
