/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::{fmt, ops};

use crate::builtin::Variant;
use crate::meta::error::ConvertError;
use crate::meta::{ClassId, FromGodot, GodotConvert, PropertyHintInfo, ToGodot};
use crate::obj::guards::DynGdRef;
use crate::obj::{bounds, AsDyn, Bounds, DynGdMut, Gd, GodotClass, Inherits, OnEditor};
use crate::registry::class::{get_dyn_property_hint_string, try_dynify_object};
use crate::registry::property::{object_export_element_type_string, Export, Var};
use crate::{meta, sys};

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
///
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
///     #[init(val = 100)]
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
/// When _receiving_ objects from Godot, the [`FromGodot`] trait is used to convert values to their Rust counterparts. `FromGodot` allows you to
/// use types in `#[func]` parameters or extract elements from arrays, among others. If you now receive a trait-enabled object back from Godot,
/// you can easily obtain it as `Gd<T>` -- but what if you need the original `DynGd<T, D>` back? If `T` is concrete and directly implements `D`,
/// then [`Gd::into_dyn()`] is of course possible. But in reality, you may have a polymorphic base class such as `RefCounted` or `Node` and
/// want to ensure that trait object `D` dispatches to the correct subclass, without manually checking every possible candidate.
///
/// To stay with the above example: let's say `Health` is implemented for two classes `Monster` and `Knight`. You now have a
/// `DynGd<RefCounted, dyn Health>`, which can represent either of the two classes. We pass this to Godot (e.g. as a `Variant`), and then back.
///
/// ```no_run
/// # use godot::prelude::*;
/// trait Health { /* ... */ }
///
/// #[derive(GodotClass)]
/// # #[class(init)]
/// struct Monster { /* ... */ }
/// #[godot_dyn]
/// impl Health for Monster { /* ... */ }
///
/// #[derive(GodotClass)]
/// # #[class(init)]
/// struct Knight { /* ... */ }
/// #[godot_dyn]
/// impl Health for Knight { /* ... */ }
///
/// // Let's construct a DynGd, and pass it to Godot as a Variant.
/// # let runtime_condition = true;
/// let variant = if runtime_condition {
///     // DynGd<Knight, dyn Health>
///     Knight::new_gd().into_dyn::<dyn Health>().to_variant()
/// } else {
///     // DynGd<Monster, dyn Health>
///     Monster::new_gd().into_dyn::<dyn Health>().to_variant()
/// };
///
/// // Now convert back into a DynGd -- but we don't know the concrete type.
/// // We can still represent it as DynGd<RefCounted, dyn Health>.
/// let dyn_gd: DynGd<RefCounted, dyn Health> = variant.to();
/// // Now work with the abstract object as usual.
/// ```
///
/// Any `Gd<T>` where `T` is an engine class can attempt conversion to `DynGd<T, D>` with [`Gd::try_dynify()`] as well.
///
/// ```no_run
/// # use godot::prelude::*;
/// # use godot::classes::Node2D;
/// # // ShapeCast2D is marked as experimental and thus not included in the doctests.
/// # // We use this mock to showcase some real-world usage.
/// # struct FakeShapeCastCollider2D {}
///
/// # impl FakeShapeCastCollider2D {
/// #     fn get_collider(&self, _idx: i32) -> Option<Gd<Node2D>> { Some(Node2D::new_alloc()) }
/// # }
///
/// trait Pushable { /* ... */ }
///
/// # let my_shapecast = FakeShapeCastCollider2D {};
/// # let idx = 1;
/// // We can try to convert `Gd<T>` into `DynGd<T, D>`.
/// let node: Option<DynGd<Node2D, dyn Pushable>> =
///     my_shapecast.get_collider(idx).and_then(
///         |obj| obj.try_dynify().ok()
///     );
///
/// // An object is returned after failed conversion, similarly to `Gd::try_cast()`.
/// # let some_node = Node::new_alloc();
/// match some_node.try_dynify::<dyn Pushable>() {
///     Ok(dyn_gd) => (),
///     Err(some_node) => godot_warn!("Failed to convert {some_node} into dyn Pushable!"),
/// }
/// ```
///
/// When converting from Godot back into `DynGd`, we say that the `dyn Health` trait object is _re-enriched_.
///
/// godot-rust achieves this thanks to the registration done by `#[godot_dyn]`: the library knows for which classes `Health` is implemented,
/// and it can query the dynamic type of the object. Based on that type, it can find the `impl Health` implementation matching the correct class.
/// Behind the scenes, everything is wired up correctly so that you can restore the original `DynGd` even after it has passed through Godot.
///
/// # Exporting
///
/// [Like `Gd<T>`](struct.Gd.html#exporting), using `#[export]` with `DynGd<T, D>` is possible only via [`OnEditor`] or [`Option`].
/// `DynGd<T, D>` can also be exported directly as an element of an array such as `Array<DynGd<T, D>>`.
///
/// When talking about "exporting", the following paragraphs assume that you wrap `DynGd` in one of those types.
///
/// In cases where `T: AsDyn<D>` (the trait is directly implemented on the user class, i.e. no upcasting), exporting `DynGd<T, D>` is
/// equivalent to exporting `Gd<T>` regarding Inspector UI.
///
/// ## Node-based classes
///
/// If `T` inherits `Node`, exporting `DynGd<T, D>` works identically to `Gd<T>`.
///
/// If you try to assign a class from the editor that does not implement trait `D`, Godot will report a conversion-failed error,
/// but it will only do so when accessing the given value.
///
/// ## Resource-based classes
///
/// If `T` inherits `Resource`, exporting `DynGd<T, D>>` will limit the available choices to known implementors of the trait `D`.
///
/// For example, let's say you have four Rust classes:
///
/// | Class        | Inherits   | Implements trait |
/// |--------------|------------|------------------|
/// | `Bullet`     | `Resource` | `Projectile`     |
/// | `Rocket`     | `Resource` | `Projectile`     |
/// | `BulletNode` | `Node`     | `Projectile`     |
/// | `Tower`      | `Resource` | (none)           |
///
/// Then, an exported `DynGd<Resource, dyn Projectile>` would be visible in Godot's Inspector UI with a drop-down field, i.e. users can assign
/// only objects of certain classes. **The available options for the drop-down are `Bullet` and `Rocket`.** The class `BulletNode` is not
/// available because it's not a `Resource`, and `Tower` is not because it doesn't implement the `Projectile` trait.
///
/// # Type inference
///
/// If a class implements more than one `AsDyn<D>` relation (usually via `#[godot_dyn]`), type inference will only work when the trait
/// used for `D` explicitly declares a `: 'static` bound.
/// Otherwise, if only one `impl AsDyn` is present for a given class, the type can always be inferred.
///
/// ```no_run
/// # use godot::prelude::*;
/// trait Health: 'static { /* ... */ }
///
/// // Exact equivalent to:
/// trait OtherHealth
/// where
///     Self: 'static
/// { /* ... */ }
///
/// trait NoInference { /* ... */ }
///
/// #[derive(GodotClass)]
/// # #[class(init)]
/// struct Monster { /* ... */ }
///
/// #[godot_dyn]
/// impl Health for Monster { /* ... */ }
///
/// #[godot_dyn]
/// impl NoInference for Monster { /* ... */ }
///
/// // Two example functions accepting trait object, to check type inference.
/// fn deal_damage(h: &mut dyn Health) { /* ... */ }
/// fn no_inference(i: &mut dyn NoInference) { /* ... */ }
///
/// // Type can be inferred since 'static bound is explicitly declared for Health trait.
/// let mut dyn_gd = Monster::new_gd().into_dyn();
/// deal_damage(&mut *dyn_gd.dyn_bind_mut());
///
/// // Otherwise type can't be properly inferred.
/// let mut dyn_gd = Monster::new_gd().into_dyn::<dyn NoInference>();
/// no_inference(&mut *dyn_gd.dyn_bind_mut());
/// ```
///
/// ```compile_fail
/// # use godot::prelude::*;
/// trait Health { /* ... */ }
///
/// trait OtherTrait { /* ... */ }
///
/// #[derive(GodotClass)]
/// # #[class(init)]
/// struct Monster { /* ... */ }
/// #[godot_dyn]
/// impl Health for Monster { /* ... */ }
/// #[godot_dyn]
/// impl OtherTrait for Monster { /* ... */ }
///
/// fn deal_damage(h: &mut dyn Health) { /* ... */ }
///
/// // Type can't be inferred.
/// // Would result in confusing compilation error
/// // since compiler would try to enforce 'static *lifetime* (&'static mut ...) on our reference.
/// let mut dyn_gd = Monster::new_gd().into_dyn();
/// deal_damage(&mut *dyn_gd.dyn_bind_mut());
/// ```
pub struct DynGd<T, D>
where
    // T does _not_ require AsDyn<D> here. Otherwise, it's impossible to upcast (without implementing the relation for all base classes).
    T: GodotClass,
    D: ?Sized + 'static,
{
    // Potential optimizations: use single Gd; use Rc/Arc instead of Box+clone; store a downcast fn from Gd<T>; ...
    obj: Gd<T>,
    erased_obj: Box<dyn ErasedGd<D>>,
}

