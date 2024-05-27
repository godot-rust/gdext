/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::class::{
    into_signature_info, make_constant_registration, make_method_registration,
    make_signal_registrations, ConstDefinition, FuncDefinition, SignalDefinition, SignatureInfo,
};
use crate::util::{bail, require_api_version, KvParser};
use crate::{util, ParseResult};

use proc_macro2::{Delimiter, Group, Ident, TokenStream};
use quote::spanned::Spanned;
use quote::{format_ident, quote};

/// Attribute for user-declared function.
enum ItemAttrType {
    Func {
        rename: Option<String>,
        is_virtual: bool,
        has_gd_self: bool,
    },
    Signal(venial::AttributeValue),
    Const(#[allow(dead_code)] venial::AttributeValue),
}

struct ItemAttr {
    attr_name: Ident,
    index: usize,
    ty: ItemAttrType,
}

impl ItemAttr {
    fn bail<R>(self, msg: &str, method: &venial::Function) -> ParseResult<R> {
        bail!(&method.name, "#[{}]: {}", self.attr_name, msg)
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

/// Codegen for `#[godot_api] impl MyType`
pub fn transform_inherent_impl(mut impl_block: venial::Impl) -> ParseResult<TokenStream> {
    let class_name = util::validate_impl(&impl_block, None, "godot_api")?;
    let class_name_obj = util::class_name_obj(&class_name);
    let prv = quote! { ::godot::private };

    // Can add extra functions to the end of the impl block.
    let (funcs, signals) = process_godot_fns(&class_name, &mut impl_block)?;
    let consts = process_godot_constants(&mut impl_block)?;

    let signal_registrations = make_signal_registrations(signals, &class_name_obj);

    let method_registrations: Vec<TokenStream> = funcs
        .into_iter()
        .map(|func_def| make_method_registration(&class_name, func_def))
        .collect::<ParseResult<Vec<TokenStream>>>()?; // <- FIXME transpose this

    let constant_registration = make_constant_registration(consts, &class_name, &class_name_obj)?;

    let result = quote! {
        #impl_block

        impl ::godot::obj::cap::ImplementsGodotApi for #class_name {
            fn __register_methods() {
                #( #method_registrations )*
                #( #signal_registrations )*
            }

            fn __register_constants() {
                #constant_registration
            }
        }

        ::godot::sys::plugin_add!(__GODOT_PLUGIN_REGISTRY in #prv; #prv::ClassPlugin {
            class_name: #class_name_obj,
            item: #prv::PluginItem::InherentImpl {
                register_methods_constants_fn: #prv::ErasedRegisterFn {
                    raw: #prv::callbacks::register_user_methods_constants::<#class_name>,
                },
            },
            init_level: <#class_name as ::godot::obj::GodotClass>::INIT_LEVEL,
        });
    };

    Ok(result)
}

