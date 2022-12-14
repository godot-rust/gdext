/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::builtin::GodotString;
use godot_ffi as sys;
use sys::{ffi_methods, GodotFfi};

use std::fmt::{Debug, Display, Formatter, Result as FmtResult};
use std::hash::{Hash, Hasher};

#[repr(C)]
pub struct StringName {
    opaque: sys::types::OpaqueStringName,
}

impl StringName {
    fn from_opaque(opaque: sys::types::OpaqueStringName) -> Self {
        Self { opaque }
    }

    ffi_methods! {
        type sys::GDExtensionStringNamePtr = *mut Opaque;

        // Note: unlike from_sys, from_string_sys does not default-construct instance first. Typical usage in C++ is placement new.
        fn from_string_sys = from_sys;
        fn from_string_sys_init = from_sys_init;
        fn string_sys = sys;
        fn write_string_sys = write_sys;
    }
}

impl GodotFfi for StringName {
    ffi_methods! {
        type sys::GDExtensionTypePtr = *mut Opaque;
        fn from_sys;
        fn sys;
        fn write_sys;
    }

    unsafe fn from_sys_init(init_fn: impl FnOnce(sys::GDExtensionTypePtr)) -> Self {
        // Can't use uninitialized pointer -- StringName implementation in C++ expects that on assignment,
        // the target type is a valid string (possibly empty)

        let mut result = Self::default();
        init_fn(result.sys_mut());
        result
    }
}

impl_builtin_traits! {
    for StringName {
        Clone => string_name_construct_copy;
        Drop => string_name_destroy;
        Eq => string_name_operator_equal;
        Ord => string_name_operator_less;
    }
}

impl Default for StringName {
    fn default() -> Self {
        // Note: can't use from_sys_init(), as that calls the default constructor

        let mut uninit = std::mem::MaybeUninit::<StringName>::uninit();

        unsafe {
            let self_ptr = (*uninit.as_mut_ptr()).sys_mut();
            sys::builtin_call! {
                string_name_construct_default(self_ptr, std::ptr::null_mut())
            };

            uninit.assume_init()
        }
    }
}

impl Display for StringName {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        // TODO consider using GDScript built-in to_string()

        let s = GodotString::from(self);
        <GodotString as Display>::fmt(&s, f)
    }
}

impl Debug for StringName {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        // TODO consider using GDScript built-in to_string()

        let s = GodotString::from(self);
        <GodotString as Debug>::fmt(&s, f)
    }
}

impl Hash for StringName {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // TODO use Godot hash via codegen
        // C++: internal::gdn_interface->variant_get_ptr_builtin_method(GDEXTENSION_VARIANT_TYPE_STRING_NAME, "hash", 171192809);

        self.to_string().hash(state)
    }
}

impl From<&GodotString> for StringName {
    fn from(s: &GodotString) -> Self {
        unsafe {
            Self::from_sys_init(|self_ptr| {
                let ctor = sys::builtin_fn!(string_name_from_string);
                let args = [s.sys_const()];
                ctor(self_ptr, args.as_ptr());
            })
        }
    }
}

impl From<&str> for StringName {
    fn from(s: &str) -> Self {
        let intermediate = GodotString::from(s);
        Self::from(&intermediate)
    }
}

impl From<&StringName> for GodotString {
    fn from(s: &StringName) -> Self {
        unsafe {
            Self::from_sys_init(|self_ptr| {
                let ctor = sys::builtin_fn!(string_from_string_name);
                let args = [s.sys_const()];
                ctor(self_ptr, args.as_ptr());
            })
        }
    }
}
