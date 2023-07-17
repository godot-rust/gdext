/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::convert::Infallible;
use std::fmt::{Debug, Display, Formatter, Result as FmtResult};
use std::ops::{Deref, DerefMut};

use godot_ffi as sys;
use godot_ffi::VariantType;

use crate::builtin::meta::{ClassName, VariantMetadata};
use crate::builtin::{
    Callable, FromVariant, StringName, ToVariant, Variant, VariantConversionError,
};
use crate::obj::mem::Memory as _;
use crate::obj::{cap, dom, mem, GdMut, GdRef, GodotClass, Inherits, InstanceId, Share};
use crate::out;
use crate::property::{Export, ExportInfo, Property, TypeStringHint};

use super::RawGd;

/// Smart pointer to objects owned by the Godot engine.
///
/// This smart pointer can only hold _objects_ in the Godot sense: instances of Godot classes (`Node`, `RefCounted`, etc.)
/// or user-declared structs (`#[derive(GodotClass)]`). It does **not** hold built-in types (`Vector3`, `Color`, `i32`).
///
/// `Gd<T>` never holds null objects. If you need nullability, use `Option<Gd<T>>`.
///
/// This smart pointer behaves differently depending on `T`'s associated types, see [`GodotClass`] for their documentation.
/// In particular, the memory management strategy is fully dependent on `T`:
///
/// * Objects of type [`RefCounted`] or inherited from it are **reference-counted**. This means that every time a smart pointer is
///   shared using [`Share::share()`], the reference counter is incremented, and every time one is dropped, it is decremented.
///   This ensures that the last reference (either in Rust or Godot) will deallocate the object and call `T`'s destructor.
///
/// * Objects inheriting from [`Object`] which are not `RefCounted` (or inherited) are **manually-managed**.
///   Their destructor is not automatically called (unless they are part of the scene tree). Creating a `Gd<T>` means that
///   you are responsible of explicitly deallocating such objects using [`Gd::free()`].
///
/// * For `T=Object`, the memory strategy is determined **dynamically**. Due to polymorphism, a `Gd<T>` can point to either
///   reference-counted or manually-managed types at runtime. The behavior corresponds to one of the two previous points.
///   Note that if the dynamic type is also `Object`, the memory is manually-managed.
///
/// [`Object`]: crate::engine::Object
/// [`RefCounted`]: crate::engine::RefCounted
#[repr(transparent)]
pub struct Gd<T: GodotClass> {
    raw: RawGd<T>,
}