fn process_godot_fns(
    class_name: &Ident,
    impl_block: &mut venial::Impl,
) -> ParseResult<(Vec<FuncDefinition>, Vec<SignalDefinition>)> {
    let mut func_definitions = vec![];
    let mut signal_definitions = vec![];
    let mut virtual_functions = vec![];

    let mut removed_indexes = vec![];
    for (index, item) in impl_block.body_items.iter_mut().enumerate() {
        let venial::ImplMember::AssocFunction(function) = item else {
            continue;
        };

        let Some(attr) = extract_attributes(&function, &function.attributes)? else {
            continue;
        };

        // Remaining code no longer has attribute -- rest stays
        function.attributes.remove(attr.index);

        if function.qualifiers.tk_default.is_some()
            || function.qualifiers.tk_const.is_some()
            || function.qualifiers.tk_async.is_some()
            || function.qualifiers.tk_unsafe.is_some()
            || function.qualifiers.tk_extern.is_some()
            || function.qualifiers.extern_abi.is_some()
        {
            return attr.bail("fn qualifiers are not allowed", function);
        }

        if function.generic_params.is_some() {
            return attr.bail("generic fn parameters are not supported", function);
        }

        match attr.ty {
            ItemAttrType::Func {
                rename,
                is_virtual,
                has_gd_self,
            } => {
                let external_attributes = function.attributes.clone();

                // Signatures are the same thing without body.
                let mut signature = util::reduce_to_signature(function);
                let gd_self_parameter = if has_gd_self {
                    if signature.params.is_empty() {
                        return bail_attr(
                            attr.attr_name,
                            "with attribute key `gd_self`, the method must have a first parameter of type Gd<Self>",
                            function
                        );
                    } else {
                        let param = signature.params.inner.remove(0);

                        let venial::FnParam::Typed(param) = param.0 else {
                            return bail_attr(
                                attr.attr_name,
                                "with attribute key `gd_self`, the first parameter must be Gd<Self> (not a `self` receiver)",
                                function
                            );
                        };

                        // Note: parameter is explicitly NOT renamed (maybe_rename_parameter).
                        Some(param.name)
                    }
                } else {
                    None
                };

                // Clone might not strictly be necessary, but the 2 other callers of into_signature_info() are better off with pass-by-value.
                let signature_info =
                    into_signature_info(signature.clone(), class_name, gd_self_parameter.is_some());

                // For virtual methods, rename/mangle existing user method and create a new method with the original name,
                // which performs a dynamic dispatch.
                if is_virtual {
                    add_virtual_script_call(
                        &mut virtual_functions,
                        function,
                        &signature_info,
                        class_name,
                        &rename,
                        gd_self_parameter,
                    );
                };

                func_definitions.push(FuncDefinition {
                    signature,
                    signature_info,
                    external_attributes,
                    rename,
                    is_script_virtual: is_virtual,
                    has_gd_self,
                });
            }
            ItemAttrType::Signal(ref _attr_val) => {
                if function.return_ty.is_some() {
                    return attr.bail("return types are not supported", function);
                }

                let external_attributes = function.attributes.clone();
                let sig = util::reduce_to_signature(function);

                signal_definitions.push(SignalDefinition {
                    signature: sig,
                    external_attributes,
                });

                removed_indexes.push(index);
            }
            ItemAttrType::Const(_) => {
                return attr.bail(
                    "#[constant] can only be used on associated constant",
                    function,
                )
            }
        }
    }

    // Remove some elements (e.g. signals) from impl.
    // O(n^2); alternative: retain(), but elements themselves don't have the necessary information.
    for index in removed_indexes.into_iter().rev() {
        impl_block.body_items.remove(index);
    }

    // Add script-virtual extra functions at the end of same impl block (subject to same attributes).
    for f in virtual_functions.into_iter() {
        let member = venial::ImplMember::AssocFunction(f);
        impl_block.body_items.push(member);
    }

    Ok((func_definitions, signal_definitions))
}

fn process_godot_constants(decl: &mut venial::Impl) -> ParseResult<Vec<ConstDefinition>> {
    let mut constant_signatures = vec![];

    for item in decl.body_items.iter_mut() {
        let venial::ImplMember::AssocConstant(constant) = item else {
            continue;
        };

        if let Some(attr) = extract_attributes(&constant, &constant.attributes)? {
            // Remaining code no longer has attribute -- rest stays
            constant.attributes.remove(attr.index);

            match attr.ty {
                ItemAttrType::Func { .. } => {
                    return bail!(constant, "#[func] can only be used on functions")
                }
                ItemAttrType::Signal(_) => {
                    return bail!(constant, "#[signal] can only be used on functions")
                }
                ItemAttrType::Const(_) => {
                    if constant.initializer.is_none() {
                        return bail!(constant, "exported constant must have initializer");
                    }

                    let definition = ConstDefinition {
                        raw_constant: constant.clone(),
                    };

                    constant_signatures.push(definition);
                }
            }
        }
    }

    Ok(constant_signatures)
}

