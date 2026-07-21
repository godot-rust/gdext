/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! Class registration: turns a user's `#[godot_api] impl I* for MyClass` into the callbacks Godot invokes.
//!
//! # Naming of lifecycle callbacks
//! Layers godot-rust owns are named after the user-facing `I*` virtual; those 1:1 with GDExtension use Godot's slot names.
//! Example `on_get`, from outermost to innermost:
//!
//! - `godot-macros`, `interface_trait_impl::TraitImpl::handle_on_get()` -- generates the code below.
//! - `godot-macros`, `interface_trait_impl::Decls::on_get_impl` -- holds the generated `cap` trait impl.
//! - [`shard::ITraitImpl::with_on_get()`] -- registers the callback; the sole point where the naming switches sides.
//! - `callbacks::get()` -- the `extern "C" fn` filling `GDExtensionClassCreationInfo::get_func`, via field `ITraitImpl::user_get_fn`.
//! - `handle_method_panic()`'s context, reported as `MyClass::on_get()` -- diagnostics, so user-facing again.
//! - [`cap::GodotGet::__godot_on_get()`][crate::obj::cap::GodotGet] -- calls the user's `on_get()`.
//!
//! Both sides are needed since the mapping isn't 1:1: `on_property_get_revert` installs the `property_can_revert` and
//! `property_get_revert` slots, `on_get_property_list` additionally `free_property_list`.

// Note: final re-exports from godot-core are in lib.rs, mod register::private.
// These are public here for simplicity, but many are not imported by the main crate.

pub mod callbacks;
pub mod class;
pub mod constant;
pub mod info;
pub mod method;
pub mod property;
pub mod shard;

// RpcConfig uses MultiplayerPeer::TransferMode and MultiplayerApi::RpcMode, which are only enabled in `codegen-full` feature.
#[cfg(feature = "codegen-full")]
mod rpc_config;
#[cfg(feature = "codegen-full")]
pub use rpc_config::RpcConfig;

#[doc(hidden)]
pub mod godot_register_wrappers;
