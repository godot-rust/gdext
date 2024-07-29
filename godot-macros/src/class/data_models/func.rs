/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::util::{bail_fn, ident, safe_ident};
use crate::{util, ParseResult};
use proc_macro2::{Group, Ident, TokenStream, TokenTree};
use quote::{format_ident, quote};

/// Information used for registering a Rust function with Godot.
pub struct FuncDefinition {
    /// Refined signature, with higher level info and renamed parameters.
    pub signature_info: SignatureInfo,
    /// The function's non-gdext attributes (all except #[func]).
    pub external_attributes: Vec<venial::Attribute>,
    /// The name the function will be exposed as in Godot. If `None`, the Rust function name is used.
    pub rename: Option<String>,
    pub is_script_virtual: bool,
}

/// Returns a C function which acts as the callback when a virtual method of this instance is invoked.
//
// Virtual methods are non-static by their nature; so there's no support for static ones.
pub fn make_virtual_callback(
    class_name: &Ident,
    signature_info: SignatureInfo,
    before_kind: BeforeKind,
) -> TokenStream {
    let method_name = &signature_info.method_name;

    let wrapped_method = make_forwarding_closure(class_name, &signature_info, before_kind);
    let sig_tuple = signature_info.tuple_type();

    let call_ctx = make_call_context(
        class_name.to_string().as_str(),
        method_name.to_string().as_str(),
    );
    let invocation = make_ptrcall_invocation(&wrapped_method, true);

    quote! {
        {
            use ::godot::sys;
            type Sig = #sig_tuple;

            unsafe extern "C" fn virtual_fn(
                instance_ptr: sys::GDExtensionClassInstancePtr,
                args_ptr: *const sys::GDExtensionConstTypePtr,
                ret: sys::GDExtensionTypePtr,
            ) {
                let call_ctx = #call_ctx;
                let _success = ::godot::private::handle_ptrcall_panic(
                    &call_ctx,
                    || #invocation
                );
            }
            Some(virtual_fn)
        }
    }
}

