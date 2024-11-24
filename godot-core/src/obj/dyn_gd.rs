/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::classes;
use crate::obj::guards::DynGdRef;
use crate::obj::{bounds, AsDyn, Bounds, DynGdMut, Gd, GodotClass, Inherits};
use std::ops;

pub struct DynGd<T, D>
where
    // T does _not_ require AsDyn<D> here. Otherwise, it's impossible to upcast (without implementing the relation for all base classes).
    T: GodotClass,
    D: ?Sized,
{
    obj: Gd<T>,

    // Potential optimization: these are fn pointers, not closures, that don't depend on `self`. Could be stored outside.
    erased_downcast: fn(&Gd<classes::Object>) -> DynGdRef<D>,
    erased_downcast_mut: fn(&mut Gd<classes::Object>) -> DynGdMut<D>,
}

impl<T, D> DynGd<T, D>
where
    T: AsDyn<D> + Bounds<Declarer = bounds::DeclUser>,
    D: ?Sized,
{
    pub fn from_gd(gd_instance: Gd<T>) -> Self {
        let downcast: fn(&Gd<classes::Object>) -> DynGdRef<D> = |obj: &Gd<classes::Object>| {
            // SAFETY: the original instance is Gd<T> as per outer parameter, so downcasting to T is safe.
            let concrete: &Gd<T> = unsafe { obj.any_cast_ref() };

            // Use this syntax because rustc cannot infer type with `concrete.bind()`.
            let guard = Gd::bind(concrete);

            DynGdRef::from_guard::<T>(guard)
        };

        let downcast_mut: fn(&mut Gd<classes::Object>) -> DynGdMut<D> =
            |obj: &mut Gd<classes::Object>| {
                // SAFETY: the original instance is Gd<T> as per outer parameter, so downcasting to T is safe.
                let concrete: &mut Gd<T> = unsafe { obj.any_cast_mut() };

                // Use this syntax because rustc cannot infer type with `concrete.bind_mut()`.
                let guard = Gd::bind_mut(concrete);

                DynGdMut::from_guard::<T>(guard)
            };

        Self {
            obj: gd_instance,
            erased_downcast: downcast,
            erased_downcast_mut: downcast_mut,
        }
    }
}

impl<T, D> DynGd<T, D>
where
    // Again, T deliberately does not require AsDyn<D> here. See above.
    T: GodotClass,
    D: ?Sized,
{
    pub fn dbind(&self) -> DynGdRef<D> {
        // SAFETY: Object is always a valid base.
        let object = unsafe { self.obj.any_cast_ref::<classes::Object>() };

        (self.erased_downcast)(object)
    }

    pub fn dbind_mut(&mut self) -> DynGdMut<D> {
        // SAFETY: Object is always a valid base.
        let object = unsafe { self.obj.any_cast_mut::<classes::Object>() };

        (self.erased_downcast_mut)(object)
    }

    // Certain methods "overridden" from deref'ed Gd here, so they're more idiomatic to use.
    // Those taking self by value, like free(), must be overridden.

    /// Upcast to a Godot base while retaining same trait.
    ///
    /// See [`Gd::upcast()`].
    pub fn upcast<Base>(self) -> DynGd<Base, D>
    where
        Base: GodotClass,
        T: Inherits<Base>,
    {
        DynGd {
            obj: self.obj.upcast::<Base>(),
            erased_downcast: self.erased_downcast,
            erased_downcast_mut: self.erased_downcast_mut,
        }
    }
}

impl<T, D> DynGd<T, D>
where
    T: GodotClass + Bounds<Memory = bounds::MemManual>,
    D: ?Sized,
{
    pub fn free(self) {
        self.obj.free()
    }
}

// Don't derive since that messes with bounds, and `.clone()` may silently fall back to deref'ed `Gd::clone()`.
impl<T, D> Clone for DynGd<T, D>
where
    T: GodotClass,
    D: ?Sized,
{
    fn clone(&self) -> Self {
        Self {
            obj: self.obj.clone(),
            erased_downcast: self.erased_downcast,
            erased_downcast_mut: self.erased_downcast_mut,
        }
    }
}

impl<T, D> ops::Deref for DynGd<T, D>
where
    T: GodotClass,
    D: ?Sized,
{
    type Target = Gd<T>;

    fn deref(&self) -> &Self::Target {
        &self.obj
    }
}

impl<T, D> ops::DerefMut for DynGd<T, D>
where
    T: GodotClass,
    D: ?Sized,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.obj
    }
}
