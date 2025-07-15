use crate::{
    bail,
    class::{
        add_virtual_script_call, extract_gd_self, into_signature_info, parse_attributes,
        FuncDefinition, ItemAttrType, SignalDefinition,
    },
    util, ParseResult,
};

use proc_macro2::{Ident, TokenStream};
use quote::quote;

pub fn transform_dyn_trait_impl(
    mut decl: venial::Impl,
    prv: TokenStream,
    class_path: venial::TypeExpr,
    trait_path: venial::TypeExpr,
    assoc_type_constraints: TokenStream,
) -> ParseResult<TokenStream> {
    //eprintln!("decl: {decl:?}");

    // Extract Ident from class_path for into_signature_info
    let class_path = class_path.as_path().unwrap();
    let class_name = class_path.segments.last().unwrap().ident.clone();

    let (funcs, signals) = process_godot_fns(&class_name, &mut decl)?;

    eprintln!("funcs: {funcs:?}");
    eprintln!("signals: {signals:?}");

    let new_code = quote! {
        #decl

        impl ::godot::obj::AsDyn<dyn #trait_path #assoc_type_constraints> for #class_path {
            fn dyn_upcast(&self) -> &(dyn #trait_path #assoc_type_constraints + 'static) {
                self
            }

            fn dyn_upcast_mut(&mut self) -> &mut (dyn #trait_path #assoc_type_constraints + 'static) {
                self
            }
        }

        ::godot::sys::plugin_add!(#prv::__GODOT_PLUGIN_REGISTRY; #prv::ClassPlugin::new::<#class_path>(
            #prv::PluginItem::DynTraitImpl(#prv::DynTraitImpl::new::<#class_path, dyn #trait_path #assoc_type_constraints>()))
        );

    };
    Ok(new_code)
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

        eprintln!(
            "Processing function: {} with attributes: {:?}",
            function.name, attr
        );

        if function.generic_params.is_some() {
            return bail!(
                &function.generic_params,
                "#[func]: generic fn parameters are not supported"
            );
        }

        match attr.ty {
            ItemAttrType::Func(func, rpc_info) => {
                let external_attributes = function.attributes.clone();

                // Transforms the following.
                //   from function:     #[attr] pub fn foo(&self, a: i32) -> i32 { ... }
                //   into signature:    fn foo(&self, a: i32) -> i32
                let mut signature = util::reduce_to_signature(function);
                let gd_self_parameter = if func.has_gd_self {
                    // Removes Gd<Self> receiver from signature for further processing.
                    let param_name = extract_gd_self(&mut signature, &attr.attr_name)?;
                    Some(param_name)
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

            ItemAttrType::Signal(ref signal, ref _attr_val) => {
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
