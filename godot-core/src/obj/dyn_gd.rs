/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
use crate::classes;
use crate::obj::guards::DynGdRef;
use crate::obj::{bounds, Bounds, DynGdMut, Gd, Implements, Inherits};
use std::ops;

pub struct DynGd<T, D>
where
    T: Implements<D>,
    D: ?Sized,
{
    obj: Gd<T>,
    erased_downcast: fn(&Gd<classes::Object>) -> DynGdRef<D>,
    erased_downcast_mut: fn(&mut Gd<classes::Object>) -> DynGdMut<D>,
}

impl<T, D> DynGd<T, D>
where
    T: Implements<D> + Bounds<Declarer = bounds::DeclUser>,
    D: ?Sized,
{
    pub fn from_gd(gd_instance: Gd<T>) -> Self {
        let downcast: fn(&Gd<classes::Object>) -> DynGdRef<D> = |obj: &Gd<classes::Object>| {
            // SAFETY: the original instance is Gd<T> as per outer parameter, so downcasting to T is safe.
            let concrete: &Gd<T> = unsafe { obj.any_cast_ref() };

            // Use this syntax because rustc cannot infer type with `concrete.bind()`.
            let guard = Gd::bind(concrete);

            // For some reason, `= From::from` or `= Into::into` fails to compile with "one type is more general than the other".
            // Compilation also fails if we annotate the closure instead of the rhs type.
            let f: fn(&T) -> &D = Implements::dyn_upcast;
            DynGdRef::from_guard::<T>(guard, f)
        };

        let downcast_mut: fn(&mut Gd<classes::Object>) -> DynGdMut<D> =
            |obj: &mut Gd<classes::Object>| {
                // SAFETY: the original instance is Gd<T> as per outer parameter, so downcasting to T is safe.
                let concrete: &mut Gd<T> = unsafe { obj.any_cast_mut() };

                // Use this syntax because rustc cannot infer type with `concrete.bind_mut()`.
                let guard = Gd::bind_mut(concrete);

                // For some reason, `= From::from` or `= Into::into` fails to compile with "one type is more general than the other".
                // Compilation also fails if we annotate the closure instead of the rhs type.
                let f: fn(&mut T) -> &mut D = Implements::dyn_upcast_mut;
                DynGdMut::from_guard::<T>(guard, f)
            };

        Self::from_gd_downcasters(gd_instance, downcast, downcast_mut)
    }

    #[doc(hidden)]
    pub fn from_gd_downcasters(
        obj: Gd<T>,
        erased_downcast: fn(&Gd<classes::Object>) -> DynGdRef<D>,
        erased_downcast_mut: fn(&mut Gd<classes::Object>) -> DynGdMut<D>,
    ) -> Self {
        Self {
            obj,
            erased_downcast,
            erased_downcast_mut,
        }
    }

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

    /// See [`Gd::upcast()`].
    pub fn upcast<Base>(self) -> DynGd<Base, D>
    where
        Base: Implements<D>,
        T: Inherits<Base>,
    {
        // let erased_downcast: fn(&Gd<Object>) -> DynGdRef<T, D> = self.erased_downcast;
        // let erased_downcast_mut: fn(&mut Gd<Object>) -> DynGdMut<T, D> = self.erased_downcast_mut;

        // let erased_downcast: fn(&Gd<Object>) -> DynGdRef<Base, D> = unsafe {
        //     std::mem::transmute(erased_downcast)
        // };
        //
        // let erased_downcast_mut: fn(&mut Gd<Object>) -> DynGdMut<Base, D> = unsafe {
        //     std::mem::transmute(erased_downcast_mut)
        // };

        // DynGd {
        //     obj: self.obj.upcast::<Base>(),
        //     erased_downcast,
        //     erased_downcast_mut,
        // }

        todo!()
    }
}

impl<T, D> DynGd<T, D>
where
    T: Implements<D> + Bounds<Memory = bounds::MemManual>,
    D: ?Sized,
{
    pub fn free(self) {
        self.obj.free()
    }
}

impl<T, D> ops::Deref for DynGd<T, D>
where
    T: Implements<D>,
    D: ?Sized,
{
    type Target = Gd<T>;

    fn deref(&self) -> &Self::Target {
        &self.obj
    }
}

impl<T, D> ops::DerefMut for DynGd<T, D>
where
    T: Implements<D>,
    D: ?Sized,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.obj
    }
}

#[macro_export]
macro_rules! dyn_gd {
    ($Trait:ident; $gd_instance:expr) => {{
        use ::godot::obj::{DynGd, DynGdMut, DynGdRef, Gd};
        use ::godot::classes::Object;

        // Without the explicit type annotation, we get the weird error:
        // error[E0308]: mismatched types
        //    |
        // 59 |         DynGd::<Thing, dyn Health>::new(gd, downcast)
        //    |         ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ one type is more general than the other
        //
        // We can also not extract the closure into a separate variable, it needs to be inline in Box::new(...).
        //
        // Furthermore, we should be able to store this in a fn pointer without Box, however rustc
        // doesn't tolerate captures (type_), *even if they are ZSTs*. Thanks to not having decltype, we really
        // have to pay a runtime price just to pass in type information.
        let downcast: fn(&Gd<Object>) -> DynGdRef<_, dyn $Trait> =
            |obj: &Gd<Object>| {
                let concrete: &Gd<_> = unsafe { obj.any_cast_ref() };

                // Use this syntax because rustc cannot infer type with `concrete.bind()`.
                let guard = Gd::bind(concrete);

                DynGdRef::from_guard(guard, |t: &_| -> &dyn $Trait { t })
            };

        let downcast_mut: fn(&mut Gd<Object>) -> DynGdMut<dyn $Trait> =
            |obj: &mut Gd<Object>| {
                let concrete: &mut Gd<_> = unsafe { obj.any_cast_mut() };

                // Use this syntax because rustc cannot infer type with `concrete.bind_mut()`.
                let guard = Gd::bind_mut(concrete);

                DynGdMut::from_guard(guard, |t: &mut _| -> &mut dyn $Trait { t })
            };

        DynGd::<_, dyn $Trait>::from_gd_downcasters($gd_instance, downcast, downcast_mut)
    }}
}
