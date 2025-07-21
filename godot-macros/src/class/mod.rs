/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

mod derive_godot_class;
mod godot_api;
mod godot_dyn;

mod data_models {
    pub mod constant;
    pub mod field;
    pub mod field_export;
    pub mod field_var;
    pub mod fields;
    pub mod func;
    pub mod group_export;
    pub mod inherent_impl;
    pub mod interface_trait_impl;
    pub mod property;
    #[cfg_attr(not(feature = "codegen-full"), allow(dead_code))]
    pub mod rpc;
    pub mod signal;
}

pub(crate) use data_models::constant::*;
pub(crate) use data_models::field::*;
pub(crate) use data_models::field_export::*;
pub(crate) use data_models::field_var::*;
pub(crate) use data_models::func::*;
pub(crate) use data_models::inherent_impl::*;
pub(crate) use data_models::interface_trait_impl::*;
pub(crate) use data_models::property::*;
pub(crate) use data_models::rpc::*;
pub(crate) use data_models::signal::*;
pub(crate) use derive_godot_class::*;
pub(crate) use godot_api::*;
pub(crate) use godot_dyn::*;
