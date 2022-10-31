/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use godot_ffi as sys;
use sys::GodotFfi;

use crate::obj::{EngineClass, Gd, GodotClass};

mod private {
    pub trait Sealed {}
}
use private::Sealed;

pub trait AsArg: Sealed {
    #[doc(hidden)]
    fn as_arg_ptr(&self) -> sys::GDNativeTypePtr;
}

impl<T: GodotClass> Sealed for Gd<T> {}
impl<T: GodotClass> AsArg for Gd<T> {
    fn as_arg_ptr(&self) -> sys::GDNativeTypePtr {
        // Pass argument to engine: increment refcount
        <T::Mem as crate::obj::mem::Memory>::maybe_inc_ref(self);
        self.sys()
    }
}

// impl<T: EngineClass> Sealed for &T {}
// impl<T: EngineClass> AsArg for &T {
//     fn as_arg_ptr(&self) -> sys::GDNativeTypePtr {
//         // TODO what if this is dropped by the user? Same behavior as Gd<T>, no?
//         self.as_type_ptr()
//     }
// }