/// _The methods in this impl block are only available for user-declared `T`, that is,
/// structs with `#[derive(GodotClass)]` but not Godot classes like `Node` or `RefCounted`._ <br><br>
impl<T> Gd<T>
where
    T: GodotClass<Declarer = dom::UserDomain>,
{
    /// Moves a user-created object into this smart pointer, submitting ownership to the Godot engine.
    ///
    /// This is only useful for types `T` which do not store their base objects (if they have a base,
    /// you cannot construct them standalone).
    pub fn new(user_object: T) -> Self {
        Self::from_raw(RawGd::new(user_object)).expect("new object should not be null")
    }

    /// Creates a default-constructed instance of `T` inside a smart pointer.
    ///
    /// This is equivalent to the GDScript expression `T.new()`.
    pub fn new_default() -> Self
    where
        T: cap::GodotInit,
    {
        Self::from_raw(RawGd::new_default()).expect("new object should not be null")
    }

    /// Creates a `Gd<T>` using a function that constructs a `T` from a provided base.
    ///
    /// Imagine you have a type `T`, which has a `#[base]` field that you cannot default-initialize.
    /// The `init` function provides you with a `Base<T::Base>` object that you can use inside your `T`, which
    /// is then wrapped in a `Gd<T>`.
    ///
    /// Example:
    /// ```no_run
    /// # use godot::prelude::*;
    /// #[derive(GodotClass)]
    /// #[class(init, base=Node2D)]
    /// struct MyClass {
    ///     #[base]
    ///     my_base: Base<Node2D>,
    ///     other_field: i32,
    /// }
    ///
    /// let obj = Gd::<MyClass>::with_base(|my_base| {
    ///     // accepts the base and returns a constructed object containing it
    ///     MyClass { my_base, other_field: 732 }
    /// });
    /// ```
    pub fn with_base<F>(init: F) -> Self
    where
        F: FnOnce(crate::obj::Base<T::Base>) -> T,
    {
        Self::from_raw(RawGd::with_base(init)).expect("new object should not be null")
    }

    /// Hands out a guard for a shared borrow, through which the user instance can be read.
    ///
    /// The pattern is very similar to interior mutability with standard [`RefCell`][std::cell::RefCell].
    /// You can either have multiple `GdRef` shared guards, or a single `GdMut` exclusive guard to a Rust
    /// `GodotClass` instance, independently of how many `Gd` smart pointers point to it. There are runtime
    /// checks to ensure that Rust safety rules (e.g. no `&` and `&mut` coexistence) are upheld.
    ///
    /// # Panics
    /// * If another `Gd` smart pointer pointing to the same Rust instance has a live `GdMut` guard bound.
    /// * If there is an ongoing function call from GDScript to Rust, which currently holds a `&mut T`
    ///   reference to the user instance. This can happen through re-entrancy (Rust -> GDScript -> Rust call).
    // Note: possible names: write/read, hold/hold_mut, r/w, r/rw, ...
    pub fn bind(&self) -> GdRef<T> {
        self.raw.bind()
    }

    /// Hands out a guard for an exclusive borrow, through which the user instance can be read and written.
    ///
    /// The pattern is very similar to interior mutability with standard [`RefCell`][std::cell::RefCell].
    /// You can either have multiple `GdRef` shared guards, or a single `GdMut` exclusive guard to a Rust
    /// `GodotClass` instance, independently of how many `Gd` smart pointers point to it. There are runtime
    /// checks to ensure that Rust safety rules (e.g. no `&mut` aliasing) are upheld.
    ///
    /// # Panics
    /// * If another `Gd` smart pointer pointing to the same Rust instance has a live `GdRef` or `GdMut` guard bound.
    /// * If there is an ongoing function call from GDScript to Rust, which currently holds a `&T` or `&mut T`
    ///   reference to the user instance. This can happen through re-entrancy (Rust -> GDScript -> Rust call).
    pub fn bind_mut(&mut self) -> GdMut<T> {
        self.raw.bind_mut()
    }
}

/// _The methods in this impl block are available for any `T`._ <br><br>
impl<T: GodotClass> Gd<T> {
    pub(super) fn raw_as_ref(raw: &RawGd<T>) -> Option<&Self> {
        if !raw.is_null() {
            // SAFETY: `Gd` is `repr(transparent)` over `RawGd<T>`.
            Some(unsafe { std::mem::transmute::<&RawGd<T>, &Gd<T>>(raw) })
        } else {
            None
        }
    }

    pub(super) fn raw_as_mut(raw: &mut RawGd<T>) -> Option<&mut Self> {
        if !raw.is_null() {
            // SAFETY: `Gd` is `repr(transparent)` over `RawGd<T>`.
            Some(unsafe { std::mem::transmute::<&mut RawGd<T>, &mut Gd<T>>(raw) })
        } else {
            None
        }
    }

    pub fn from_raw(raw: RawGd<T>) -> Option<Self> {
        // SAFETY: We increment the refcount before calling `from_raw_no_inc`.
        unsafe { Self::from_raw_no_inc(raw.with_inc_refcount()) }
    }

    pub fn raw(&self) -> &RawGd<T> {
        &self.raw
    }