/// Generates code that registers the specified method for the given class.
pub fn make_method_registration(
    class_name: &Ident,
    func_definition: FuncDefinition,
) -> ParseResult<TokenStream> {
    let signature_info = &func_definition.signature_info;
    let sig_tuple = signature_info.tuple_type();

    let is_script_virtual = func_definition.is_script_virtual;
    let method_flags = match make_method_flags(signature_info.receiver_type, is_script_virtual) {
        Ok(mf) => mf,
        Err(msg) => return bail_fn(msg, &signature_info.method_name),
    };

    let forwarding_closure =
        make_forwarding_closure(class_name, signature_info, BeforeKind::Without);

    // String literals
    let method_name = &signature_info.method_name;
    let class_name_str = class_name.to_string();
    let method_name_str = if let Some(rename) = func_definition.rename {
        rename
    } else {
        method_name.to_string()
    };

    let call_ctx = make_call_context(&class_name_str, &method_name_str);
    let varcall_fn_decl = make_varcall_fn(&call_ctx, &forwarding_closure);
    let ptrcall_fn_decl = make_ptrcall_fn(&call_ctx, &forwarding_closure);

    // String literals II
    let param_ident_strs = signature_info
        .param_idents
        .iter()
        .map(|ident| ident.to_string());

    // Transport #[cfg] attrs to the FFI glue to ensure functions which were conditionally
    // removed from compilation don't cause errors.
    let cfg_attrs = util::extract_cfg_attrs(&func_definition.external_attributes)
        .into_iter()
        .collect::<Vec<_>>();

    let registration = quote! {
        #(#cfg_attrs)*
        {
            use ::godot::obj::GodotClass;
            use ::godot::register::private::method::ClassMethodInfo;
            use ::godot::builtin::{StringName, Variant};
            use ::godot::sys;

            type Sig = #sig_tuple;

            let method_name = StringName::from(#method_name_str);

            #varcall_fn_decl;
            #ptrcall_fn_decl;

            // SAFETY:
            // `get_varcall_func` upholds all the requirements for `call_func`.
            // `get_ptrcall_func` upholds all the requirements for `ptrcall_func`
            let method_info = unsafe {
                ClassMethodInfo::from_signature::<Sig>(
                    #class_name::class_name(),
                    method_name,
                    Some(varcall_fn),
                    Some(ptrcall_fn),
                    #method_flags,
                    &[
                        #( #param_ident_strs ),*
                    ],
                    Vec::new()
                )
            };

            ::godot::private::out!(
                "   Register fn:   {}::{}",
                #class_name_str,
                #method_name_str
            );

            // Note: information whether the method is virtual is stored in method method_info's flags.
            method_info.register_extension_class_method();
        };
    };

    Ok(registration)
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Implementation

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum ReceiverType {
    Ref,
    Mut,
    GdSelf,
    Static,
}

#[derive(Debug)]
pub struct SignatureInfo {
    pub method_name: Ident,
    pub receiver_type: ReceiverType,
    pub param_idents: Vec<Ident>,
    pub param_types: Vec<venial::TypeExpr>,
    pub ret_type: TokenStream,
}

impl SignatureInfo {
    pub fn fn_ready() -> Self {
        Self {
            method_name: ident("ready"),
            receiver_type: ReceiverType::Mut,
            param_idents: vec![],
            param_types: vec![],
            ret_type: quote! { () },
        }
    }

    pub fn tuple_type(&self) -> TokenStream {
        // Note: for GdSelf receivers, first parameter is not even part of SignatureInfo anymore.
        util::make_signature_tuple_type(&self.ret_type, &self.param_types)
    }
}

pub enum BeforeKind {
    /// Default: just call the method.
    Without,

    /// Call `before_{method}` before calling the method itself.
    WithBefore,

    /// Call **only** `before_{method}`, not the method itself.
    OnlyBefore,
}

/// Returns a closure expression that forwards the parameters to the Rust instance.
fn make_forwarding_closure(
    class_name: &Ident,
    signature_info: &SignatureInfo,
    before_kind: BeforeKind,
) -> TokenStream {
    let method_name = &signature_info.method_name;
    let params = &signature_info.param_idents;

    let instance_decl = match &signature_info.receiver_type {
        ReceiverType::Ref => quote! {
            let instance = ::godot::private::Storage::get(storage);
        },
        ReceiverType::Mut => quote! {
            let mut instance = ::godot::private::Storage::get_mut(storage);
        },
        _ => quote! {},
    };

    let before_method_call = match before_kind {
        BeforeKind::WithBefore | BeforeKind::OnlyBefore => {
            let before_method = format_ident!("__before_{}", method_name);
            quote! { instance.#before_method(); }
        }
        BeforeKind::Without => TokenStream::new(),
    };

    match signature_info.receiver_type {
        ReceiverType::Ref | ReceiverType::Mut => {
            // Generated default virtual methods (e.g. for ready) may not have an actual implementation (user code), so
            // all they need to do is call the __before_ready() method. This means the actual method call may be optional.
            let method_call = if matches!(before_kind, BeforeKind::OnlyBefore) {
                TokenStream::new()
            } else {
                quote! { instance.#method_name( #(#params),* ) }
            };

            quote! {
                |instance_ptr, params| {
                    let ( #(#params,)* ) = params;

                    let storage =
                        unsafe { ::godot::private::as_storage::<#class_name>(instance_ptr) };

                    #instance_decl
                    #before_method_call
                    #method_call
                }
            }
        }
        ReceiverType::GdSelf => {
            // Method call is always present, since GdSelf implies that the user declares the method.
            // (Absent method is only used in the case of a generated default virtual method, e.g. for ready()).
            quote! {
                |instance_ptr, params| {
                    let ( #(#params,)* ) = params;

                    let storage =
                        unsafe { ::godot::private::as_storage::<#class_name>(instance_ptr) };

                    #before_method_call
                    #class_name::#method_name(::godot::private::Storage::get_gd(storage), #(#params),*)
                }
            }
        }
        ReceiverType::Static => {
            // No before-call needed, since static methods are not virtual.
            quote! {
                |_, params| {
                    let ( #(#params,)* ) = params;
                    #class_name::#method_name(#(#params),*)
                }
            }
        }
    }
}

/// Maps each usage of `Self` to the struct it's referencing,
/// since `Self` can't be used inside nested functions.
fn map_self_to_class_name<In, Out>(tokens: In, class_name: &Ident) -> Out
where
    In: IntoIterator<Item = TokenTree>,
    Out: FromIterator<TokenTree>,
{
    tokens
        .into_iter()
        .map(|tt| match tt {
            // Change instances of Self to the class name.
            TokenTree::Ident(ident) if ident == "Self" => TokenTree::Ident(class_name.clone()),
            // Recurse into groups and make sure ALL instances are changed.
            TokenTree::Group(group) => TokenTree::Group(Group::new(
                group.delimiter(),
                map_self_to_class_name(group.stream(), class_name),
            )),
            // Pass all other tokens through unchanged.
            tt => tt,
        })
        .collect()
}

pub(crate) fn into_signature_info(
    signature: venial::Function,
    class_name: &Ident,
    has_gd_self: bool,
) -> SignatureInfo {
    let method_name = signature.name.clone();
    let mut receiver_type = if has_gd_self {
        ReceiverType::GdSelf
    } else {
        ReceiverType::Static
    };

    let num_params = signature.params.inner.len();
    let mut param_idents = Vec::with_capacity(num_params);
    let mut param_types = Vec::with_capacity(num_params);
    let ret_type = match signature.return_ty {
        None => quote! { () },
        Some(ty) => map_self_to_class_name(ty.tokens, class_name),
    };

    let mut next_unnamed_index = 0;
    for (arg, _) in signature.params.inner {
        match arg {
            venial::FnParam::Receiver(recv) => {
                if receiver_type == ReceiverType::GdSelf {
                    // This shouldn't happen, as when has_gd_self is true the first function parameter should have been removed.
                    // And the first parameter should be the only one that can be a Receiver.
                    panic!("has_gd_self is true for a signature starting with a Receiver param.");
                }
                receiver_type = if recv.tk_mut.is_some() {
                    ReceiverType::Mut
                } else if recv.tk_ref.is_some() {
                    ReceiverType::Ref
                } else {
                    panic!("Receiver not supported");
                };
            }
            venial::FnParam::Typed(arg) => {
                let ident = maybe_rename_parameter(arg.name, &mut next_unnamed_index);
                let ty = venial::TypeExpr {
                    tokens: map_self_to_class_name(arg.ty.tokens, class_name),
                };

                param_types.push(ty);
                param_idents.push(ident);
            }
        }
    }

    SignatureInfo {
        method_name,
        receiver_type,
        param_idents,
        param_types,
        ret_type,
    }
}

pub(crate) fn maybe_rename_parameter(param_ident: Ident, next_unnamed_index: &mut i32) -> Ident {
    // Parameter will be forwarded as an argument to the instance, so we need to give `_` a name.
    let param_str = param_ident.to_string(); // a pity that Ident has no string operations.

    if param_str == "_" {
        let ident = format_ident!("__unnamed_{next_unnamed_index}");
        *next_unnamed_index += 1;
        ident
    } else if let Some(remain) = param_str.strip_prefix('_') {
        // If parameters are currently unused, still use the actual name, as "used-ness" is an implementation detail.
        // This could technically collide with another parameter of the same name (without "_"), but that's very unlikely and not
        // something we really need to support.
        // Note that the case of a single "_" is handled above.
        safe_ident(remain)
    } else {
        param_ident
    }
}

fn make_method_flags(
    method_type: ReceiverType,
    is_script_virtual: bool,
) -> Result<TokenStream, String> {
    let flags = quote! { ::godot::global::MethodFlags };

    let base_flags = match method_type {
        ReceiverType::Ref => {
            quote! { #flags::NORMAL | #flags::CONST }
        }
        // Conservatively assume Gd<Self> receivers to mutate the object, since user can call bind_mut().
        ReceiverType::Mut | ReceiverType::GdSelf => {
            quote! { #flags::NORMAL }
        }
        ReceiverType::Static => {
            if is_script_virtual {
                return Err(
                    "#[func(virtual)] is not allowed for associated (static) functions".to_string(),
                );
            }
            quote! { #flags::NORMAL | #flags::STATIC }
        }
    };

    let flags = if is_script_virtual {
        quote! { #base_flags | #flags::VIRTUAL }
    } else {
        base_flags
    };

    Ok(flags)
}

/// Generate code for a C FFI function that performs a varcall.
fn make_varcall_fn(call_ctx: &TokenStream, wrapped_method: &TokenStream) -> TokenStream {
    let invocation = make_varcall_invocation(wrapped_method);

    // TODO reduce amount of code generated, by delegating work to a library function. Could even be one that produces this function pointer.
    quote! {
        unsafe extern "C" fn varcall_fn(
            _method_data: *mut std::ffi::c_void,
            instance_ptr: sys::GDExtensionClassInstancePtr,
            args_ptr: *const sys::GDExtensionConstVariantPtr,
            arg_count: sys::GDExtensionInt,
            ret: sys::GDExtensionVariantPtr,
            err: *mut sys::GDExtensionCallError,
        ) {
            let call_ctx = #call_ctx;
            ::godot::private::handle_varcall_panic(
                &call_ctx,
                &mut *err,
                || #invocation
            );
        }
    }
}

/// Generate code for a C FFI function that performs a ptrcall.
fn make_ptrcall_fn(call_ctx: &TokenStream, wrapped_method: &TokenStream) -> TokenStream {
    let invocation = make_ptrcall_invocation(wrapped_method, false);

    quote! {
        unsafe extern "C" fn ptrcall_fn(
            _method_data: *mut std::ffi::c_void,
            instance_ptr: sys::GDExtensionClassInstancePtr,
            args_ptr: *const sys::GDExtensionConstTypePtr,
            ret: sys::GDExtensionTypePtr,
        ) {
            let call_ctx = #call_ctx;
            let _success = ::godot::private::handle_panic(
                || &call_ctx,
                || #invocation
            );

            // if success.is_err() {
            //     // TODO set return value to T::default()?
            // }
        }
    }
}

/// Generate code for a `ptrcall` call expression.
fn make_ptrcall_invocation(wrapped_method: &TokenStream, is_virtual: bool) -> TokenStream {
    let ptrcall_type = if is_virtual {
        quote! { sys::PtrcallType::Virtual }
    } else {
        quote! { sys::PtrcallType::Standard }
    };

    quote! {
         <Sig as ::godot::meta::PtrcallSignatureTuple>::in_ptrcall(
            instance_ptr,
            &call_ctx,
            args_ptr,
            ret,
            #wrapped_method,
            #ptrcall_type,
        )
    }
}

/// Generate code for a `varcall()` call expression.
fn make_varcall_invocation(wrapped_method: &TokenStream) -> TokenStream {
    quote! {
        <Sig as ::godot::meta::VarcallSignatureTuple>::in_varcall(
            instance_ptr,
            &call_ctx,
            args_ptr,
            arg_count,
            ret,
            err,
            #wrapped_method,
        )
    }
}

fn make_call_context(class_name_str: &str, method_name_str: &str) -> TokenStream {
    quote! {
        ::godot::meta::CallContext::func(#class_name_str, #method_name_str)
    }
}