fn add_virtual_script_call(
    virtual_functions: &mut Vec<venial::Function>,
    function: &mut venial::Function,
    signature_info: &SignatureInfo,
    class_name: &Ident,
    rename: &Option<String>,
    gd_self_parameter: Option<Ident>,
) {
    assert!(cfg!(since_api = "4.3"));

    // Update parameter names, so they can be forwarded (e.g. a "_" declared by the user cannot).
    let is_params = function.params.iter_mut().skip(1); // skip receiver.
    let should_param_names = signature_info.param_idents.iter();
    is_params
        .zip(should_param_names)
        .for_each(|(param, should_param_name)| {
            if let venial::FnParam::Typed(param) = &mut param.0 {
                param.name = should_param_name.clone();
            }
        });

    let class_name_str = class_name.to_string();
    let early_bound_name = format_ident!("__earlybound_{}", &function.name);
    let method_name_str = rename
        .clone()
        .unwrap_or_else(|| format!("_{}", function.name));

    let sig_tuple = signature_info.tuple_type();
    let arg_names = &signature_info.param_idents;

    let (object_ptr, receiver);
    if let Some(gd_self_parameter) = gd_self_parameter {
        object_ptr = quote! { #gd_self_parameter.obj_sys() };
        receiver = gd_self_parameter;
    } else {
        object_ptr = quote! { <Self as ::godot::obj::WithBaseField>::base_field(self).obj_sys() };
        receiver = util::ident("self");
    };

    let code = quote! {
        let object_ptr = #object_ptr;
        let method_sname = ::godot::builtin::StringName::from(#method_name_str);
        let method_sname_ptr = method_sname.string_sys();
        let has_virtual_override = unsafe { ::godot::private::has_virtual_script_method(object_ptr, method_sname_ptr) };

        if has_virtual_override {
            // Dynamic dispatch.
            type CallSig = #sig_tuple;
            let args = (#( #arg_names, )*);
            unsafe {
                <CallSig as ::godot::meta::VarcallSignatureTuple>::out_script_virtual_call(
                    #class_name_str,
                    #method_name_str,
                    method_sname_ptr,
                    object_ptr,
                    args,
                )
            }
        } else {
            // Fall back to default implementation.
            Self::#early_bound_name(#receiver, #( #arg_names ),*)
        }
    };

    let mut early_bound_function = venial::Function {
        name: early_bound_name,
        body: Some(Group::new(Delimiter::Brace, code)),
        ..function.clone()
    };

    std::mem::swap(&mut function.body, &mut early_bound_function.body);
    virtual_functions.push(early_bound_function);
}

fn extract_attributes<T>(
    error_scope: T,
    attributes: &[venial::Attribute],
) -> ParseResult<Option<ItemAttr>>
where
    for<'a> &'a T: Spanned,
{
    let mut found = None;
    for (index, attr) in attributes.iter().enumerate() {
        let Some(attr_name) = attr.get_single_path_segment() else {
            // Attribute of the form #[segmented::path] can't be what we are looking for
            continue;
        };

        let new_found = match attr_name {
            // #[func]
            name if name == "func" => {
                // Safe unwrap since #[func] must be present if we got to this point
                let mut parser = KvParser::parse(attributes, "func")?.unwrap();

                // #[func(rename = MyClass)]
                let rename = parser.handle_expr("rename")?.map(|ts| ts.to_string());

                // #[func(virtual)]
                let is_virtual = if let Some(span) = parser.handle_alone_with_span("virtual")? {
                    require_api_version!("4.3", span, "#[func(virtual)]")?;
                    true
                } else {
                    false
                };

                // #[func(gd_self)]
                let has_gd_self = parser.handle_alone("gd_self")?;

                parser.finish()?;

                ItemAttr {
                    attr_name: attr_name.clone(),
                    index,
                    ty: ItemAttrType::Func {
                        rename,
                        is_virtual,
                        has_gd_self,
                    },
                }
            }

            // #[signal]
            name if name == "signal" => {
                // TODO once parameters are supported, this should probably be moved to the struct definition
                // E.g. a zero-sized type Signal<(i32, String)> with a provided emit(i32, String) method
                // This could even be made public (callable on the struct obj itself)
                ItemAttr {
                    attr_name: attr_name.clone(),
                    index,
                    ty: ItemAttrType::Signal(attr.value.clone()),
                }
            }

            // #[constant]
            name if name == "constant" => ItemAttr {
                attr_name: attr_name.clone(),
                index,
                ty: ItemAttrType::Const(attr.value.clone()),
            },

            // Ignore unknown attributes.
            _ => continue,
        };

        // Ensure at most 1 attribute.
        if found.is_some() {
            bail!(
                &error_scope,
                "at most one #[func], #[signal] or #[constant] attribute per declaration allowed",
            )?;
        }

        found = Some(new_found);
    }

    Ok(found)
}

fn bail_attr<R>(attr_name: Ident, msg: &str, method: &venial::Function) -> ParseResult<R> {
    bail!(&method.name, "#[{}]: {}", attr_name, msg)
}