    /// # Safety
    /// the refcount of `raw` must already be incremented.
    pub unsafe fn from_raw_no_inc(raw: RawGd<T>) -> Option<Self> {
        // `Gd` cannot be null, however it can be a free object.
        if !raw.is_null() {
            Some(Self { raw })
        } else {
            None
        }
    }

    /// Looks up the given instance ID and returns the associated object, if possible.
    ///
    /// If no such instance ID is registered, or if the dynamic type of the object behind that instance ID
    /// is not compatible with `T`, then `None` is returned.
    pub fn try_from_instance_id(instance_id: InstanceId) -> Option<Self> {
        Self::from_raw(RawGd::try_from_instance_id(instance_id)?)
    }

    /// Remove the `raw` from self and return it.
    ///
    /// This does not decrement the refcount.
    fn take_raw(mut self) -> RawGd<T> {
        let raw = std::mem::take(&mut self.raw);
        // `self` is now null, this is an invalid state to drop a `Gd` in, so we must forget it.
        std::mem::forget(self);
        raw
    }

    /// ⚠️ Looks up the given instance ID and returns the associated object.
    ///
    /// # Panics
    /// If no such instance ID is registered, or if the dynamic type of the object behind that instance ID
    /// is not compatible with `T`.
    pub fn from_instance_id(instance_id: InstanceId) -> Self {
        let raw = RawGd::try_from_instance_id(instance_id).unwrap_or_else(|| {
            panic!(
                "Instance ID {} does not belong to an object of class '{}'",
                instance_id,
                T::CLASS_NAME
            )
        });

        Self::from_raw(raw).unwrap_or_else(|| {
            panic!(
                "Instance ID {} is either a freed or null object",
                instance_id,
            )
        })
    }

    /// Returns the instance ID of this object, or `None` if the object is dead.
    pub fn instance_id_or_none(&self) -> Option<InstanceId> {
        self.raw.instance_id_or_none()
    }

    /// ⚠️ Returns the instance ID of this object, or `None` if no instance ID is cached.
    ///
    /// This function does not check that the returned instance ID points to a valid instance!
    /// Unless performance is a problem, use [`instance_id_or_none`].
    pub fn instance_id_or_none_unchecked(&self) -> Option<InstanceId> {
        self.raw.instance_id_or_none_unchecked()
    }

    /// ⚠️ Returns the instance ID of this object (panics when dead).
    ///
    /// # Panics
    /// If this object is no longer alive (registered in Godot's object database).
    pub fn instance_id(&self) -> InstanceId {
        self.instance_id_or_none().unwrap_or_else(|| {
            panic!(
                "failed to call instance_id() on destroyed object; \
                use instance_id_or_none() or keep your objects alive"
            )
        })
    }

    /// Checks if this smart pointer points to a live object (read description!).
    ///
    /// Using this method is often indicative of bad design -- you should dispose of your pointers once an object is
    /// destroyed. However, this method exists because GDScript offers it and there may be **rare** use cases.
    ///
    /// Do not use this method to check if you can safely access an object. Accessing dead objects is generally safe
    /// and will panic in a defined manner. Encountering such panics is almost always a bug you should fix, and not a
    /// runtime condition to check against.
    pub fn is_instance_valid(&self) -> bool {
        // This call refreshes the instance ID, and recognizes dead objects.
        self.raw.is_instance_valid()
    }

    /// **Upcast:** convert into a smart pointer to a base class. Always succeeds.
    ///
    /// Moves out of this value. If you want to create _another_ smart pointer instance,
    /// use this idiom:
    /// ```no_run
    /// # use godot::prelude::*;
    /// #[derive(GodotClass)]
    /// #[class(init, base=Node2D)]
    /// struct MyClass {}
    ///
    /// let obj: Gd<MyClass> = Gd::new_default();
    /// let base = obj.share().upcast::<Node>();
    /// ```
    pub fn upcast<Base>(self) -> Gd<Base>
    where
        Base: GodotClass,
        T: Inherits<Base>,
    {
        self.owned_cast::<Base>()
            .expect("Upcast failed. This is a bug; please report it.")
    }

