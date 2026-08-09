/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! Replaces existing Godot APIs with more type-safe ones where appropriate.
//!
//! Each entry here *must* be accompanied by
//!
//! See also sister module [super::manual_extensions].

use crate::builtin::{Callable, GString, StringName, Variant};
use crate::classes::notify::NodeNotification;
use crate::classes::object::ConnectFlags;
use crate::classes::scene_tree::GroupCallFlags;
use crate::classes::{Node, Object, SceneTree, Script};
use crate::global::Error;
use crate::meta::{AsArg, ToGodot, arg_into_owned, arg_into_ref};
use crate::obj::{EngineBitfield, Gd};
use crate::signal::store_custom_callable_connection;

impl Object {
    pub fn get_script(&self) -> Option<Gd<Script>> {
        let variant = self.raw_get_script();
        if variant.is_nil() {
            None
        } else {
            Some(variant.to())
        }
    }

    pub fn set_script(&mut self, script: impl AsArg<Option<Gd<Script>>>) {
        arg_into_ref!(script);

        self.raw_set_script(&script.to_variant());
    }

    pub fn connect(&mut self, signal: impl AsArg<StringName>, callable: &Callable) -> Error {
        arg_into_owned!(signal);
        let result = self.raw_connect(&signal, callable);
        track_callable_connection(self, &signal, callable);
        result
    }

    pub fn connect_flags(
        &mut self,
        signal: impl AsArg<StringName>,
        callable: &Callable,
        flags: ConnectFlags,
    ) -> Error {
        arg_into_owned!(signal);
        let result = self
            .raw_connect_ex(&signal, callable)
            .flags(flags.ord() as u32)
            .done();
        track_callable_connection(self, &signal, callable);
        result
    }
}

/// Registers a custom-callable connection so it can be auto-disconnected before hot reload.
///
/// Called by `Object::connect` / `Object::connect_flags` APIs, used both directly and through typed signal APIs. Ignores non-custom callables.
fn track_callable_connection(receiver: &Object, signal_name: &StringName, callable: &Callable) {
    // Only the editor needs the registry; skip the weak-`Gd` construction entirely outside it.
    if !crate::sys::is_editor() {
        return;
    }

    // SAFETY: `receiver` is a live `Object` reference for the duration of this call. The weak `Gd` is passed to
    // `store_custom_callable_connection` (which clones its own handle) and disposed of here via `drop_weak`.
    let weak_gd: Gd<Object> = unsafe { Gd::from_obj_sys_weak(receiver.__object_ptr()) };
    store_custom_callable_connection(&weak_gd, signal_name, callable);
    weak_gd.drop_weak();
}

impl Node {
    /// ⚠️ Assuming the node is inside a scene tree, obtains the latter.
    ///
    /// # Panics
    /// If the node is not inside the scene tree. If you're unsure, use [`get_tree_or_null()`][Self::get_tree_or_null].
    pub fn get_tree(&self) -> Gd<SceneTree> {
        // Don't call get_tree_or_null() to avoid extra FFI call.
        // If the invariant is wrong, this panics and Godot additionally prints its own error.
        self.raw_get_tree()
            .unwrap_or_else(|| panic!("node outside scene tree; use get_tree_or_null() instead"))
    }

    /// Fallibly obtains the scene tree containing the node, or `None`.
    pub fn get_tree_or_null(&self) -> Option<Gd<SceneTree>> {
        self.is_inside_tree().then(|| self.get_tree())
    }
}

impl SceneTree {
    // Note: this creates different order between call_group(), call_group_flags() in docs.
    // Maybe worth redeclaring those as well?

    pub fn call_group_flags(
        &mut self,
        flags: GroupCallFlags,
        group: impl AsArg<StringName>,
        method: impl AsArg<StringName>,
        varargs: &[Variant],
    ) {
        self.raw_call_group_flags(flags.ord() as i64, group, method, varargs)
    }

    pub fn set_group_flags(
        &mut self,
        call_flags: GroupCallFlags,
        group: impl AsArg<StringName>,
        property: impl AsArg<GString>,
        value: &Variant,
    ) {
        self.raw_set_group_flags(call_flags.ord() as u32, group, property, value)
    }

    /// Assumes notifications of `Node`. To relay those of derived constants, use [`NodeNotification::Unknown`].
    pub fn notify_group(&mut self, group: impl AsArg<StringName>, notification: NodeNotification) {
        self.raw_notify_group(group, notification.into())
    }

    /// Assumes notifications of `Node`. To relay those of derived constants, use [`NodeNotification::Unknown`].
    pub fn notify_group_flags(
        &mut self,
        call_flags: GroupCallFlags,
        group: impl AsArg<StringName>,
        notification: NodeNotification,
    ) {
        self.raw_notify_group_flags(call_flags.ord() as u32, group, notification.into())
    }
}

#[cfg(feature = "codegen-full")] #[cfg_attr(published_docs, doc(cfg(feature = "codegen-full")))]
mod codegen_full {
    // For future, expanding manual replacements for classes in the codegen-full set.
}
