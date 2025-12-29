/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use proc_macro2::{Delimiter, Group, Ident, Span, TokenStream};
use quote::spanned::Spanned;
use quote::{format_ident, quote, ToTokens};

use crate::class::data_models::func;
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

/// Attribute for user-declared function.
enum ItemAttrType {
    Func(FuncAttr, Option<RpcAttr>),
    Signal(SignalAttr, venial::AttributeValue),
    Const(#[allow(dead_code)] venial::AttributeValue),
}

struct ItemAttr {
    attr_name: Ident,
    ty: ItemAttrType,
}

enum AttrParseResult {
    Func(FuncAttr),
    Rpc(RpcAttr),
    FuncRpc(FuncAttr, RpcAttr),
    Signal(SignalAttr, venial::AttributeValue),
    Constant(#[allow(dead_code)] venial::AttributeValue),
}

impl AttrParseResult {
    fn into_attr_ty(self) -> ItemAttrType {
        match self {
            AttrParseResult::Func(func) => ItemAttrType::Func(func, None),
            // If only `#[rpc]` is present, we assume #[func] with default values.
            AttrParseResult::Rpc(rpc) => ItemAttrType::Func(FuncAttr::default(), Some(rpc)),
            AttrParseResult::FuncRpc(func, rpc) => ItemAttrType::Func(func, Some(rpc)),
            AttrParseResult::Signal(signal, attr_val) => ItemAttrType::Signal(signal, attr_val),
            AttrParseResult::Constant(constant) => ItemAttrType::Const(constant),
        }
    }
}

#[derive(Default)]
struct FuncAttr {
    pub rename: Option<String>,
    pub is_virtual: bool,
    pub has_gd_self: bool,
}

#[derive(Default)]
struct SignalAttr {
    pub no_builder: bool,
}

pub(crate) struct InherentImplAttr {
    /// For implementation reasons, there can be a single 'primary' impl block and 0 or more 'secondary' impl blocks.
    /// For now, this is controlled by a key in the 'godot_api' attribute.
    pub secondary: bool,

    /// When typed signal generation is explicitly disabled by the user.
    pub no_typed_signals: bool,
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

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

    let inherent_impl_docs =
        crate::docs::make_trait_docs_registration(&funcs, &consts, &signals, &class_name, &prv);

    // Container struct holding names of all registered #[func]s.
    // The struct is declared by #[derive(GodotClass)].
    let funcs_collection = {
        let struct_name = format_funcs_collection_struct(&class_name);
        replace_class_in_path(self_path, struct_name)
    };

    // For each #[func] in this impl block, create one constant.
    let func_name_constants = make_funcs_collection_constants(&funcs, &class_name);
    let (signal_registrations, signal_symbol_types) = make_signal_registrations(
        &signals,
        &class_name,
        &class_name_obj,
        meta.no_typed_signals,
    )?;

    #[cfg(feature = "codegen-full")]
    let rpc_registrations = crate::class::make_rpc_registrations_fn(&class_name, &funcs);
    #[cfg(not(feature = "codegen-full"))]
    let rpc_registrations = TokenStream::new();

    let method_registrations: Vec<TokenStream> = funcs
        .into_iter()
        .map(|func_def| make_method_registration(&class_name, func_def, None))
        .collect::<ParseResult<Vec<TokenStream>>>()?;

    let constant_registration = make_constant_registration(consts, &class_name, &class_name_obj)?;

    // Internal idents unlikely to surface in user code; but span shouldn't hurt.
    let class_span = class_name.span();
    let method_storage_name =
        format_ident!("__registration_methods_{class_name}", span = class_span);
    let constants_storage_name =
        format_ident!("__registration_constants_{class_name}", span = class_span);

    let fill_storage = {
        quote! {
            ::godot::sys::plugin_execute_pre_main!({
                #method_storage_name.lock().unwrap().push(|| {
                    #( #method_registrations )*
                    #( #signal_registrations )*
                });

                #constants_storage_name.lock().unwrap().push(|| {
                    #constant_registration
                });

            });
        }
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
            ::godot::sys::plugin_add!(#prv::__GODOT_PLUGIN_REGISTRY; #prv::ClassPlugin::new::<#class_name>(
                #prv::PluginItem::InherentImpl(#prv::InherentImpl::new::<#class_name>())
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
            #signal_symbol_types
            #inherent_impl_docs
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
            #inherent_impl_docs
        };

        Ok(result)
    }
}