    /// **Downcast:** try to convert into a smart pointer to a derived class.
    ///
    /// If `T`'s dynamic type is not `Derived` or one of its subclasses, `None` is returned
    /// and the reference is dropped. Otherwise, `Some` is returned and the ownership is moved
    /// to the returned value.
    // TODO consider Result<Gd<Derived>, Self> so that user can still use original object (e.g. to free if manual)
    pub fn try_cast<Derived>(self) -> Option<Gd<Derived>>
    where
        Derived: GodotClass + Inherits<T>,
    {
        self.owned_cast().ok()
    }

    /// ⚠️ **Downcast:** convert into a smart pointer to a derived class. Panics on error.
    ///
    /// # Panics
    /// If the class' dynamic type is not `Derived` or one of its subclasses. Use [`Self::try_cast()`] if you want to check the result.
    pub fn cast<Derived>(self) -> Gd<Derived>
    where
        Derived: GodotClass + Inherits<T>,
    {
        self.owned_cast().unwrap_or_else(|from_obj| {
            panic!(
                "downcast from {from} to {to} failed; instance {from_obj:?}",
                from = T::CLASS_NAME,
                to = Derived::CLASS_NAME,
            )
        })
    }

    /// Returns `Ok(cast_obj)` on success, `Err(self)` on error
    fn owned_cast<U>(self) -> Result<Gd<U>, Self>
    where
        U: GodotClass,
    {
        let raw = self
            .take_raw()
            .owned_cast::<U>()
            .map_err(|raw| unsafe { Self::from_raw_no_inc(raw).unwrap() })?;
        Ok(unsafe { Gd::from_raw_no_inc(raw).expect("`Gd` should never be null") })
    }

    /// Returns a callable referencing a method from this object named `method_name`.
    pub fn callable<S: Into<StringName>>(&self, method_name: S) -> Callable {
        Callable::from_object_method(self.share(), method_name)
    }

    #[doc(hidden)]
    pub(crate) unsafe fn from_obj_sys(ptr: sys::GDExtensionObjectPtr) -> Self {
        Self::from_raw(RawGd::from_obj_sys(ptr)).unwrap()
    }
}

/// _The methods in this impl block are only available for objects `T` that are manually managed,
/// i.e. anything that is not `RefCounted` or inherited from it._ <br><br>
impl<T, M> Gd<T>
where
    T: GodotClass<Mem = M>,
    M: mem::PossiblyManual + mem::Memory,
{
    /// Destroy the manually-managed Godot object.
    ///
    /// Consumes this smart pointer and renders all other `Gd` smart pointers (as well as any GDScript references) to the same object
    /// immediately invalid. Using those `Gd` instances will lead to panics, but not undefined behavior.
    ///
    /// This operation is **safe** and effectively prevents double-free.
    ///
    /// Not calling `free()` on manually-managed instances causes memory leaks, unless their ownership is delegated, for
    /// example to the node tree in case of nodes.
    ///
    /// # Panics
    /// * When the referred-to object has already been destroyed.
    /// * When this is invoked on an upcast `Gd<Object>` that dynamically points to a reference-counted type (i.e. operation not supported).
    pub fn free(mut self) {
        // TODO disallow for singletons, either only at runtime or both at compile time (new memory policy) and runtime

        // Runtime check in case of T=Object, no-op otherwise
        let ref_counted = T::Mem::is_ref_counted(&self.raw);
        assert_ne!(
            ref_counted, Some(true),
            "called free() on Gd<Object> which points to a RefCounted dynamic type; free() only supported for manually managed types."
        );

        // If ref_counted returned None, that means the instance was destroyed
        assert!(
            ref_counted == Some(false) && self.is_instance_valid(),
            "called free() on already destroyed object"
        );

        let raw = std::mem::take(&mut self.raw);
        // SAFETY:
        // We've checked that `raw` isn't reference counted, and that it is a valid instance, so we know that
        // We arent calling `free` on an already freed object, and that it wont be used again.
        unsafe { raw.free() };

        // `Gd` is now null, this is not a valid state for the `Gd` to be dropped in, so we must forget it.
        std::mem::forget(self);
    }
}

