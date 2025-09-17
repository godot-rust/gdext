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
use crate::obj::{EngineBitfield, EngineEnum, Gd};

#[cfg(feature = "codegen-full")]
mod codegen_full {
    use super::*;
    use crate::builtin::{Color, Transform2D, Transform3D, Vector2, Vector3};
    use crate::classes::file_access::{ExRawCreateTemp, ModeFlags};
    use crate::classes::gpu_particles_2d::EmitFlags as EmitFlags2D;
    use crate::classes::gpu_particles_3d::EmitFlags as EmitFlags3D;
    use crate::classes::tree::DropModeFlags;
    use crate::classes::{FileAccess, GpuParticles2D, GpuParticles3D, Tree};
    use crate::obj::Gd;

    impl Tree {
        /// Set drop mode flags with type-safe enum instead of raw integer.
        pub fn set_drop_mode_flags(&mut self, flags: DropModeFlags) {
            self.raw_set_drop_mode_flags(flags.ord())
        }

        /// Get drop mode flags as type-safe enum instead of raw integer.
        pub fn get_drop_mode_flags(&self) -> DropModeFlags {
            DropModeFlags::from_ord(self.raw_get_drop_mode_flags())
        }
    }

    impl FileAccess {
        /// Create a temporary file with type-safe mode flags.
        pub fn create_temp(mode_flags: ModeFlags) -> Option<Gd<FileAccess>> {
            Self::raw_create_temp(mode_flags.ord() as i32)
        }

        // FIXME: warning: type `gen::classes::file_access::ExRawCreateTemp<'a>` is more private than the item `codegen_full::<impl gen::classes::file_access::re_export::FileAccess>::create_temp_ex`
        pub fn create_temp_ex<'a>(mode_flags: ModeFlags) -> ExRawCreateTemp<'a> {
            Self::raw_create_temp_ex(mode_flags.ord() as i32)
        }
    }

    impl GpuParticles2D {
        /// Emit a particle with type-safe emit flags.
        pub fn emit_particle(
            &mut self,
            xform: Transform2D,
            velocity: Vector2,
            color: Color,
            custom: Color,
            flags: EmitFlags2D,
        ) {
            self.raw_emit_particle(xform, velocity, color, custom, flags.ord() as u32)
        }
    }

    impl GpuParticles3D {
        /// Emit a particle with type-safe emit flags.
        pub fn emit_particle(
            &mut self,
            xform: Transform3D,
            velocity: Vector3,
            color: Color,
            custom: Color,
            flags: EmitFlags3D,
        ) {
            self.raw_emit_particle(xform, velocity, color, custom, flags.ord() as u32)
        }
    }
}

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
