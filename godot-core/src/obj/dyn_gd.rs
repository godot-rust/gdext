/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::builtin::Variant;
use crate::meta::error::ConvertError;
use crate::meta::{FromGodot, GodotConvert, ToGodot};
use crate::obj::guards::DynGdRef;
use crate::obj::{bounds, AsDyn, Bounds, DynGdMut, Gd, GodotClass, Inherits};
use crate::registry::class::try_dynify_object;
use crate::{meta, sys};
use std::{fmt, ops};

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
/// # Construction and API
/// You can convert between `Gd` and `DynGd` using [`Gd::into_dyn()`] and [`DynGd::into_gd()`]. The former sometimes needs an explicit
/// `::<dyn Trait>` type argument, but can often be inferred.
///
/// The `DynGd` API is very close to `Gd`. In fact, both `Deref` and `DerefMut` are implemented for `DynGd` -> `Gd`, so you can access all the
/// underlying `Gd` methods as well as Godot class APIs directly.
///
/// The main new parts are two methods [`dyn_bind()`][Self::dyn_bind] and [`dyn_bind_mut()`][Self::dyn_bind_mut]. These are very similar to `Gd`'s
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
/// // However, the trait Health is still accessible through dyn_bind().
/// assert!(dyn_monster.dyn_bind().is_alive());
///
/// // To mutate the object, call dyn_bind_mut(). Rust borrow rules apply.
/// let mut guard = dyn_monster.dyn_bind_mut();
/// guard.deal_damage(120);
/// assert!(!guard.is_alive());
/// ```
///
/// # Polymorphic `dyn` re-enrichment
///
/// When passing `DynGd<T, D>` to Godot, you will lose the `D` part of the type inside the engine, because Godot doesn't know about Rust traits.
/// The trait methods won't be accessible through GDScript, either.
///
/// If you now receive the same object back from Godot, you can easily obtain it as `Gd<T>` -- but what if you need the original `DynGd<T, D>`?
/// If `T` is concrete (i.e. directly implements `D`), then [`Gd::into_dyn()`] is of course possible. But in reality, you may have a polymorphic
/// base class such as `RefCounted` and want to ensure that trait object `D` dispatches to the correct subclass, without manually checking every
/// possible candidate.
///
/// To stay with the above example: let's say `Health` is implemented for both `Monster` and `Knight` classes. You now receive a
/// `DynGd<RefCounted, dyn Health>`, which can represent either of the two classes. How can this work without trying to downcast to both?
///
/// godot-rust has a mechanism to re-enrich the `DynGd` with the correct trait object. Thanks to `#[godot_dyn]`, the library knows for which
/// classes `Health` is implemented, and it can query the dynamic type of the object. Based on that type, it can find the `impl Health`
/// implementation matching the correct class. Behind the scenes, everything is wired up correctly so that you can restore the original `DynGd`
/// even after it has passed through Godot.
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
    pub(crate) fn from_gd(gd_instance: Gd<T>) -> Self {
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
    pub fn dyn_bind(&self) -> DynGdRef<D> {
        self.erased_obj.dyn_bind()
    }

    /// Acquires an exclusive reference guard to the trait object `D`.
    ///
    /// The resulting guard implements `DerefMut<Target = D>`, allowing exclusive mutable access to the trait's methods.
    ///
    /// See [`Gd::bind_mut()`][Gd::bind_mut] for borrow checking semantics and panics.
    pub fn dyn_bind_mut(&mut self) -> DynGdMut<D> {
        self.erased_obj.dyn_bind_mut()
    }

    // Certain methods "overridden" from deref'ed Gd here, so they're more idiomatic to use.
    // Those taking self by value, like free(), must be overridden.

    /// **Upcast** to a Godot base, while retaining the `D` trait object.
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

    /// **Downcast** to a more specific Godot class, while retaining the `D` trait object.
    ///
    /// If `T`'s dynamic type is not `Derived` or one of its subclasses, `Err(self)` is returned, meaning you can reuse the original
    /// object for further casts.
    ///
    /// See also [`Gd::try_cast()`].
    pub fn try_cast<Derived>(self) -> Result<DynGd<Derived, D>, Self>
    where
        Derived: Inherits<T>,
    {
        match self.obj.try_cast::<Derived>() {
            Ok(obj) => Ok(DynGd {
                obj,
                erased_obj: self.erased_obj,
            }),
            Err(obj) => Err(DynGd {
                obj,
                erased_obj: self.erased_obj,
            }),
        }
    }

    /// ⚠️ **Downcast:** to a more specific Godot class, while retaining the `D` trait object.
    ///
    /// See also [`Gd::cast()`].
    ///
    /// # Panics
    /// If the class' dynamic type is not `Derived` or one of its subclasses. Use [`Self::try_cast()`] if you want to check the result.
    pub fn cast<Derived>(self) -> DynGd<Derived, D>
    where
        Derived: Inherits<T>,
    {
        self.try_cast().unwrap_or_else(|from_obj| {
            panic!(
                "downcast from {from} to {to} failed; instance {from_obj:?}",
                from = T::class_name(),
                to = Derived::class_name(),
            )
        })
    }

    /// Unsafe fast downcasts, no trait bounds.
    ///
    /// # Safety
    /// The caller must ensure that the dynamic type of the object is `Derived` or a subclass of `Derived`.
    // Not intended for public use. The lack of bounds simplifies godot-rust implementation, but adds another unsafety layer.
    #[deny(unsafe_op_in_unsafe_fn)]
    pub(crate) unsafe fn cast_unchecked<Derived>(self) -> DynGd<Derived, D>
    where
        Derived: GodotClass,
    {
        let cast_obj = self.obj.owned_cast::<Derived>();

        // SAFETY: ensured by safety invariant.
        let cast_obj = unsafe { cast_obj.unwrap_unchecked() };

        DynGd {
            obj: cast_obj,
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
    /// Destroy the manually-managed Godot object.
    ///
    /// See [`Gd::free()`] for semantics and panics.
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

impl<T, D> PartialEq for DynGd<T, D>
where
    T: GodotClass,
    D: ?Sized,
{
    fn eq(&self, other: &Self) -> bool {
        self.obj == other.obj
    }
}

impl<T, D> Eq for DynGd<T, D>
where
    T: GodotClass,
    D: ?Sized,
{
}

impl<T, D> std::hash::Hash for DynGd<T, D>
where
    T: GodotClass,
    D: ?Sized,
{
    /// ⚠️ Hashes this object based on its instance ID.
    ///
    /// # Panics
    /// When `self` is dead.
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.obj.hash(state);
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

impl<T, D> fmt::Debug for DynGd<T, D>
where
    T: GodotClass,
    D: ?Sized,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let trt = sys::short_type_name::<D>();
        crate::classes::debug_string_with_trait::<T>(self, f, "DynGd", &trt)
    }
}

impl<T, D> fmt::Display for DynGd<T, D>
where
    T: GodotClass,
    D: ?Sized,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        crate::classes::display_string(self, f)
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Type erasure

trait ErasedGd<D: ?Sized> {
    fn dyn_bind(&self) -> DynGdRef<D>;
    fn dyn_bind_mut(&mut self) -> DynGdMut<D>;

    fn clone_box(&self) -> Box<dyn ErasedGd<D>>;
}

impl<T, D> ErasedGd<D> for Gd<T>
where
    T: AsDyn<D> + Bounds<Declarer = bounds::DeclUser>,
    D: ?Sized,
{
    fn dyn_bind(&self) -> DynGdRef<D> {
        DynGdRef::from_guard::<T>(Gd::bind(self))
    }

    fn dyn_bind_mut(&mut self) -> DynGdMut<D> {
        DynGdMut::from_guard::<T>(Gd::bind_mut(self))
    }

    fn clone_box(&self) -> Box<dyn ErasedGd<D>> {
        Box::new(Gd::clone(self))
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Integration with Godot traits -- most are directly delegated to Gd<T>.

impl<T, D> GodotConvert for DynGd<T, D>
where
    T: GodotClass,
    D: ?Sized,
{
    type Via = Gd<T>;
}

impl<T, D> ToGodot for DynGd<T, D>
where
    T: GodotClass,
    D: ?Sized,
{
    type ToVia<'v>
        = <Gd<T> as ToGodot>::ToVia<'v>
    where
        D: 'v;

    fn to_godot(&self) -> Self::ToVia<'_> {
        self.obj.to_godot()
    }

    fn to_variant(&self) -> Variant {
        self.obj.to_variant()
    }
}

impl<T, D> FromGodot for DynGd<T, D>
where
    T: GodotClass,
    D: ?Sized + 'static,
{
    fn try_from_godot(via: Self::Via) -> Result<Self, ConvertError> {
        try_dynify_object(via)
    }
}

impl<'r, T, D> meta::AsArg<DynGd<T, D>> for &'r DynGd<T, D>
where
    T: GodotClass,
    D: ?Sized + 'static,
{
    fn into_arg<'cow>(self) -> meta::CowArg<'cow, DynGd<T, D>>
    where
        'r: 'cow, // Original reference must be valid for at least as long as the returned cow.
    {
        meta::CowArg::Borrowed(self)
    }
}

impl<T, D> meta::ParamType for DynGd<T, D>
where
    T: GodotClass,
    D: ?Sized + 'static,
{
    type Arg<'v> = meta::CowArg<'v, DynGd<T, D>>;

    fn owned_to_arg<'v>(self) -> Self::Arg<'v> {
        meta::CowArg::Owned(self)
    }

    fn arg_to_ref<'r>(arg: &'r Self::Arg<'_>) -> &'r Self {
        arg.cow_as_ref()
    }
}

impl<T, D> meta::ArrayElement for DynGd<T, D>
where
    T: GodotClass,
    D: ?Sized + 'static,
{
}
