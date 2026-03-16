/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use proc_macro2::{Ident, TokenStream};
use quote::{TokenStreamExt, format_ident, quote};

use crate::class::FuncDefinition;

/// Possible ways the user can specify RPC configuration.
pub enum RpcAttr {
    // Individual keys in the `rpc` attribute.
    // Example: `#[rpc(any_peer, reliable, call_remote, channel = 3)]`
    SeparatedArgs {
        rpc_mode: Option<RpcMode>,
        transfer_mode: Option<TransferMode>,
        call_local: Option<bool>,
        channel: Option<u32>,
    },

    // `args` key in the `rpc` attribute.
    // Example:
    // const RPC_CFG: RpcConfig = RpcConfig { mode: RpcMode::Authority, ..RpcConfig::default() };
    // #[rpc(config = RPC_CFG)]
    Expression(TokenStream),
}

#[derive(Copy, Clone)]
pub enum RpcMode {
    AnyPeer,
    Authority,
}

impl RpcMode {
    pub fn from_usize(value: usize) -> Option<Self> {
        match value {
            0 => Some(RpcMode::AnyPeer),
            1 => Some(RpcMode::Authority),
            _ => None,
        }
    }
}

#[derive(Copy, Clone)]
pub enum TransferMode {
    Reliable,
    Unreliable,
    UnreliableOrdered,
}

impl TransferMode {
    pub fn from_usize(value: usize) -> Option<Self> {
        match value {
            0 => Some(TransferMode::Reliable),
            1 => Some(TransferMode::Unreliable),
            2 => Some(TransferMode::UnreliableOrdered),
            _ => None,
        }
    }
}

pub fn make_rpc_registrations(
    class_name: &Ident,
    funcs: &[FuncDefinition],
    no_typed_rpcs: bool,
) -> (TokenStream, Option<TokenStream>) {
    let rpc_registrations = funcs
        .iter()
        .filter_map(make_rpc_registration)
        .collect::<Vec<TokenStream>>();

    // This check is necessary because the class might not implement `WithBaseField` or `Inherits<Node>`,
    //   which means `to_gd` wouldn't exist or the trait bounds on `RpcConfig::register` wouldn't be satisfied.
    if rpc_registrations.is_empty() {
        return (TokenStream::new(), None);
    }

    (
        if cfg!(feature = "codegen-full") {
            quote! {
                // Clippy complains about using `..RpcConfig::default()` if all fields are overridden.
                #[allow(clippy::needless_update)]
                fn __register_rpcs(object: &mut dyn ::std::any::Any) {
                    use ::std::any::Any;
                    use ::godot::register::RpcConfig;
                    use ::godot::classes::multiplayer_api::RpcMode;
                    use ::godot::classes::multiplayer_peer::TransferMode;
                    use ::godot::classes::Node;
                    use ::godot::obj::{WithBaseField, Gd};

                    let this = object
                        .downcast_ref::<#class_name>()
                        .expect("bad type erasure when registering RPCs");

                    // Use fully-qualified syntax, so that error message isn't just "no method named `to_gd` found".
                    let mut gd = ::godot::obj::WithBaseField::to_gd(this);

                    let node = gd.upcast_mut::<Node>();
                    #( #rpc_registrations )*
                }
            }
        } else {
            TokenStream::new()
        },
        if !no_typed_rpcs {
            make_rpc_api(
                class_name,
                funcs
                    .iter()
                    .filter(|func| func.rpc_info.is_some())
                    .collect(),
            )
        } else {
            None
        },
    )
}

fn make_rpc_registration(func_def: &FuncDefinition) -> Option<TokenStream> {
    let rpc_info = func_def.rpc_info.as_ref()?;

    let create_struct = match rpc_info {
        RpcAttr::SeparatedArgs {
            rpc_mode,
            transfer_mode,
            call_local,
            channel,
        } => {
            let override_rpc_mode = rpc_mode.map(|mode| {
                let token = match mode {
                    RpcMode::Authority => quote! { RpcMode::AUTHORITY },
                    RpcMode::AnyPeer => quote! { RpcMode::ANY_PEER },
                };

                quote! { rpc_mode: #token, }
            });

            let override_transfer_mode = transfer_mode.map(|mode| {
                let token = match mode {
                    TransferMode::Reliable => quote! { TransferMode::RELIABLE },
                    TransferMode::Unreliable => quote! { TransferMode::UNRELIABLE },
                    TransferMode::UnreliableOrdered => quote! { TransferMode::UNRELIABLE_ORDERED },
                };

                quote! { transfer_mode: #token, }
            });

            let override_call_local = call_local.map(|call_local| {
                quote! { call_local: #call_local, }
            });

            let override_channel = channel.map(|channel| {
                quote! { channel: #channel, }
            });

            quote! {
                let args = RpcConfig {
                    #override_rpc_mode
                    #override_transfer_mode
                    #override_call_local
                    #override_channel
                    ..RpcConfig::default()
                };
            }
        }
        RpcAttr::Expression(expr) => {
            quote! { let args = #expr; }
        }
    };

    let method_name_str = func_def.godot_name();

    let registration = quote! {
        {
            #create_struct
            args.configure_node(node, #method_name_str)
        }
    };

    Some(registration)
}

// TODO: respect function visibility when generating builders
pub fn make_rpc_api(class_name: &Ident, rpcs: Vec<&FuncDefinition>) -> Option<TokenStream> {
    if rpcs.is_empty() {
        return None;
    }

    let collection_name = format_ident!("__godot_Rpcs_{class_name}", span = class_name.span());

    let mut collection_impl_methods = TokenStream::new();
    for rpc in rpcs {
        let rust_name = rpc.rust_ident();
        let rpc_name = &rpc.godot_name();
        let param_idents = &rpc.signature_info.param_idents;
        let rpc_typed_args = param_idents
            .iter()
            .zip(&rpc.signature_info.param_types)
            .map(|(param_name, param_type)| {
                quote! {
                    #param_name: impl ::godot::meta::AsArg<#param_type>
                }
            });

        collection_impl_methods.append_all(quote! {
            #[must_use]
            pub fn #rust_name(self, #( #rpc_typed_args ),*) -> RpcBuilder<'c, #class_name> {
                RpcBuilder::new(
                    self.object,
                    #rpc_name,
                    vec![#( ::godot::meta::ToGodot::to_variant(&#param_idents.into_arg()) ),*],
                )
            }
        });
    }

    let rpc_mod = format_ident!("__{class_name}_rpcs");
    Some(quote! {
        use #rpc_mod::*;

        #[allow(non_camel_case_types)]
        mod #rpc_mod {
            use super::*;
            use ::godot::obj::rpc::{RpcCollection, UserRpcObject, RpcBuilder};

            #[doc(hidden)]
            pub struct #collection_name<'c> {
                object: UserRpcObject<'c, #class_name>,
            }

            impl<'c> #collection_name<'c> {
                #collection_impl_methods
            }

            impl<'c> RpcCollection<'c, #class_name> for #collection_name<'c> {
                fn from_user_rpc_object(object: UserRpcObject<'c, #class_name>) -> Self {
                    #collection_name {
                        object,
                    }
                }
            }

            impl<'c> ::godot::obj::WithUserRpcs<'c, #class_name> for #class_name
            where
                #class_name: ::godot::obj::WithBaseField,
            {
                type Collection = #collection_name<'c>;

                fn rpcs(&'c mut self) -> Self::Collection {
                    Self::Collection::from_user_rpc_object(
                        UserRpcObject::Internal(self)
                    )
                }
            }
        }
    })
}
