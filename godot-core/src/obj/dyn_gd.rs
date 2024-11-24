/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::obj::guards::DynGdRef;
use crate::obj::{bounds, AsDyn, Bounds, DynGdMut, Gd, GodotClass, Inherits};
use std::ops;

/// Smart pointer integrating Rust traits via `dyn` dispatch.
///
/// `DynGd<T, D>` extends a Godot object [`Gd<T>`] with functionality for Rust's trait dynamic dispatch.  \
/// In this context, the type parameters have the following meaning:
/// - `T` is the Godot class.
/// - `D` is a trait object `dyn Trait`, where `T: Trait`.
///
/// To register the `T` -> `D` relation with godot-rust, `T` must implement [`AsDyn<D>`]. This can be automated with the
/// [`#[godot_dyn]`](../register/attr.godot_dyn.html) attribute macro.
///
/// # Public API
/// The API is very close to `Gd`. In fact, both `Deref` and `DerefMut` are implemented for `DynGd` -> `Gd`, so you can access all the
/// underlying `Gd` methods as well as Godot class APIs directly.
///
/// The main new parts are two methods [`dbind()`][Self::dbind] and [`dbind_mut()`][Self::dbind_mut]. These are very similar to `Gd`'s
/// [`bind()`][Gd::bind] and [`bind_mut()`][Gd::bind_mut], but return a reference guard to the trait object `D` instead of the Godot class `T`.
///
/// # Example
///
/// ```no_run
/// use godot::obj::{Gd, DynGd,NewGd};
/// use godot::register::{godot_dyn, GodotClass};
/// use godot::classes::RefCounted;
///
/// #[derive(GodotClass)]
/// #[class(init)]
/// struct Monster {
///    #[init(val = 100)]
///     hitpoints: u16,
/// }
///
/// trait Health {
///     fn is_alive(&self) -> bool;
///     fn deal_damage(&mut self, damage: u16);
/// }
///
/// // The #[godot_dyn] attribute macro registers the dynamic relation in godot-rust.
/// // Traits are implemented as usual.
/// #[godot_dyn]
/// impl Health for Monster {
///     fn is_alive(&self) -> bool {
///         self.hitpoints > 0
///     }
///
///     fn deal_damage(&mut self, damage: u16) {
///         self.hitpoints = self.hitpoints.saturating_sub(damage);
///     }
/// }
///
/// // Create a Gd<Monster> and convert it -> DynGd<Monster, dyn Health>.
/// let monster = Monster::new_gd();
/// let dyn_monster = monster.into_dyn::<dyn Health>();
///
/// // Now upcast it to its base class -> type is DynGd<RefCounted, dyn Health>.
/// let mut dyn_monster = dyn_monster.upcast::<RefCounted>();
///
/// // Due to RefCounted abstraction, you can no longer access concrete Monster properties.
/// // However, the trait Health is still accessible through dbind().
/// assert!(dyn_monster.dbind().is_alive());
///
/// // To mutate the object, call dbind_mut(). Rust borrow rules apply.
/// let mut guard = dyn_monster.dbind_mut();
/// guard.deal_damage(120);
/// assert!(!guard.is_alive());
/// ```
pub struct DynGd<T, D>
where
    // T does _not_ require AsDyn<D> here. Otherwise, it's impossible to upcast (without implementing the relation for all base classes).
    T: GodotClass,
    D: ?Sized,
{
    // Potential optimizations: use single Gd; use Rc/Arc instead of Box+clone; store a downcast fn from Gd<T>; ...
    obj: Gd<T>,
    erased_obj: Box<dyn ErasedGd<D>>,
}

impl<T, D> DynGd<T, D>
where
    T: AsDyn<D> + Bounds<Declarer = bounds::DeclUser>,
    D: ?Sized,
{
    pub fn from_gd(gd_instance: Gd<T>) -> Self {
        let erased_obj = Box::new(gd_instance.clone());

        Self {
            obj: gd_instance,
            erased_obj,
        }
    }
}

impl<T, D> DynGd<T, D>
where
    // Again, T deliberately does not require AsDyn<D> here. See above.
    T: GodotClass,
    D: ?Sized,
{
    /// Acquires a shared reference guard to the trait object `D`.
    ///
    /// The resulting guard implements `Deref<Target = D>`, allowing shared access to the trait's methods.
    ///
    /// See [`Gd::bind()`][Gd::bind] for borrow checking semantics and panics.
    pub fn dbind(&self) -> DynGdRef<D> {
        self.erased_obj.dbind()
    }

    /// Acquires an exclusive reference guard to the trait object `D`.
    ///
    /// The resulting guard implements `DerefMut<Target = D>`, allowing exclusive mutable access to the trait's methods.
    ///
    /// See [`Gd::bind_mut()`][Gd::bind_mut] for borrow checking semantics and panics.
    pub fn dbind_mut(&mut self) -> DynGdMut<D> {
        self.erased_obj.dbind_mut()
    }

    // Certain methods "overridden" from deref'ed Gd here, so they're more idiomatic to use.
    // Those taking self by value, like free(), must be overridden.

    /// Upcast to a Godot base, while retaining the `D` trait object.
    ///
    /// This is useful when you want to gather multiple objects under a common Godot base (e.g. `Node`), but still enable common functionality.
    /// The common functionality is still accessible through `D` even when upcasting.
    ///
    /// See also [`Gd::upcast()`].
    pub fn upcast<Base>(self) -> DynGd<Base, D>
    where
        Base: GodotClass,
        T: Inherits<Base>,
    {
        DynGd {
            obj: self.obj.upcast::<Base>(),
            erased_obj: self.erased_obj,
        }
    }

    /// Downgrades to a `Gd<T>` pointer, abandoning the `D` abstraction.
    #[must_use]
    pub fn into_gd(self) -> Gd<T> {
        self.obj
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
            erased_obj: self.erased_obj.clone_box(),
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

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Type erasure

trait ErasedGd<D: ?Sized> {
    fn dbind(&self) -> DynGdRef<D>;
    fn dbind_mut(&mut self) -> DynGdMut<D>;

    fn clone_box(&self) -> Box<dyn ErasedGd<D>>;
}

impl<T, D> ErasedGd<D> for Gd<T>
where
    T: AsDyn<D> + Bounds<Declarer = bounds::DeclUser>,
    D: ?Sized,
{
    fn dbind(&self) -> DynGdRef<D> {
        DynGdRef::from_guard::<T>(Gd::bind(self))
    }

    fn dbind_mut(&mut self) -> DynGdMut<D> {
        DynGdMut::from_guard::<T>(Gd::bind_mut(self))
    }

    fn clone_box(&self) -> Box<dyn ErasedGd<D>> {
        Box::new(Gd::clone(self))
    }
}
