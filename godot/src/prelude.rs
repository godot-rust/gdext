/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

pub use super::register::property::{Export, Var};

// Re-export macros.
pub use super::register::{godot_api, godot_dyn, Export, GodotClass, GodotConvert, Var};

pub use super::builtin::__prelude_reexport::*;
pub use super::builtin::math::FloatExt as _;
pub use super::meta::error::{ConvertError, IoError};
pub use super::meta::{FromGodot, GodotConvert, ToGodot};

pub use super::classes::{
    AudioStreamPlayer, Camera2D, Camera3D, IAudioStreamPlayer, ICamera2D, ICamera3D, INode,
    INode2D, INode3D, IObject, IPackedScene, IRefCounted, IResource, ISceneTree, Input, Node,
    Node2D, Node3D, Object, PackedScene, RefCounted, Resource, SceneTree,
};
pub use super::global::{
    godot_error, godot_print, godot_print_rich, godot_script_error, godot_warn,
};
pub use super::tools::{load, save, try_load, try_save, GFile};

pub use super::init::{gdextension, ExtensionLibrary, InitLevel};
pub use super::obj::{
    AsDyn, Base, DynGd, DynGdMut, DynGdRef, Gd, GdMut, GdRef, GodotClass, Inherits, InstanceId,
    OnReady,
};

// Make trait methods available.
pub use super::obj::cap::WithFuncs as _; // funcs()
pub use super::obj::cap::WithSignals as _; // signals()
pub use super::obj::WithBaseField as _; // base(), base_mut(), to_gd()

pub use super::obj::EngineBitfield as _;
pub use super::obj::EngineEnum as _;
pub use super::obj::NewAlloc as _;
pub use super::obj::NewGd as _;
