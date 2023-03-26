/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::convert::Infallible;
use std::fmt;
use std::str::FromStr;

use crate::builtin::GodotString;
use godot_ffi as sys;
use godot_ffi::{ffi_methods, GDExtensionTypePtr, GodotFfi};

pub struct NodePath {
    opaque: sys::types::OpaqueNodePath,
}

impl NodePath {
    fn from_opaque(opaque: sys::types::OpaqueNodePath) -> Self {
        Self { opaque }
    }
}

unsafe impl GodotFfi for NodePath {
    ffi_methods! { type sys::GDExtensionTypePtr = *mut Opaque;
        fn from_sys;
        fn sys;
        fn from_sys_init;
        // SAFETY:
        // Nothing special needs to be done beyond a `std::mem::swap` when returning a NodePath.
        fn move_return_ptr;
    }

    // SAFETY:
    // NodePaths are properly initialized through a `from_sys` call, but the ref-count should be
    // incremented as that is the callee's responsibility.
    //
    // Using `std::mem::forget(node_path.share())` increments the ref count.
    unsafe fn from_arg_ptr(ptr: sys::GDExtensionTypePtr, _call_type: sys::CallType) -> Self {
        let node_path = Self::from_sys(ptr);
        std::mem::forget(node_path.clone());
        node_path
    }

    unsafe fn from_sys_init_default(init_fn: impl FnOnce(GDExtensionTypePtr)) -> Self {
        let mut result = Self::default();
        init_fn(result.sys_mut());
        result
    }
}

impl From<&GodotString> for NodePath {
    fn from(path: &GodotString) -> Self {
        unsafe {
            Self::from_sys_init_default(|self_ptr| {
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
            Self::from_sys_init_default(|self_ptr| {
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

impl FromStr for NodePath {
    type Err = Infallible;

    fn from_str(path: &str) -> Result<Self, Self::Err> {
        Ok(Self::from(path))
    }
}

impl fmt::Display for NodePath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let string = GodotString::from(self);
        <GodotString as fmt::Display>::fmt(&string, f)
    }
}

/// Uses literal syntax from GDScript: `^"node_path"`
impl fmt::Debug for NodePath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let string = GodotString::from(self);
        write!(f, "^\"{string}\"")
    }
}

impl_builtin_traits! {
    for NodePath {
        Default => node_path_construct_default;
        Clone => node_path_construct_copy;
        Drop => node_path_destroy;
        Eq => node_path_operator_equal;
        // Ord => node_path_operator_less;
    }
}