impl<T> Deref for Gd<T>
where
    T: GodotClass<Declarer = dom::EngineDomain>,
{
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.raw.as_inner()
    }
}

impl<T> DerefMut for Gd<T>
where
    T: GodotClass<Declarer = dom::EngineDomain>,
{
    fn deref_mut(&mut self) -> &mut T {
        self.raw.as_inner_mut()
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct ObjectIsNullError;

impl std::fmt::Display for ObjectIsNullError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        f.write_str("object is null")
    }
}

impl std::error::Error for ObjectIsNullError {}

impl<T: GodotClass> sys::GodotFuncMarshal for Gd<T> {
    type Via = RawGd<T>;

    type FromViaError = ObjectIsNullError;

    type IntoViaError = Infallible;

    fn try_from_via(via: Self::Via) -> Result<Self, Self::FromViaError> {
        Self::from_raw(via).ok_or(ObjectIsNullError)
    }

    fn try_into_via(self) -> Result<Self::Via, Self::IntoViaError> {
        Ok(self.take_raw())
    }

    unsafe fn drop_via(via: &mut Self::Via) {
        // SAFETY:
        // `drop_via` can only be called on a `via` that was returned from `try_into_via`. We don't decrement
        // the refcount in `try_into_via` so the refcount is already incremented as it should be.
        Self::from_raw_no_inc(via.clone());
    }
}

impl<T: GodotClass> sys::GodotNullableFuncMarshal for Gd<T> {
    fn try_from_via_opt(via: Self::Via) -> Result<Option<Self>, Self::FromViaError> {
        Ok(Self::from_raw(via))
    }

    fn try_into_via_opt(opt: Option<Self>) -> Result<Self::Via, Self::IntoViaError> {
        let raw = opt.map(Self::take_raw).unwrap_or_default();
        Ok(raw)
    }
}

impl<T: GodotClass> Gd<T> {
    /// Runs `init_fn` on the address of a pointer (initialized to null). If that pointer is still null after the `init_fn` call,
    /// then `None` will be returned; otherwise `Gd::from_obj_sys(ptr)`.
    ///
    /// This method will **NOT** increment the reference-count of the object, as it assumes the input to come from a Godot API
    /// return value.
    ///
    /// # Safety
    /// `init_fn` must be a function that correctly handles a _type pointer_ pointing to an _object pointer_.
    #[doc(hidden)]
    pub unsafe fn from_sys_init_opt(init_fn: impl FnOnce(sys::GDExtensionTypePtr)) -> Option<Self> {
        // TODO(uninit) - should we use GDExtensionUninitializedTypePtr instead? Then update all the builtin codegen...
        let init_fn = |ptr| {
            init_fn(sys::AsUninit::force_init(ptr));
        };

        // Note: see _call_native_mb_ret_obj() in godot-cpp, which does things quite different (e.g. querying the instance binding).

        // Initialize pointer with given function, return Some(ptr) on success and None otherwise
        let object_ptr = super::raw_object_init(init_fn);

        // Do not increment ref-count; assumed to be return value from FFI.
        Gd::from_raw_no_inc(RawGd::from_obj_sys(object_ptr))
    }
}

