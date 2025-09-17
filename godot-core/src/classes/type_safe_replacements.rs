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

use crate::classes::{Object, Script};
use crate::meta::{AsObjectArg, GodotFfiVariant};
use crate::obj::Gd;

impl Object {
    pub fn get_script(&self) -> Option<Gd<Script>> {
        let variant = self.raw_get_script();
        if variant.is_nil() {
            None
        } else {
            Some(variant.to())
        }
    }

    pub fn set_script(&mut self, script: impl AsObjectArg<Script>) {
        let variant = script.as_object_arg().ffi_to_variant();
        self.raw_set_script(&variant);
    }

    //    pub fn  connect(
    //     &mut self,
    //     signal: impl AsArg<StringName>,
    //     callable: &Callable,
    // ) -> Error {
    //
    //    }
}