impl<T, D> DynGd<T, D>
where
    T: AsDyn<D> + Bounds<Declarer = bounds::DeclUser>,
    D: ?Sized + 'static,
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
    D: ?Sized + 'static,
{
    /// Acquires a shared reference guard to the trait object `D`.
    ///
    /// The resulting guard implements `Deref<Target = D>`, allowing shared access to the trait's methods.
    ///
    /// See [`Gd::bind()`][Gd::bind] for borrow checking semantics and panics.
    pub fn dyn_bind(&self) -> DynGdRef<'_, D> {
        self.erased_obj.dyn_bind()
    }

    /// Acquires an exclusive reference guard to the trait object `D`.
    ///
    /// The resulting guard implements `DerefMut<Target = D>`, allowing exclusive mutable access to the trait's methods.
    ///
    /// See [`Gd::bind_mut()`][Gd::bind_mut] for borrow checking semantics and panics.
    pub fn dyn_bind_mut(&mut self) -> DynGdMut<'_, D> {
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
                from = T::class_id(),
                to = Derived::class_id(),
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

    /// Represents `null` when passing a dynamic object argument to Godot.
    ///
    /// See [`Gd::null_arg()`]
    pub fn null_arg() -> impl meta::AsArg<Option<DynGd<T, D>>> {
        meta::NullArg(std::marker::PhantomData)
    }
}

