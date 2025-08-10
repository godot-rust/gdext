/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use proc_macro2::{Ident, TokenStream};
use quote::quote;

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

pub fn make_rpc_registrations_fn(class_name: &Ident, funcs: &[FuncDefinition]) -> TokenStream {
    let rpc_registrations = funcs
        .iter()
        .filter_map(make_rpc_registration)
        .collect::<Vec<TokenStream>>();

    // This check is necessary because the class might not implement `WithBaseField` or `Inherits<Node>`,
    //   which means `to_gd` wouldn't exist or the trait bounds on `RpcConfig::register` wouldn't be satisfied.
    if rpc_registrations.is_empty() {
        return TokenStream::new();
    }

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