/// Destructor with semantics depending on memory strategy.
///
/// * If this `Gd` smart pointer holds a reference-counted type, this will decrement the reference counter.
///   If this was the last remaining reference, dropping it will invoke `T`'s destructor.
///
/// * If the held object is manually-managed, **nothing happens**.
///   To destroy manually-managed `Gd` pointers, you need to call [`Self::free()`].
impl<T: GodotClass> Drop for Gd<T> {
    fn drop(&mut self) {
        // No-op for manually managed objects

        out!("Gd::drop   <{}>", std::any::type_name::<T>());
        let raw = std::mem::take(&mut self.raw);
        let is_last = T::Mem::maybe_dec_ref(&raw); // may drop
        if is_last {
            unsafe { raw.free() }
        }

        /*let st = self.storage();
        out!("    objd;  self={:?}, val={:?}", st as *mut _, st.lifecycle);
        //out!("    objd2; self={:?}, val={:?}", st as *mut _, st.lifecycle);

        // If destruction is triggered by Godot, Storage already knows about it, no need to notify it
        if !self.storage().destroyed_by_godot() {
            let is_last = T::Mem::maybe_dec_ref(&self); // may drop
            if is_last {
                //T::Declarer::destroy(self);
                unsafe {
                    interface_fn!(object_destroy)(self.obj_sys());
                }
            }
        }*/
    }
}

impl<T: GodotClass> Share for Gd<T> {
    fn share(&self) -> Self {
        out!("Gd::share");
        Self::from_raw(self.raw.clone()).expect("`Gd` should never be null")
    }
}

impl<T: GodotClass> TypeStringHint for Gd<T> {
    fn type_string() -> String {
        RawGd::<T>::type_string()
    }
}

impl<T: GodotClass> Property for Gd<T> {
    type Intermediate = Self;

    fn get_property(&self) -> Self {
        self.share()
    }

    fn set_property(&mut self, value: Self) {
        *self = value;
    }
}

impl<T: GodotClass> Export for Gd<T> {
    fn default_export_info() -> ExportInfo {
        RawGd::<T>::default_export_info()
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Trait impls

impl<T: GodotClass> FromVariant for Gd<T> {
    fn try_from_variant(variant: &Variant) -> Result<Self, VariantConversionError> {
        Self::from_raw(RawGd::try_from_variant(variant)?)
            .ok_or(VariantConversionError::VariantIsNull)
    }
}

impl<T: GodotClass> ToVariant for Gd<T> {
    fn to_variant(&self) -> Variant {
        // This already increments the refcount.
        self.raw.to_variant()
    }
}

impl<T: GodotClass> ToVariant for Option<Gd<T>> {
    fn to_variant(&self) -> Variant {
        match self {
            Some(gd) => gd.to_variant(),
            None => Variant::nil(),
        }
    }
}

impl<T: GodotClass> FromVariant for Option<Gd<T>> {
    fn try_from_variant(variant: &Variant) -> Result<Self, VariantConversionError> {
        if variant.is_nil() {
            Ok(None)
        } else {
            Gd::try_from_variant(variant).map(Some)
        }
    }
}

impl<T: GodotClass> PartialEq for Gd<T> {
    /// ⚠️ Returns whether two `Gd` pointers point to the same object.
    ///
    /// # Panics
    /// When `self` or `other` is dead.
    fn eq(&self, other: &Self) -> bool {
        // Panics when one is dead
        self.instance_id() == other.instance_id()
    }
}

impl<T: GodotClass> Eq for Gd<T> {}

impl<T: GodotClass> Display for Gd<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        self.raw.display_string(f)
    }
}

impl<T: GodotClass> Debug for Gd<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        self.raw.debug_string(f, "Gd")
    }
}

impl<T: GodotClass> VariantMetadata for Gd<T> {
    fn variant_type() -> VariantType {
        RawGd::<T>::variant_type()
    }

    fn class_name() -> ClassName {
        RawGd::<T>::class_name()
    }
}

// Gd unwinding across panics does not invalidate any invariants;
// its mutability is anyway present, in the Godot engine.
impl<T: GodotClass> std::panic::UnwindSafe for Gd<T> {}
impl<T: GodotClass> std::panic::RefUnwindSafe for Gd<T> {}
