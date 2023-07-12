/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::method_registration::gdext_virtual_method_callback;
use crate::method_registration::make_method_registration;
use crate::util;
use crate::util::bail;
use proc_macro2::{Ident, TokenStream};
use quote::quote;
use quote::spanned::Spanned;
use venial::{
    Attribute, AttributeValue, Constant, Declaration, Error, FnParam, Function, Impl, ImplMember,
    TyExpr,
};

pub fn transform(input_decl: Declaration) -> Result<TokenStream, Error> {
    let decl = match input_decl {
        Declaration::Impl(decl) => decl,
        _ => bail!(
            input_decl,
            "#[godot_api] can only be applied on impl blocks",
        )?,
    };

    if decl.impl_generic_params.is_some() {
        bail!(
            &decl,
            "#[godot_api] currently does not support generic parameters",
        )?;
    }

    if decl.self_ty.as_path().is_none() {
        return bail!(decl, "invalid Self type for #[godot_api] impl");
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
    Const(AttributeValue),
}

struct BoundAttr {
    attr_name: Ident,
    index: usize,
    ty: BoundAttrType,
}

impl BoundAttr {
    fn bail<R>(self, msg: &str, method: &Function) -> Result<R, Error> {
        bail!(&method.name, "#[{}]: {}", self.attr_name, msg)
    }
}

/// Codegen for `#[godot_api] impl MyType`
fn transform_inherent_impl(mut decl: Impl) -> Result<TokenStream, Error> {
    let class_name = util::validate_impl(&decl, None, "godot_api")?;
    let class_name_str = class_name.to_string();

    let (funcs, signals) = process_godot_fns(&mut decl)?;

    let mut signal_name_strs: Vec<String> = Vec::new();
    let mut signal_parameters_count: Vec<usize> = Vec::new();
    let mut signal_parameters: Vec<TokenStream> = Vec::new();

    for signature in signals {
        let mut param_types: Vec<TyExpr> = Vec::new();
        let mut param_names: Vec<String> = Vec::new();

        for param in signature.params.inner {
            match &param.0 {
                FnParam::Typed(param) => {
                    param_types.push(param.ty.clone());
                    param_names.push(param.name.to_string());
                }
                FnParam::Receiver(_) => {}
            };
        }

        let signature_tuple = util::make_signature_tuple_type(&quote! { () }, &param_types);
        let indexes = 0..param_types.len();
        let param_array_decl = quote! {
            [
                // Don't use raw sys pointers directly, very easy to have objects going out of scope.
                #(
                    <#signature_tuple as godot::builtin::meta::VarcallSignatureTuple>
                        ::param_property_info(#indexes, #param_names),
                )*
            ]
        };

        signal_name_strs.push(signature.name.to_string());
        signal_parameters_count.push(param_names.len());
        signal_parameters.push(param_array_decl);
    }

    let prv = quote! { ::godot::private };

    let methods_registration = funcs
        .into_iter()
        .map(|func| make_method_registration(&class_name, func));

    let consts = process_godot_constants(&mut decl)?;
    let mut integer_constant_names = Vec::new();
    let mut integer_constant_values = Vec::new();

    for constant in consts.iter() {
        if constant.initializer.is_none() {
            return bail!(constant, "exported const should have initializer");
        };

        let name = &constant.name;

        integer_constant_names.push(constant.name.to_string());
        integer_constant_values.push(quote! { #class_name::#name });
    }

    let register_constants = if !integer_constant_names.is_empty() {
        quote! {
            use ::godot::builtin::meta::registration::constant::*;
            use ::godot::builtin::meta::ClassName;
            use ::godot::builtin::StringName;

            #(
                ExportConstant::new(
                    ClassName::of::<#class_name>(),
                    ConstantKind::Integer(
                        IntegerConstant::new(
                            StringName::from(#integer_constant_names),
                            #integer_constant_values
                        )
                    )
                ).register();
            )*
        }
    } else {
        quote! {}
    };

    let result = quote! {
        #decl

        impl ::godot::obj::cap::ImplementsGodotApi for #class_name {
            fn __register_methods() {
                #(
                    #methods_registration
                )*

                unsafe {
                    let class_name = ::godot::builtin::StringName::from(#class_name_str);
                    use ::godot::sys;

                    #(
                        let parameters_info: [::godot::builtin::meta::PropertyInfo; #signal_parameters_count] = #signal_parameters;

                        let mut parameters_info_sys: [::godot::sys::GDExtensionPropertyInfo; #signal_parameters_count] =
                            std::array::from_fn(|i| parameters_info[i].property_sys());

                        let signal_name = ::godot::builtin::StringName::from(#signal_name_strs);

                        sys::interface_fn!(classdb_register_extension_class_signal)(
                            sys::get_library(),
                            class_name.string_sys(),
                            signal_name.string_sys(),
                            parameters_info_sys.as_ptr(),
                            sys::GDExtensionInt::from(#signal_parameters_count as i64),
                        );
                    )*
                }
            }

            fn __register_constants() {
                #register_constants
        }
        }

        impl ::godot::private::Cannot_export_without_godot_api_impl for #class_name {}

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

        if let Some(attr) = extract_attributes(&method, &method.attributes)? {
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
                BoundAttrType::Const(_) => {
                    return attr.bail(
                        "#[constant] can only be used on associated cosntant",
                        method,
                    )
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

fn process_godot_constants(decl: &mut Impl) -> Result<Vec<Constant>, Error> {
    let mut constant_signatures = vec![];

    for item in decl.body_items.iter_mut() {
        let ImplMember::Constant(constant) = item else {
            continue;
        };

        if let Some(attr) = extract_attributes(&constant, &constant.attributes)? {
            // Remaining code no longer has attribute -- rest stays
            constant.attributes.remove(attr.index);

            match attr.ty {
                BoundAttrType::Func(_) => {
                    return bail!(constant, "#[func] can only be used on functions")
                }
                BoundAttrType::Signal(_) => {
                    return bail!(constant, "#[signal] can only be used on functions")
                }
                BoundAttrType::Const(_) => {
                    if constant.initializer.is_none() {
                        return bail!(constant, "exported constant must have initializer");
                    }
                    constant_signatures.push(constant.clone());
                }
            }
        }
    }

    Ok(constant_signatures)
}

fn extract_attributes<T>(
    error_scope: T,
    attributes: &[Attribute],
) -> Result<Option<BoundAttr>, Error>
where
    for<'a> &'a T: Spanned,
{
    let mut found = None;
    for (index, attr) in attributes.iter().enumerate() {
        let attr_name = attr
            .get_single_path_segment()
            .expect("get_single_path_segment");

        let new_found = match attr_name {
            name if name == "func" => Some(BoundAttr {
                attr_name: attr_name.clone(),
                index,
                ty: BoundAttrType::Func(attr.value.clone()),
            }),
            name if name == "signal" => {
                // TODO once parameters are supported, this should probably be moved to the struct definition
                // E.g. a zero-sized type Signal<(i32, String)> with a provided emit(i32, String) method
                // This could even be made public (callable on the struct obj itself)
                Some(BoundAttr {
                    attr_name: attr_name.clone(),
                    index,
                    ty: BoundAttrType::Signal(attr.value.clone()),
                })
            }
            name if name == "constant" => Some(BoundAttr {
                attr_name: attr_name.clone(),
                index,
                ty: BoundAttrType::Const(attr.value.clone()),
            }),
            _ => None,
        };

        // Validate at most 1 attribute
        if found.is_some() && new_found.is_some() {
            bail!(
                &error_scope,
                "at most one #[func], #[signal], or #[constant] attribute per declaration allowed",
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

    let virtual_method_callbacks: Vec<TokenStream> = virtual_methods
        .iter()
        .map(|method| gdext_virtual_method_callback(&class_name, method))
        .collect();

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
                       #virtual_method_names => #virtual_method_callbacks,
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
