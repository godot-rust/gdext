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
use crate::classes::{Object, SceneTree, Script};
use crate::global::Error;
use crate::meta::{arg_into_ref, AsArg, ToGodot};
use crate::obj::{EngineBitfield, Gd};

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
        self.raw_connect(signal, callable)
    }

    pub fn connect_flags(
        &mut self,
        signal: impl AsArg<StringName>,
        callable: &Callable,
        flags: ConnectFlags,
    ) -> Error {
        self.raw_connect_ex(signal, callable)
            .flags(flags.ord() as u32)
            .done()
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

#[cfg(feature = "codegen-full")]
mod codegen_full {
    // For future, expanding manual replacements for classes in the codegen-full set.
}