/* Re-enable if we allow controlling declarative macros for signals (base_field_macro, visibility_macros).
fn extract_hint_attribute(impl_block: &mut venial:: Impl) -> ParseResult<GodotApiHints> {
    // #[hint(has_base_field = BOOL)]
    let has_base_field;
    if let Some(mut hints) = KvParser::parse_remove(&mut impl_block.attributes, "hint")? {
        has_base_field = hints.handle_bool("has_base_field")?;
    } else {
        has_base_field = None;
    }

    // #[hint(class_visibility = pub(crate))]
    // ...

    Ok(GodotApiHints { has_base_field })
}
*/

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

        let Some(attr) = parse_attributes(function)? else {
            continue;
        };

        if function.qualifiers.tk_default.is_some()
            || function.qualifiers.tk_const.is_some()
            || function.qualifiers.tk_async.is_some()
            || function.qualifiers.tk_unsafe.is_some()
            || function.qualifiers.tk_extern.is_some()
            || function.qualifiers.extern_abi.is_some()
        {
            return bail!(
                &function.qualifiers,
                "#[func]: fn qualifiers are not allowed"
            );
        }

        if function.generic_params.is_some() {
            return bail!(
                &function.generic_params,
                "#[func]: generic fn parameters are not supported"
            );
        }

        match attr.ty {
            ItemAttrType::Func(func, rpc_info) => {
                if rpc_info.is_some() && is_secondary_impl {
                    return bail!(
                        &function,
                        "#[rpc] is currently not supported in secondary impl blocks",
                    )?;
                }

                let external_attributes = function.attributes.clone();

                // Transforms the following.
                //   from function:     #[attr] pub fn foo(&self, a: i32) -> i32 { ... }
                //   into signature:    fn foo(&self, a: i32) -> i32
                let mut signature = util::reduce_to_signature(function);
                let gd_self_parameter = func::validate_receiver_extract_gdself(
                    &mut signature,
                    func.has_gd_self,
                    &attr.attr_name,
                )?;

                // Clone might not strictly be necessary, but the 2 other callers of into_signature_info() are better off with pass-by-value.
                let mut signature_info =
                    into_signature_info(signature.clone(), class_name, gd_self_parameter.is_some());

                // Default value expressions from `#[opt(default = EXPR)]`; None for required parameters.
                let all_param_maybe_defaults = parse_default_expressions(&mut function.params)?;
                signature_info.optional_param_default_exprs =
                    validate_default_exprs(all_param_maybe_defaults, &signature_info.param_idents)?;

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

            ItemAttrType::Signal(ref signal, ref _attr_val) => {
                if is_secondary_impl {
                    return bail!(
                        function,
                        "#[signal] is currently not supported in secondary impl blocks",
                    );
                }
                if function.return_ty.is_some() {
                    return bail!(
                        &function.return_ty,
                        "#[signal] does not support return types"
                    );
                }
                if function.body.is_some() {
                    return bail!(
                        &function.body,
                        "#[signal] must not have a body; declare the function with a semicolon"
                    );
                }

                let external_attributes = function.attributes.clone();

                let mut fn_signature = util::reduce_to_signature(function);
                fn_signature.vis_marker = function.vis_marker.clone();

                signal_definitions.push(SignalDefinition {
                    fn_signature,
                    external_attributes,
                    has_builder: !signal.no_builder,
                });

                removed_indexes.push(index);
            }

            ItemAttrType::Const(_) => {
                return bail!(
                    function,
                    "#[constant] can only be used on associated constant",
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

        if let Some(attr) = parse_attributes(constant)? {
            match attr.ty {
                ItemAttrType::Func(_, _) => {
                    return bail!(constant, "#[func] and #[rpc] can only be used on functions")
                }
                ItemAttrType::Signal(_, _) => {
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

/// Replaces the body of `function` with custom code that performs virtual dispatch.
///
/// Appends the virtual function to `virtual_functions`.
///
/// Returns the Godot-registered name of the virtual function, usually `_<name>` (but overridable with `#[func(rename = ...)]`).
fn add_virtual_script_call(
    virtual_functions: &mut Vec<venial::Function>,
    function: &mut venial::Function,
    signature_info: &SignatureInfo,
    class_name: &Ident,
    rename: &Option<String>,
    gd_self_parameter: Option<Ident>,
) -> String {
    #[allow(clippy::assertions_on_constants)]
    {
        // Without braces, clippy removes the #[allow] for some reason...
        assert!(cfg!(since_api = "4.3"));
    }

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
    let early_bound_name = format_ident!(
        "__earlybound_{}",
        function.name,
        span = function.name.span()
    );

    let method_name_str = match rename {
        Some(rename) => rename.clone(),
        None => format!("_{}", function.name),
    };
    let method_name_cstr = c_str(&method_name_str);

    let call_params = signature_info.params_type();
    let call_ret = &signature_info.return_type;
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
        let method_sname = ::godot::builtin::StringName::__cstr(#method_name_cstr);
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

/// Parses an entire item (`fn`, `const`) inside an `impl` block and returns a domain representation.
///
/// See also [`parse_attributes_inner`].
fn parse_attributes<T: ImplItem>(item: &mut T) -> ParseResult<Option<ItemAttr>> {
    let span = util::span_of(item);
    parse_attributes_inner(item.attributes_mut(), span)
}

/// Non-generic version of [`parse_attributes`].
///
/// `attributes` are all `#[...]` attributes of the item, including foreign (non-godot-rust) ones.
/// `full_item_span` is the span of the entire item (attributes + `fn`/...), for error messages.
fn parse_attributes_inner(
    attributes: &mut Vec<venial::Attribute>,
    full_item_span: Span,
) -> ParseResult<Option<ItemAttr>> {
    // Option<(attr_name: Ident, attr: ParsedAttr)>
    let mut found = None;
    let mut index = 0;

    while let Some(attr) = attributes.get(index) {
        index += 1;

        let Some(attr_name) = attr.get_single_path_segment() else {
            // Attribute of the form #[segmented::path] can't be what we are looking for.
            continue;
        };

        let parsed_attr = match attr_name {
            name if name == "func" => parse_func_attr(attributes)?,
            name if name == "rpc" => parse_rpc_attr(attributes)?,
            name if name == "signal" => parse_signal_attr(attributes, attr)?,
            name if name == "constant" => parse_constant_attr(attributes, attr)?,

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
                return bail!(full_item_span, "attributes `{found_name}` and `{attr_name}` cannot be used in the same declaration");
            }
        };

        found = Some((new_name, new_attr));
    }

    Ok(found.map(|(attr_name, attr)| ItemAttr {
        attr_name,
        ty: attr.into_attr_ty(),
    }))
}

/// `#[func]` attribute.
fn parse_func_attr(attributes: &[venial::Attribute]) -> ParseResult<AttrParseResult> {
    // Safe unwrap, since #[func] must be present if we got to this point.
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

    Ok(AttrParseResult::Func(FuncAttr {
        rename,
        is_virtual,
        has_gd_self,
    }))
}

/// `#[rpc]` attribute.
fn parse_rpc_attr(attributes: &[venial::Attribute]) -> ParseResult<AttrParseResult> {
    // Safe unwrap, since #[rpc] must be present if we got to this point.
    let mut parser = KvParser::parse(attributes, "rpc")?.unwrap();

    let rpc_mode =
        handle_mutually_exclusive_keys(&mut parser, "#[rpc]", &["any_peer", "authority"])?
            .map(|idx| RpcMode::from_usize(idx).unwrap());

    let transfer_mode = handle_mutually_exclusive_keys(
        &mut parser,
        "#[rpc]",
        &["reliable", "unreliable", "unreliable_ordered"],
    )?
    .map(|idx| TransferMode::from_usize(idx).unwrap());

    let call_local =
        handle_mutually_exclusive_keys(&mut parser, "#[rpc]", &["call_local", "call_remote"])?
            .map(|idx| idx == 0);

    let channel = parser.handle_usize("channel")?.map(|x| x as u32);

    let config_expr = parser.handle_expr("config")?;

    let item_span = parser.span();
    parser.finish()?;

    let rpc_attr = match (config_expr, (&rpc_mode, &transfer_mode, &call_local, &channel)) {
        // Ok: Only `config = [expr]` is present.
        (Some(expr), (None, None, None, None)) => RpcAttr::Expression(expr),

        // Err: `config = [expr]` is present along other parameters, which is not allowed.
        (Some(_), _) => return bail!(
            item_span,
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

    Ok(AttrParseResult::Rpc(rpc_attr))
}

/// `#[signal]` attribute.
fn parse_signal_attr(
    attributes: &[venial::Attribute],
    attr: &venial::Attribute,
) -> ParseResult<AttrParseResult> {
    // Safe unwrap, since #[signal] must be present if we got to this point.
    let mut parser = KvParser::parse(attributes, "signal")?.unwrap();

    // Private #[signal(__no_builder)]
    let no_builder = parser.handle_alone("__no_builder")?;

    parser.finish()?;

    let signal_attr = SignalAttr { no_builder };

    Ok(AttrParseResult::Signal(signal_attr, attr.value.clone()))
}

/// `#[constant]` attribute.
fn parse_constant_attr(
    attributes: &[venial::Attribute],
    attr: &venial::Attribute,
) -> ParseResult<AttrParseResult> {
    // Ensure no keys are present.
    let parser = KvParser::parse(attributes, "constant")?.unwrap();
    parser.finish()?;

    Ok(AttrParseResult::Constant(attr.value.clone()))
}

/// Parses `#[opt(default = ...)]` parameter attributes and validates that optional parameters only appear at the end.
///
/// Returns a vector of optional default values, one per parameter (skipping receiver).
fn parse_default_expressions(
    params: &mut venial::Punctuated<venial::FnParam>,
) -> ParseResult<Vec<Option<TokenStream>>> {
    let mut res = vec![];

    for param in params.iter_mut() {
        let typed_param = match &mut param.0 {
            venial::FnParam::Receiver(_) => continue,
            venial::FnParam::Typed(fn_typed_param) => fn_typed_param,
        };

        let optional_value = match KvParser::parse_remove(&mut typed_param.attributes, "opt")? {
            None => None,
            Some(mut parser) => Some(parser.handle_expr_required("default")?),
        };

        res.push(optional_value);
    }

    Ok(res)
}

/// Validates that default parameters only appear at the end of the parameter list.
/// Consumes the input and returns only the non-None default expressions.
fn validate_default_exprs(
    all_param_maybe_defaults: Vec<Option<TokenStream>>,
    param_idents: &[Ident],
) -> ParseResult<Vec<TokenStream>> {
    let mut must_be_default = false;
    let mut result = Vec::new();

    for (i, param) in all_param_maybe_defaults.into_iter().enumerate() {
        match (param, must_be_default) {
            // First optional parameter encountered.
            (Some(default_expr), false) => {
                must_be_default = true;
                result.push(default_expr);
            }

            // Subsequent optional parameters.
            (Some(default_expr), true) => {
                result.push(default_expr);
            }

            // Required parameter before any optional ones.
            (None, false) => {}

            // Required parameter after optional ones.
            (None, true) => {
                let name = &param_idents[i];
                return bail!(
                    name,
                    "parameter `{name}` must have a default value, because previous parameters are already optional",
                );
            }
        }
    }

    Ok(result)
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

trait ImplItem
where
    Self: ToTokens,
    for<'a> &'a Self: Spanned,
{
    fn attributes_mut(&mut self) -> &mut Vec<venial::Attribute>;
}

impl ImplItem for venial::Function {
    fn attributes_mut(&mut self) -> &mut Vec<venial::Attribute> {
        &mut self.attributes
    }
}

impl ImplItem for venial::Constant {
    fn attributes_mut(&mut self) -> &mut Vec<venial::Attribute> {
        &mut self.attributes
    }
}
