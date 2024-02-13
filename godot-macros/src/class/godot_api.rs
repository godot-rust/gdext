/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use proc_macro2::{Delimiter, Group, Ident, TokenStream};
use quote::spanned::Spanned;
use quote::{format_ident, quote};

use crate::class::{
    into_signature_info, make_method_registration, make_virtual_callback, BeforeKind,
    FuncDefinition, SignatureInfo,
};
use crate::util::{bail, require_api_version, KvParser};
use crate::{util, ParseResult};

pub fn attribute_godot_api(input_decl: venial::Declaration) -> ParseResult<TokenStream> {
    let decl = match input_decl {
        venial::Declaration::Impl(decl) => decl,
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

fn bail_attr<R>(attr_name: Ident, msg: &str, method: &venial::Function) -> ParseResult<R> {
    bail!(&method.name, "#[{}]: {}", attr_name, msg)
}

/// Holds information known from a signal's definition
struct SignalDefinition {
    /// The signal's function signature.
    signature: venial::Function,

    /// The signal's non-gdext attributes (all except #[signal]).
    external_attributes: Vec<venial::Attribute>,
}

/// Codegen for `#[godot_api] impl MyType`
fn transform_inherent_impl(mut original_impl: venial::Impl) -> ParseResult<TokenStream> {
    let class_name = util::validate_impl(&original_impl, None, "godot_api")?;
    let class_name_obj = util::class_name_obj(&class_name);
    let prv = quote! { ::godot::private };

    let (funcs, signals, out_virtual_impl) = process_godot_fns(&class_name, &mut original_impl)?;

    let signal_registrations = make_signal_registrations(signals, &class_name_obj);

    let method_registrations: Vec<TokenStream> = funcs
        .into_iter()
        .map(|func_def| make_method_registration(&class_name, func_def))
        .collect::<ParseResult<Vec<TokenStream>>>()?; // <- FIXME transpose this

    let constant_registration =
        make_constant_registration(&mut original_impl, &class_name, &class_name_obj)?;

    let result = quote! {
        #original_impl
        #out_virtual_impl

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

fn make_signal_registrations(
    signals: Vec<SignalDefinition>,
    class_name_obj: &TokenStream,
) -> Vec<TokenStream> {
    let mut signal_registrations = Vec::new();

    for signal in signals.iter() {
        let SignalDefinition {
            signature,
            external_attributes,
        } = signal;
        let mut param_types: Vec<venial::TyExpr> = Vec::new();
        let mut param_names: Vec<String> = Vec::new();

        for param in signature.params.inner.iter() {
            match &param.0 {
                venial::FnParam::Typed(param) => {
                    param_types.push(param.ty.clone());
                    param_names.push(param.name.to_string());
                }
                venial::FnParam::Receiver(_) => {}
            };
        }

        let signature_tuple = util::make_signature_tuple_type(&quote! { () }, &param_types);
        let indexes = 0..param_types.len();
        let param_array_decl = quote! {
            [
                // Don't use raw sys pointers directly; it's very easy to have objects going out of scope.
                #(
                    <#signature_tuple as godot::builtin::meta::VarcallSignatureTuple>
                        ::param_property_info(#indexes, #param_names),
                )*
            ]
        };

        // Transport #[cfg] attributes to the FFI glue, to ensure signals which were conditionally
        // removed from compilation don't cause errors.
        let signal_cfg_attrs: Vec<&venial::Attribute> =
            util::extract_cfg_attrs(external_attributes)
                .into_iter()
                .collect();
        let signal_name_str = signature.name.to_string();
        let signal_parameters_count = param_names.len();
        let signal_parameters = param_array_decl;

        let signal_registration = quote! {
            #(#signal_cfg_attrs)*
            unsafe {
                use ::godot::sys;
                let parameters_info: [::godot::builtin::meta::PropertyInfo; #signal_parameters_count] = #signal_parameters;

                let mut parameters_info_sys: [sys::GDExtensionPropertyInfo; #signal_parameters_count] =
                    std::array::from_fn(|i| parameters_info[i].property_sys());

                let signal_name = ::godot::builtin::StringName::from(#signal_name_str);

                sys::interface_fn!(classdb_register_extension_class_signal)(
                    sys::get_library(),
                    #class_name_obj.string_sys(),
                    signal_name.string_sys(),
                    parameters_info_sys.as_ptr(),
                    sys::GDExtensionInt::from(#signal_parameters_count as i64),
                );
            }
        };

        signal_registrations.push(signal_registration);
    }
    signal_registrations
}

fn make_constant_registration(
    original_impl: &mut venial::Impl,
    class_name: &Ident,
    class_name_obj: &TokenStream,
) -> ParseResult<TokenStream> {
    let consts = process_godot_constants(original_impl)?;
    let mut integer_constant_cfg_attrs = Vec::new();
    let mut integer_constant_names = Vec::new();
    let mut integer_constant_values = Vec::new();

    for constant in consts.iter() {
        if constant.initializer.is_none() {
            return bail!(constant, "exported const should have initializer");
        };

        let name = &constant.name;

        // In contrast #[func] and #[signal], we don't remove the attributes from constant signatures
        // within process_godot_constants().
        let cfg_attrs = util::extract_cfg_attrs(&constant.attributes)
            .into_iter()
            .collect::<Vec<_>>();

        // Transport #[cfg] attributes to the FFI glue, to ensure constants which were conditionally removed
        // from compilation don't cause errors.
        integer_constant_cfg_attrs.push(cfg_attrs);
        integer_constant_names.push(constant.name.to_string());
        integer_constant_values.push(quote! { #class_name::#name });
    }

    let tokens = if !integer_constant_names.is_empty() {
        quote! {
            use ::godot::builtin::meta::registration::constant::*;
            use ::godot::builtin::meta::ClassName;
            use ::godot::builtin::StringName;

            #(
                #(#integer_constant_cfg_attrs)*
                ExportConstant::new(
                    #class_name_obj,
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
        TokenStream::new()
    };

    Ok(tokens)
}

fn process_godot_fns(
    class_name: &Ident,
    impl_block: &mut venial::Impl,
) -> ParseResult<(Vec<FuncDefinition>, Vec<SignalDefinition>, TokenStream)> {
    let mut func_definitions = vec![];
    let mut signal_definitions = vec![];
    let mut virtual_functions = vec![];

    let mut removed_indexes = vec![];
    for (index, item) in impl_block.body_items.iter_mut().enumerate() {
        let venial::ImplMember::Method(function) = item else {
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

                        Some(param.name)
                    }
                } else {
                    None
                };

                // For virtual methods, rename/mangle existing user method and create a new method with the original name,
                // which performs a dynamic dispatch.
                if is_virtual {
                    add_virtual_script_call(
                        &mut virtual_functions,
                        function,
                        &signature,
                        class_name,
                        &rename,
                        gd_self_parameter,
                    );
                };

                func_definitions.push(FuncDefinition {
                    signature,
                    external_attributes,
                    rename,
                    is_virtual,
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

    let out_virtual_impl = if virtual_functions.is_empty() {
        TokenStream::new()
    } else {
        quote! {
            impl #class_name {
                #(#virtual_functions)*
            }
        }
    };

    Ok((func_definitions, signal_definitions, out_virtual_impl))
}

fn add_virtual_script_call(
    virtual_functions: &mut Vec<venial::Function>,
    function: &mut venial::Function,
    reduced_signature: &venial::Function,
    class_name: &Ident,
    rename: &Option<String>,
    gd_self_parameter: Option<Ident>,
) {
    assert!(cfg!(since_api = "4.3"));

    let class_name_str = class_name.to_string();
    let early_bound_name = format_ident!("__earlybound_{}", &function.name);
    let method_name_str = rename
        .clone()
        .unwrap_or_else(|| format!("_{}", function.name));

    // Clone might not strictly be necessary, but the 2 other callers of into_signature_info() are better off with pass-by-value.
    let signature_info = into_signature_info(
        reduced_signature.clone(),
        class_name,
        gd_self_parameter.is_some(),
    );

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
        let method_sname_ptr = method_sname.string_sys_const();
        let has_virtual_method = unsafe { ::godot::private::has_virtual_script_method(object_ptr, method_sname_ptr) };

        if has_virtual_method {
            // Dynamic dispatch.
            type CallSig = #sig_tuple;
            let args = (#( #arg_names, )*);
            unsafe {
                <CallSig as ::godot::builtin::meta::VarcallSignatureTuple>::out_script_virtual_call(
                    #class_name_str,
                    #method_name_str,
                    method_sname_ptr,
                    object_ptr,
                    args,
                )
            }
        } else {
            // Fall back to default implementation.
            Self::#early_bound_name(#receiver, #(#arg_names),*)
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

fn process_godot_constants(decl: &mut venial::Impl) -> ParseResult<Vec<venial::Constant>> {
    let mut constant_signatures = vec![];

    for item in decl.body_items.iter_mut() {
        let venial::ImplMember::Constant(constant) = item else {
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
                    constant_signatures.push(constant.clone());
                }
            }
        }
    }

    Ok(constant_signatures)
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

// ----------------------------------------------------------------------------------------------------------------------------------------------

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

/// Codegen for `#[godot_api] impl GodotExt for MyType`
fn transform_trait_impl(original_impl: venial::Impl) -> ParseResult<TokenStream> {
    let (class_name, trait_path) = util::validate_trait_impl_virtual(&original_impl, "godot_api")?;
    let class_name_obj = util::class_name_obj(&class_name);

    let mut godot_init_impl = TokenStream::new();
    let mut to_string_impl = TokenStream::new();
    let mut register_class_impl = TokenStream::new();
    let mut on_notification_impl = TokenStream::new();
    let mut get_property_impl = TokenStream::new();
    let mut set_property_impl = TokenStream::new();

    let mut register_fn = None;
    let mut create_fn = None;
    let mut recreate_fn = None;
    let mut to_string_fn = None;
    let mut on_notification_fn = None;
    let mut get_property_fn = None;
    let mut set_property_fn = None;

    let mut virtual_methods = vec![];
    let mut virtual_method_cfg_attrs = vec![];
    let mut virtual_method_names = vec![];

    let prv = quote! { ::godot::private };

    for item in original_impl.body_items.iter() {
        let method = if let venial::ImplMember::Method(f) = item {
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

    let result = quote! {
        #original_impl
        #godot_init_impl
        #to_string_impl
        #on_notification_impl
        #register_class_impl
        #get_property_impl
        #set_property_impl

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
                get_virtual_fn: #prv::callbacks::get_virtual::<#class_name>,
            },
            init_level: <#class_name as ::godot::obj::GodotClass>::INIT_LEVEL,
        });
    };

    Ok(result)
}