impl<T, D> DynGd<T, D>
where
    T: GodotClass + Bounds<Memory = bounds::MemManual>,
    D: ?Sized + 'static,
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
    D: ?Sized + 'static,
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
    D: ?Sized + 'static,
{
    type Target = Gd<T>;

    fn deref(&self) -> &Self::Target {
        &self.obj
    }
}

impl<T, D> ops::DerefMut for DynGd<T, D>
where
    T: GodotClass,
    D: ?Sized + 'static,
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

trait ErasedGd<D>
where
    D: ?Sized + 'static,
{
    fn dyn_bind(&self) -> DynGdRef<'_, D>;
    fn dyn_bind_mut(&mut self) -> DynGdMut<'_, D>;

    fn clone_box(&self) -> Box<dyn ErasedGd<D>>;
}

impl<T, D> ErasedGd<D> for Gd<T>
where
    T: AsDyn<D> + Bounds<Declarer = bounds::DeclUser>,
    D: ?Sized + 'static,
{
    fn dyn_bind(&self) -> DynGdRef<'_, D> {
        DynGdRef::from_guard::<T>(Gd::bind(self))
    }

    fn dyn_bind_mut(&mut self) -> DynGdMut<'_, D> {
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
    // Delegate to Gd<T> passing strategy.
    type Pass = <Gd<T> as ToGodot>::Pass;

    fn to_godot(&self) -> &Self::Via {
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
        match try_dynify_object(via) {
            Ok(dyn_gd) => Ok(dyn_gd),
            Err((from_godot_err, obj)) => Err(from_godot_err.into_error(obj)),
        }
    }
}

