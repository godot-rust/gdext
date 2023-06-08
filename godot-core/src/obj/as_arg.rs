/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use godot_ffi as sys;
use sys::GodotFfi;

use crate::obj::{Gd, GodotClass};

mod private {
    pub trait Sealed {}
}
use private::Sealed;

pub trait AsArg: Sealed {
    #[doc(hidden)]
    fn as_arg_ptr(&self) -> sys::GDExtensionConstTypePtr;
}

impl<T: GodotClass> Sealed for Gd<T> {}
impl<T: GodotClass> AsArg for Gd<T> {
    fn as_arg_ptr(&self) -> sys::GDExtensionConstTypePtr {
        // We're passing a reference to the object to the callee. If the reference count needs to be
        // incremented then the callee will do so. We do not need to prematurely do so.
        //
        // In Rust terms, if `T` is refcounted then we are effectively passing a `&Arc<T>`, and the callee
        // would need to call `.clone()` if desired.

        // In 4.0, argument pointers are passed to godot as `T*`, except for in virtual method calls. We
        // can't perform virtual method calls currently, so they are always `T*`.
        //
        // In 4.1 argument pointers were standardized to always be `T**`.
        if cfg!(gdextension_api = "4.0") {
            self.sys_const()
        } else {
            std::ptr::addr_of!(self.opaque) as sys::GDExtensionConstTypePtr
        }
    }
}

// impl<T: EngineClass> Sealed for &T {}
// impl<T: EngineClass> AsArg for &T {
//     fn as_arg_ptr(&self) -> sys::GDExtensionTypePtr {
//         // TODO what if this is dropped by the user after the call? Same behavior as Gd<T>, no?
//         self.as_type_ptr()
//     }
// }
