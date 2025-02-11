/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::class::{
    into_signature_info, make_constant_registration, make_method_registration,
    make_signal_registrations, ConstDefinition, FuncDefinition, RpcAttr, RpcMode, SignalDefinition,
    SignatureInfo, TransferMode,
};
use crate::util::{
    bail, c_str, format_funcs_collection_struct, ident, make_funcs_collection_constants,
    replace_class_in_path, require_api_version, KvParser,
};
use crate::{handle_mutually_exclusive_keys, util, ParseResult};

use proc_macro2::{Delimiter, Group, Ident, TokenStream};
use quote::spanned::Spanned;
use quote::{format_ident, quote};

/// Attribute for user-declared function.
enum ItemAttrType {
    Func(FuncAttr, Option<RpcAttr>),
    Signal(venial::AttributeValue),
    Const(#[allow(dead_code)] venial::AttributeValue),
}

struct ItemAttr {
    attr_name: Ident,
    ty: ItemAttrType,
}

impl ItemAttr {
    fn bail<R>(self, msg: &str, method: &venial::Function) -> ParseResult<R> {
        bail!(&method.name, "#[{}]: {}", self.attr_name, msg)
    }
}

enum AttrParseResult {
    Func(FuncAttr),
    Rpc(RpcAttr),
    FuncRpc(FuncAttr, RpcAttr),
    Signal(venial::AttributeValue),
    Const(#[allow(dead_code)] venial::AttributeValue),
}

impl AttrParseResult {
    fn into_attr_ty(self) -> ItemAttrType {
        match self {
            AttrParseResult::Func(func) => ItemAttrType::Func(func, None),
            // If only `#[rpc]` is present, we assume #[func] with default values.
            AttrParseResult::Rpc(rpc) => ItemAttrType::Func(FuncAttr::default(), Some(rpc)),
            AttrParseResult::FuncRpc(func, rpc) => ItemAttrType::Func(func, Some(rpc)),
            AttrParseResult::Signal(signal) => ItemAttrType::Signal(signal),
            AttrParseResult::Const(constant) => ItemAttrType::Const(constant),
        }
    }
}

#[derive(Default)]
struct FuncAttr {
    pub rename: Option<String>,
    pub is_virtual: bool,
    pub has_gd_self: bool,
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

pub struct InherentImplAttr {
    /// For implementation reasons, there can be a single 'primary' impl block and 0 or more 'secondary' impl blocks.
    /// For now, this is controlled by a key in the 'godot_api' attribute.
    pub secondary: bool,
}

/// Codegen for `#[godot_api] impl MyType`
pub fn transform_inherent_impl(
    meta: InherentImplAttr,
    mut impl_block: venial::Impl,
    self_path: venial::Path,
) -> ParseResult<TokenStream> {
    let class_name = util::validate_impl(&impl_block, None, "godot_api")?;
    let class_name_obj = util::class_name_obj(&class_name);
    let prv = quote! { ::godot::private };

    // Can add extra functions to the end of the impl block.
    let (funcs, signals) = process_godot_fns(&class_name, &mut impl_block, meta.secondary)?;
    let consts = process_godot_constants(&mut impl_block)?;

    #[cfg(all(feature = "register-docs", since_api = "4.3"))]
    let docs = crate::docs::make_inherent_impl_docs(&funcs, &consts, &signals);
    #[cfg(not(all(feature = "register-docs", since_api = "4.3")))]
    let docs = quote! {};

    // Container struct holding names of all registered #[func]s.
    // The struct is declared by #[derive(GodotClass)].
    let funcs_collection = {
        let struct_name = format_funcs_collection_struct(&class_name);
        replace_class_in_path(self_path, struct_name)
    };

    // For each #[func] in this impl block, create one constant.
    let func_name_constants = make_funcs_collection_constants(&funcs, &class_name);
    let signal_registrations = make_signal_registrations(signals, &class_name_obj);

    #[cfg(feature = "codegen-full")]
    let rpc_registrations = crate::class::make_rpc_registrations_fn(&class_name, &funcs);
    #[cfg(not(feature = "codegen-full"))]
    let rpc_registrations = TokenStream::new();

    let method_registrations: Vec<TokenStream> = funcs
        .into_iter()
        .map(|func_def| make_method_registration(&class_name, func_def, None))
        .collect::<ParseResult<Vec<TokenStream>>>()?;

    let constant_registration = make_constant_registration(consts, &class_name, &class_name_obj)?;

    let method_storage_name = format_ident!("__registration_methods_{class_name}");
    let constants_storage_name = format_ident!("__registration_constants_{class_name}");

    let fill_storage = quote! {
        ::godot::sys::plugin_execute_pre_main!({
            #method_storage_name.lock().unwrap().push(|| {
                #( #method_registrations )*
                #( #signal_registrations )*
            });

            #constants_storage_name.lock().unwrap().push(|| {
                #constant_registration
            });
        });
    };

    if !meta.secondary {
        // We are the primary `impl` block.

        let storage = quote! {
            #[allow(non_upper_case_globals)]
            #[doc(hidden)]
            static #method_storage_name: std::sync::Mutex<Vec<fn()>> = std::sync::Mutex::new(Vec::new());

            #[allow(non_upper_case_globals)]
            #[doc(hidden)]
            static #constants_storage_name: std::sync::Mutex<Vec<fn()>> = std::sync::Mutex::new(Vec::new());
        };

        let trait_impl = quote! {
            impl ::godot::obj::cap::ImplementsGodotApi for #class_name {
                fn __register_methods() {
                    let guard = #method_storage_name.lock().unwrap();
                    for f in guard.iter() {
                        f();
                    }
                }

                fn __register_constants() {
                    let guard = #constants_storage_name.lock().unwrap();
                    for f in guard.iter() {
                        f();
                    }
                }

                #rpc_registrations
            }
        };

        let class_registration = quote! {
            ::godot::sys::plugin_add!(__GODOT_PLUGIN_REGISTRY in #prv; #prv::ClassPlugin::new::<#class_name>(
                #prv::PluginItem::InherentImpl(#prv::InherentImpl::new::<#class_name>(#docs))
            ));
        };

        let result = quote! {
            #impl_block
            #storage
            #trait_impl
            #fill_storage
            #class_registration
            impl #funcs_collection {
                #( #func_name_constants )*
            }
        };

        Ok(result)
    } else {
        // We are in a secondary `impl` block, so most of the work has already been done,
        // and we just need to add our registration functions in the storage defined by the primary `impl` block.

        let result = quote! {
            #impl_block
            #fill_storage
            impl #funcs_collection {
                #( #func_name_constants )*
            }
        };

        Ok(result)
    }
}

