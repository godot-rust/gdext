/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

pub use super::builtin::__prelude_reexport::*;
pub use super::classes::{
    INode, INode2D, INode3D, IObject, IPackedScene, IRefCounted, IResource, ISceneTree, Node,
    Node2D, Node3D, Object, PackedScene, RefCounted, Resource, SceneTree, match_class,
};
pub use super::global::{
    godot_error, godot_print, godot_print_rich, godot_script_error, godot_warn,
};
pub use super::init::{ExtensionLibrary, InitLevel, InitStage, gdextension};
pub use super::meta::error::{ConvertError, IoError};
pub use super::meta::{FromGodot, GodotConvert, ToGodot};
pub use super::obj::{
    AsDyn, Base, DynGd, DynGdMut, DynGdRef, Gd, GdMut, GdRef, GodotClass, Inherits, InstanceId,
    OnEditor, OnReady, UserSingleton,
};
pub use super::register::property::{Export, ExportToolButton, PhantomVar, Var};
// Re-export macros.
pub use super::register::{Export, GodotClass, GodotConvert, Var, godot_api, godot_dyn};
pub use super::tools::{GFile, load, save, try_load, try_save};

// Make trait methods available.
#[rustfmt::skip] // One per line.
mod trait_reexports {
    pub use crate::builtin::math::FloatExt as _;
    pub use crate::obj::EngineBitfield as _;
    pub use crate::obj::EngineEnum as _;
    pub use crate::obj::NewAlloc as _;
    pub use crate::obj::NewGd as _;
    pub use crate::obj::Singleton as _; // singleton()
    pub use crate::obj::WithBaseField as _; // base(), base_mut(), to_gd(), run_deferred(), run_deferred_gd()
    pub use crate::obj::WithSignals as _; // Gd::signals()
    pub use crate::obj::WithUserSignals as _; // self.signals()
}

pub use trait_reexports::*;
