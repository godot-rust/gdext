/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

#![allow(unused, dead_code)] // FIXME

use crate::engine;
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
    erased_downcast: Box<dyn Fn(&Gd<engine::Object>) -> DynGdMut<T, D>>,
}

impl<T, D> DynGd<T, D>
where
    T: GodotClass,
    D: ?Sized,
{
    pub fn new(
        obj: Gd<T>,
        erased_downcast: impl Fn(&Gd<engine::Object>) -> DynGdMut<T, D> + 'static,
    ) -> Self {
        Self {
            obj,
            erased_downcast: Box::new(erased_downcast),
        }
    }

    fn dbind_mut(&mut self) -> DynGdMut<T, D> {
        todo!()
    }
}

// fn make_dyn<T: GodotClass, D: ?Sized>(guard: DynGdMut<'_, T, D>) {
//
// }

#[macro_export]
macro_rules! dyn_gd {
    ($obj:expr) => {{
        ...
    }};

    ($Trait:ty; $obj:expr) => {{
        use $crate::obj::Gd;
        use $crate::engine::Object;
        let gd = Gd::from_object($obj);

        let downcast = |obj: Gd<Object>| {
            let concrete: Gd<_> = obj.cast();
            if false {
                std::mem::swap(&mut $obj, &mut concrete);
            }
            DynGdMut::from_guard(concrete.bind_mut())
        };

        make_dyn(downcast)
    }};
}
