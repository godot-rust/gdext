/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::rc;
use std::rc::Rc;
use crate::engine;
use crate::obj::{Gd, GdDynMut};

struct Dyn<V> {
}

pub struct DynGd<T, D>
where
    T: GodotClass,
    D: ?Sized,
{
    obj: Gd<T>,
    //rc: rc::Weak<B>
    // dyn_ptr: *mut B,
    erased_downcast: fn(Gd<engine::Object>) -> GdDynMut<T, D>,
}

impl<T, D> DynGd<T, D>
where
    T: GodotClass,
    D: ?Sized,
{
    fn dbind_mut(&mut self) -> GdDynMut<T, D> {

    }

}

fn make_fn<T: GodotClass, D: ?Sized>() -> fn(&mut T) -> &mut D {
    todo!()
}

macro_rules! downcast {
    () => {};
}

macro_rules! dyn_gd {
    ($obj:expr) => {{
        ...
    }};

    ($Trait:ty; $obj:expr) => {{
        use ::godot::obj::Gd;
        use ::godot::engine::Object;
        let gd = Gd::from_object($obj);

        fn downcast<T>(obj: Gd<Object>) -> &$Trait {
            let concrete: Gd<T> = obj.cast::<T>();
            concrete.bind()
        }
    }};
}