fn process_godot_fns(
    class_name: &Ident,
    impl_block: &mut venial::Impl,
    is_secondary_impl: bool,
) -> ParseResult<(Vec<FuncDefinition>, Vec<SignalDefinition>)> {
    let mut func_definitions = vec![];
    let mut signal_definitions = vec![];
    let mut virtual_functions = vec![];

    let mut removed_indexes = vec![];
    for (index, item) in impl_block.body_items.iter_mut().enumerate() {
        let venial::ImplMember::AssocFunction(function) = item else {
            continue;
        };

        let Some(attr) = extract_attributes(function)? else {
            continue;
        };

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
            ItemAttrType::Func(func, rpc_info) => {
                let external_attributes = function.attributes.clone();

                // Signatures are the same thing without body.
                let mut signature = util::reduce_to_signature(function);
                let gd_self_parameter = if func.has_gd_self {
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
                let registered_name = if func.is_virtual {
                    let registered_name = add_virtual_script_call(
                        &mut virtual_functions,
                        function,
                        &signature_info,
                        class_name,
                        &func.rename,
                        gd_self_parameter,
                    );

                    Some(registered_name)
                } else {
                    func.rename
                };

                func_definitions.push(FuncDefinition {
                    signature_info,
                    external_attributes,
                    registered_name,
                    is_script_virtual: func.is_virtual,
                    rpc_info,
                });
            }

            ItemAttrType::Signal(ref _attr_val) => {
                if is_secondary_impl {
                    return attr.bail(
                        "#[signal] is not currently supported in secondary impl blocks",
                        function,
                    );
                }
                if function.return_ty.is_some() {
                    return bail!(
                        &function.return_ty,
                        "return types in #[signal] are not supported"
                    );
                }
                if function.body.is_some() {
                    return bail!(
                        &function.body,
                        "#[signal] must not have a body; declare the function with a semicolon"
                    );
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

        if let Some(attr) = extract_attributes(constant)? {
            match attr.ty {
                ItemAttrType::Func(_, _) => {
                    return bail!(constant, "#[func] and #[rpc] can only be used on functions")
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
) -> String {
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

    let method_name_str = match rename {
        Some(rename) => rename.clone(),
        None => format!("_{}", function.name),
    };
    let method_name_cstr = c_str(&method_name_str);

    let call_params = signature_info.params_type();
    let call_ret = &signature_info.ret_type;
    let arg_names = &signature_info.param_idents;

    let (object_ptr, receiver);
    if let Some(gd_self_parameter) = gd_self_parameter {
        object_ptr = quote! { #gd_self_parameter.obj_sys() };
        receiver = gd_self_parameter;
    } else {
        object_ptr = quote! { <Self as ::godot::obj::WithBaseField>::base_field(self).obj_sys() };
        receiver = ident("self");
    };

    let code = quote! {
        let object_ptr = #object_ptr;
        let method_sname = ::godot::builtin::StringName::from(#method_name_cstr);
        let method_sname_ptr = method_sname.string_sys();
        let has_virtual_override = unsafe { ::godot::private::has_virtual_script_method(object_ptr, method_sname_ptr) };

        if has_virtual_override {
            // Dynamic dispatch.
            type CallParams = #call_params;
            type CallRet = #call_ret;
            let args = (#( #arg_names, )*);
            unsafe {
                ::godot::meta::Signature::<CallParams, CallRet>::out_script_virtual_call(
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

    method_name_str
}

fn extract_attributes<T>(item: &mut T) -> ParseResult<Option<ItemAttr>>
where
    for<'a> &'a T: Spanned,
    T: AttributesMut,
{
    // Option<(attr_name: Ident, attr: ParsedAttr)>
    let mut found = None;
    let mut index = 0;

    let attributes = item.attributes_mut();

    while let Some(attr) = attributes.get(index) {
        index += 1;

        let Some(attr_name) = attr.get_single_path_segment() else {
            // Attribute of the form #[segmented::path] can't be what we are looking for.
            continue;
        };

        let parsed_attr = match attr_name {
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

                AttrParseResult::Func(FuncAttr {
                    rename,
                    is_virtual,
                    has_gd_self,
                })
            }

            // #[rpc]
            name if name == "rpc" => {
                // Safe unwrap, since #[rpc] must be present if we got to this point.
                let mut parser = KvParser::parse(attributes, "rpc")?.unwrap();

                let rpc_mode = handle_mutually_exclusive_keys(
                    &mut parser,
                    "#[rpc]",
                    &["any_peer", "authority"],
                )?
                .map(|idx| RpcMode::from_usize(idx).unwrap());

                let transfer_mode = handle_mutually_exclusive_keys(
                    &mut parser,
                    "#[rpc]",
                    &["reliable", "unreliable", "unreliable_ordered"],
                )?
                .map(|idx| TransferMode::from_usize(idx).unwrap());

                let call_local = handle_mutually_exclusive_keys(
                    &mut parser,
                    "#[rpc]",
                    &["call_local", "call_remote"],
                )?
                .map(|idx| idx == 0);

                let channel = parser.handle_usize("channel")?.map(|x| x as u32);

                let config_expr = parser.handle_expr("config")?;

                parser.finish()?;

                let rpc_attr = match (config_expr, (&rpc_mode, &transfer_mode, &call_local, &channel)) {
		            // Ok: Only `config = [expr]` is present.
		            (Some(expr), (None, None, None, None)) => RpcAttr::Expression(expr),

		            // Err: `config = [expr]` is present along other parameters, which is not allowed.
		            (Some(_), _) => return bail!(
                        &*item,
                        "`#[rpc(config = ...)]` is mutually exclusive with any other parameters(`any_peer`, `reliable`, `call_local`, `channel = 0`)"
                    ),

		            // Ok: `config` is not present, any combination of the other parameters is allowed.
		            _ => RpcAttr::SeparatedArgs {
			            rpc_mode,
			            transfer_mode,
			            call_local,
			            channel,
		            }
	            };

                AttrParseResult::Rpc(rpc_attr)
            }

            // #[signal]
            name if name == "signal" => AttrParseResult::Signal(attr.value.clone()),

            // #[constant]
            name if name == "constant" => AttrParseResult::Const(attr.value.clone()),

            // Ignore unknown attributes.
            _ => continue,
        };

        let attr_name = attr_name.clone();

        // Remaining code no longer has attribute -- rest stays.
        attributes.remove(index - 1); // -1 because we bumped the index at the beginning of the loop.
        index -= 1;

        let (new_name, new_attr) = match (found, parsed_attr) {
            // First attribute.
            (None, parsed) => (attr_name, parsed),

            // Regardless of the order, if we found both `#[func]` and `#[rpc]`, we can just merge them.
            (Some((found_name, AttrParseResult::Func(func))), AttrParseResult::Rpc(rpc))
            | (Some((found_name, AttrParseResult::Rpc(rpc))), AttrParseResult::Func(func)) => (
                ident(&format!("{found_name}_{attr_name}")),
                AttrParseResult::FuncRpc(func, rpc),
            ),

            // We found two incompatible attributes.
            (Some((found_name, _)), _) => {
                return bail!(&*item, "The attributes `{found_name}` and `{attr_name}` cannot be used in the same declaration")?;
            }
        };

        found = Some((new_name, new_attr));
    }

    Ok(found.map(|(attr_name, attr)| ItemAttr {
        attr_name,
        ty: attr.into_attr_ty(),
    }))
}

fn bail_attr<R>(attr_name: Ident, msg: &str, method: &venial::Function) -> ParseResult<R> {
    bail!(&method.name, "#[{}]: {}", attr_name, msg)
}

trait AttributesMut {
    fn attributes_mut(&mut self) -> &mut Vec<venial::Attribute>;
}

impl AttributesMut for venial::Function {
    fn attributes_mut(&mut self) -> &mut Vec<venial::Attribute> {
        &mut self.attributes
    }
}

impl AttributesMut for venial::Constant {
    fn attributes_mut(&mut self) -> &mut Vec<venial::Attribute> {
        &mut self.attributes
    }
}
