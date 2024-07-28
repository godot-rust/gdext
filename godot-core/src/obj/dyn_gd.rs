/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

#![allow(unused, dead_code)] // FIXME

use crate::classes;
use crate::obj::{DynGdMut, Gd, GodotClass};

// struct Dyn<V> {
// }

pub struct DynGd<T, D>
where
    T: GodotClass,
    D: ?Sized,
{
    obj: Gd<T>,
    //rc: rc::Weak<B>
    // dyn_ptr: *mut B,
    erased_downcast: Box<dyn Fn(&mut Gd<classes::Object>) -> DynGdMut<T, D>>,
}

impl<T, D> DynGd<T, D>
where
    T: GodotClass,
    D: ?Sized,
{
    pub fn new(
        obj: Gd<T>,
        erased_downcast: impl Fn(&mut Gd<classes::Object>) -> DynGdMut<T, D>,
    ) -> Self {
        // Self {
        //     obj,
        //     erased_downcast: Box::new(erased_downcast),
        // }
        todo!()
    }

    pub fn dbind_mut(&mut self) -> DynGdMut<T, D> {
        // TODO performance+safety
        let object: &mut Gd<classes::Object> = unsafe { std::mem::transmute(&mut self.obj) };

        (self.erased_downcast)(object)
    }
}
