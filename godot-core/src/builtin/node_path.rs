/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::builtin::GodotString;
use godot_ffi as sys;
use godot_ffi::{ffi_methods, GodotFfi};
use std::fmt::{Display, Formatter, Result as FmtResult};

pub struct NodePath {
    opaque: sys::types::OpaqueNodePath,
}

impl NodePath {
    fn from_opaque(opaque: sys::types::OpaqueNodePath) -> Self {
        Self { opaque }
    }
}

impl GodotFfi for NodePath {
    ffi_methods! { type sys::GDExtensionTypePtr = *mut Opaque; .. }
}

impl From<&GodotString> for NodePath {
    fn from(path: &GodotString) -> Self {
        unsafe {
            Self::from_sys_init(|self_ptr| {
                let ctor = sys::builtin_fn!(node_path_from_string);
                let args = [path.sys_const()];
                ctor(self_ptr, args.as_ptr());
            })
        }
    }
}

impl From<&NodePath> for GodotString {
    fn from(path: &NodePath) -> Self {
        unsafe {
            Self::from_sys_init(|self_ptr| {
                let ctor = sys::builtin_fn!(string_from_node_path);
                let args = [path.sys_const()];
                ctor(self_ptr, args.as_ptr());
            })
        }
    }
}

impl From<&str> for NodePath {
    fn from(path: &str) -> Self {
        Self::from(&GodotString::from(path))
    }
}

impl Display for NodePath {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        let string = GodotString::from(self);
        <GodotString as Display>::fmt(&string, f)
    }
}

impl_builtin_traits! {
    for NodePath {
        Clone => node_path_construct_copy;
        Drop => node_path_destroy;
    }
}