/*
// See `impl AsArg for Gd<T>` for why this isn't yet implemented.
impl<'r, T, TBase, D> meta::AsArg<DynGd<TBase, D>> for &'r DynGd<T, D>
where
    T: Inherits<TBase>,
    TBase: GodotClass,
    D: ?Sized + 'static,
{
    fn into_arg<'arg>(self) -> meta::CowArg<'arg, DynGd<TBase, D>>
    where
        'r: 'arg,
    {
        meta::CowArg::Owned(self.clone().upcast::<TBase>())
    }
}
*/

impl<T, D> meta::ArrayElement for DynGd<T, D>
where
    T: GodotClass,
    D: ?Sized + 'static,
{
    fn element_type_string() -> String {
        let hint_string = get_dyn_property_hint_string::<T, D>();
        object_export_element_type_string::<T>(hint_string)
    }
}

impl<T, D> meta::ArrayElement for Option<DynGd<T, D>>
where
    T: GodotClass,
    D: ?Sized + 'static,
{
    fn element_type_string() -> String {
        DynGd::<T, D>::element_type_string()
    }
}

impl<T, D> Var for DynGd<T, D>
where
    T: GodotClass,
    D: ?Sized + 'static,
{
    fn get_property(&self) -> Self::Via {
        self.obj.get_property()
    }

    fn set_property(&mut self, value: Self::Via) {
        // `set_property` can't be delegated to Gd<T>, since we have to set `erased_obj` as well.
        *self = <Self as FromGodot>::from_godot(value);
    }
}

/// See [`DynGd` Exporting](struct.DynGd.html#exporting) section.
impl<T, D> Export for Option<DynGd<T, D>>
where
    T: GodotClass + Bounds<Exportable = bounds::Yes>,
    D: ?Sized + 'static,
{
    fn export_hint() -> PropertyHintInfo {
        PropertyHintInfo::export_dyn_gd::<T, D>()
    }

    #[doc(hidden)]
    fn as_node_class() -> Option<ClassId> {
        PropertyHintInfo::object_as_node_class::<T>()
    }
}

impl<T, D> Default for OnEditor<DynGd<T, D>>
where
    T: GodotClass,
    D: ?Sized + 'static,
{
    fn default() -> Self {
        OnEditor::gd_invalid()
    }
}

impl<T, D> GodotConvert for OnEditor<DynGd<T, D>>
where
    T: GodotClass,
    D: ?Sized + 'static,
{
    type Via = Option<<DynGd<T, D> as GodotConvert>::Via>;
}

impl<T, D> Var for OnEditor<DynGd<T, D>>
where
    T: GodotClass,
    D: ?Sized + 'static,
{
    fn get_property(&self) -> Self::Via {
        Self::get_property_inner(self)
    }

    fn set_property(&mut self, value: Self::Via) {
        // `set_property` can't be delegated to Gd<T>, since we have to set `erased_obj` as well.
        Self::set_property_inner(self, value)
    }
}

/// See [`DynGd` Exporting](struct.DynGd.html#exporting) section.
impl<T, D> Export for OnEditor<DynGd<T, D>>
where
    Self: Var,
    T: GodotClass + Bounds<Exportable = bounds::Yes>,
    D: ?Sized + 'static,
{
    fn export_hint() -> PropertyHintInfo {
        PropertyHintInfo::export_dyn_gd::<T, D>()
    }

    #[doc(hidden)]
    fn as_node_class() -> Option<ClassId> {
        PropertyHintInfo::object_as_node_class::<T>()
    }
}
