/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::fmt::{Debug, Display, Formatter, Result as FmtResult};
use std::ops::{Deref, DerefMut};

use godot_ffi as sys;
use sys::{static_assert_eq_size_align, SysPtr as _};

use crate::builtin::{Callable, GString, NodePath, StringName, Variant};
use crate::meta::error::{ConvertError, FromFfiError};
use crate::meta::{
    ArrayElement, AsArg, ByRef, CallContext, ClassName, CowArg, FromGodot, GodotConvert, GodotType,
    ParamType, PropertyHintInfo, RefArg, ToGodot,
};
use crate::obj::{
    bounds, cap, Bounds, DynGd, GdDerefTarget, GdMut, GdRef, GodotClass, Inherits, InstanceId,
    OnEditor, RawGd, WithSignals,
};
use crate::private::{callbacks, PanicPayload};
use crate::registry::class::try_dynify_object;
use crate::registry::property::{object_export_element_type_string, Export, Var};
use crate::{classes, out};

/// Smart pointer to objects owned by the Godot engine.
///
/// See also [chapter about objects][book] in the book.
///
/// This smart pointer can only hold _objects_ in the Godot sense: instances of Godot classes (`Node`, `RefCounted`, etc.)
/// or user-declared structs (declared with `#[derive(GodotClass)]`). It does **not** hold built-in types (`Vector3`, `Color`, `i32`).
///
/// `Gd<T>` never holds null objects. If you need nullability, use `Option<Gd<T>>`. To pass null objects to engine APIs, you can
/// additionally use [`Gd::null_arg()`] as a shorthand.
///
/// # Memory management
///
/// This smart pointer behaves differently depending on `T`'s associated types, see [`GodotClass`] for their documentation.
/// In particular, the memory management strategy is fully dependent on `T`:
///
/// - **Reference-counted**<br>
///   Objects of type [`RefCounted`] or inherited from it are **reference-counted**. This means that every time a smart pointer is
///   shared using [`Clone::clone()`], the reference counter is incremented, and every time one is dropped, it is decremented.
///   This ensures that the last reference (either in Rust or Godot) will deallocate the object and call `T`'s destructor.<br><br>
///
/// - **Manual**<br>
///   Objects inheriting from [`Object`] which are not `RefCounted` (or inherited) are **manually-managed**.
///   Their destructor is not automatically called (unless they are part of the scene tree). Creating a `Gd<T>` means that
///   you are responsible for explicitly deallocating such objects using [`free()`][Self::free].<br><br>
///
/// - **Dynamic**<br>
///   For `T=Object`, the memory strategy is determined **dynamically**. Due to polymorphism, a `Gd<Object>` can point to either
///   reference-counted or manually-managed types at runtime. The behavior corresponds to one of the two previous points.
///   Note that if the dynamic type is also `Object`, the memory is manually-managed.
///
/// # Construction
///
/// To construct default instances of various `Gd<T>` types, there are extension methods on the type `T` itself:
///
/// - Manually managed: [`NewAlloc::new_alloc()`][crate::obj::NewAlloc::new_alloc]
/// - Reference-counted: [`NewGd::new_gd()`][crate::obj::NewGd::new_gd]
/// - Singletons: `T::singleton()` (inherent)
///
/// In addition, the smart pointer can be constructed in multiple ways:
///
/// * [`Gd::default()`] for reference-counted types that are constructible. For user types, this means they must expose an `init` function
///   or have a generated one. `Gd::<T>::default()` is equivalent to the shorter `T::new_gd()` and primarily useful for derives or generics.
/// * [`Gd::from_init_fn(function)`][Gd::from_init_fn] for Rust objects with `Base<T>` field, which are constructed inside the smart pointer.
///   This is a very handy function if you want to pass extra parameters to your object upon construction.
/// * [`Gd::from_object(rust_obj)`][Gd::from_object] for existing Rust objects without a `Base<T>` field that are moved _into_ the smart pointer.
/// * [`Gd::from_instance_id(id)`][Gd::from_instance_id] and [`Gd::try_from_instance_id(id)`][Gd::try_from_instance_id]
///   to obtain a pointer to an object which is already alive in the engine.
///
/// # Bind guards
///
/// The [`bind()`][Self::bind] and [`bind_mut()`][Self::bind_mut] methods allow you to obtain a shared or exclusive guard to the user instance.
/// These provide interior mutability similar to [`RefCell`][std::cell::RefCell], with the addition that `Gd` simultaneously handles reference
/// counting (for some types `T`).
///
/// Holding a bind guard will prevent other code paths from obtaining their own shared/mutable bind. As such, you should drop the guard
/// as soon as you don't need it anymore, by closing a `{ }` block or calling `std::mem::drop()`.
///
/// When you declare a `#[func]` method on your own class, and it accepts `&self` or `&mut self`, an implicit `bind()` or `bind_mut()` call
/// on the owning `Gd<T>` is performed. This is important to keep in mind, as you can get into situations that violate dynamic borrow rules; for
/// example if you are inside a `&mut self` method, make a call to GDScript and indirectly call another method on the same object (re-entrancy).
///
/// # Conversions
///
/// For type conversions, please read the [`godot::meta` module docs][crate::meta].
///
/// # Exporting
///
/// The [`Export`][crate::registry::property::Export] trait is not directly implemented for `Gd<T>`, because the editor expects object-based
/// properties to be nullable, while `Gd<T>` can't be null. Instead, `Export` is implemented for [`OnEditor<Gd<T>>`][crate::obj::OnEditor],
/// which validates that objects have been set by the editor. For the most flexible but least ergonomic option, you can also export
/// `Option<Gd<T>>` fields.
///
/// Objects can only be exported if `T: Inherits<Node>` or `T: Inherits<Resource>`, just like GDScript.
/// This means you cannot use `#[export]` with `OnEditor<Gd<RefCounted>>`, for example.
///
/// [book]: https://godot-rust.github.io/book/godot-api/objects.html
/// [`Object`]: classes::Object
/// [`RefCounted`]: classes::RefCounted
#[repr(C)] // must be layout compatible with engine classes
pub struct Gd<T: GodotClass> {
    // Note: `opaque` has the same layout as GDExtensionObjectPtr == Object* in C++, i.e. the bytes represent a pointer
    // To receive a GDExtensionTypePtr == GDExtensionObjectPtr* == Object**, we need to get the address of this
    // Hence separate sys() for GDExtensionTypePtr, and obj_sys() for GDExtensionObjectPtr.
    // The former is the standard FFI type, while the latter is used in object-specific GDExtension engines.
    // pub(crate) because accessed in obj::dom
    pub(crate) raw: RawGd<T>,
}

// Size equality check (should additionally be covered by mem::transmute())
static_assert_eq_size_align!(
    sys::GDExtensionObjectPtr,
    sys::types::OpaqueObject,
    "Godot FFI: pointer type `Object*` should have size advertised in JSON extension file"
);

/// _The methods in this impl block are only available for user-declared `T`, that is,
/// structs with `#[derive(GodotClass)]` but not Godot classes like `Node` or `RefCounted`._ <br><br>
impl<T> Gd<T>
where
    T: GodotClass + Bounds<Declarer = bounds::DeclUser>,
{
    /// Creates a `Gd<T>` using a function that constructs a `T` from a provided base.
    ///
    /// Imagine you have a type `T`, which has a base field that you cannot default-initialize.
    /// The `init` function provides you with a `Base<T::Base>` object that you can use inside your `T`, which
    /// is then wrapped in a `Gd<T>`.
    ///
    /// # Example
    /// ```no_run
    /// # use godot::prelude::*;
    /// #[derive(GodotClass)]
    /// #[class(init, base=Node2D)]
    /// struct MyClass {
    ///     my_base: Base<Node2D>,
    ///     other_field: i32,
    /// }
    ///
    /// let obj = Gd::from_init_fn(|my_base| {
    ///     // accepts the base and returns a constructed object containing it
    ///     MyClass { my_base, other_field: 732 }
    /// });
    /// ```
    ///
    /// # Panics
    /// Panics occurring in the `init` function are propagated to the caller.
    pub fn from_init_fn<F>(init: F) -> Self
    where
        F: FnOnce(crate::obj::Base<T::Base>) -> T,
    {
        let object_ptr = callbacks::create_custom(init) // or propagate panic.
            .unwrap_or_else(|payload| PanicPayload::repanic(payload));

        unsafe { Gd::from_obj_sys(object_ptr) }
    }

    /// Moves a user-created object into this smart pointer, submitting ownership to the Godot engine.
    ///
    /// This is only useful for types `T` which do not store their base objects (if they have a base,
    /// you cannot construct them standalone).
    pub fn from_object(user_object: T) -> Self {
        Self::from_init_fn(move |_base| user_object)
    }

    /// Hands out a guard for a shared borrow, through which the user instance can be read.
    ///
    /// The pattern is very similar to interior mutability with standard [`RefCell`][std::cell::RefCell].
    /// You can either have multiple `GdRef` shared guards, or a single `GdMut` exclusive guard to a Rust
    /// `GodotClass` instance, independently of how many `Gd` smart pointers point to it. There are runtime
    /// checks to ensure that Rust safety rules (e.g. no `&` and `&mut` coexistence) are upheld.
    ///
    /// Drop the guard as soon as you don't need it anymore. See also [Bind guards](#bind-guards).
    ///
    /// # Panics
    /// * If another `Gd` smart pointer pointing to the same Rust instance has a live `GdMut` guard bound.
    /// * If there is an ongoing function call from GDScript to Rust, which currently holds a `&mut T`
    ///   reference to the user instance. This can happen through re-entrancy (Rust -> GDScript -> Rust call).
    // Note: possible names: write/read, hold/hold_mut, r/w, r/rw, ...
    pub fn bind(&self) -> GdRef<'_, T> {
        self.raw.bind()
    }

    /// Hands out a guard for an exclusive borrow, through which the user instance can be read and written.
    ///
    /// The pattern is very similar to interior mutability with standard [`RefCell`][std::cell::RefCell].
    /// You can either have multiple `GdRef` shared guards, or a single `GdMut` exclusive guard to a Rust
    /// `GodotClass` instance, independently of how many `Gd` smart pointers point to it. There are runtime
    /// checks to ensure that Rust safety rules (e.g. no `&mut` aliasing) are upheld.
    ///
    /// Drop the guard as soon as you don't need it anymore. See also [Bind guards](#bind-guards).
    ///
    /// # Panics
    /// * If another `Gd` smart pointer pointing to the same Rust instance has a live `GdRef` or `GdMut` guard bound.
    /// * If there is an ongoing function call from GDScript to Rust, which currently holds a `&T` or `&mut T`
    ///   reference to the user instance. This can happen through re-entrancy (Rust -> GDScript -> Rust call).
    pub fn bind_mut(&mut self) -> GdMut<'_, T> {
        self.raw.bind_mut()
    }
}

/// _The methods in this impl block are available for any `T`._ <br><br>
impl<T: GodotClass> Gd<T> {
    /// Looks up the given instance ID and returns the associated object, if possible.
    ///
    /// If no such instance ID is registered, or if the dynamic type of the object behind that instance ID
    /// is not compatible with `T`, then `None` is returned.
    pub fn try_from_instance_id(instance_id: InstanceId) -> Result<Self, ConvertError> {
        let ptr = classes::object_ptr_from_id(instance_id);

        // SAFETY: assumes that the returned GDExtensionObjectPtr is convertible to Object* (i.e. C++ upcast doesn't modify the pointer)
        let untyped = unsafe { Gd::<classes::Object>::from_obj_sys_or_none(ptr)? };
        untyped
            .owned_cast::<T>()
            .map_err(|obj| FromFfiError::WrongObjectType.into_error(obj))
    }

    /// ⚠️ Looks up the given instance ID and returns the associated object.
    ///
    /// Corresponds to Godot's global function `instance_from_id()`.
    ///
    /// # Panics
    /// If no such instance ID is registered, or if the dynamic type of the object behind that instance ID
    /// is not compatible with `T`.
    #[doc(alias = "instance_from_id")]
    pub fn from_instance_id(instance_id: InstanceId) -> Self {
        Self::try_from_instance_id(instance_id).unwrap_or_else(|err| {
            panic!(
                "Instance ID {} does not belong to a valid object of class '{}': {}",
                instance_id,
                T::class_name(),
                err
            )
        })
    }

    /// Returns the instance ID of this object, or `None` if the object is dead or null.
    pub(crate) fn instance_id_or_none(&self) -> Option<InstanceId> {
        let known_id = self.instance_id_unchecked();

        // Refreshes the internal cached ID on every call, as we cannot be sure that the object has not been
        // destroyed since last time. The only reliable way to find out is to call is_instance_id_valid().
        if self.raw.is_instance_valid() {
            Some(known_id)
        } else {
            None
        }
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

    /// Returns the last known, possibly invalid instance ID of this object.
    ///
    /// This function does not check that the returned instance ID points to a valid instance!
    /// Unless performance is a problem, use [`instance_id()`][Self::instance_id] instead.
    ///
    /// This method is safe and never panics.
    pub fn instance_id_unchecked(&self) -> InstanceId {
        let instance_id = self.raw.instance_id_unchecked();

        // SAFETY: a `Gd` can only be created from a non-null `RawGd`, meaning `raw.instance_id_unchecked()` will
        // always return `Some`.
        unsafe { instance_id.unwrap_unchecked() }
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
        self.raw.is_instance_valid()
    }

    /// Returns the dynamic class name of the object as `StringName`.
    ///
    /// This method retrieves the class name of the object at runtime, which can be different from [`T::class_name()`][GodotClass::class_name]
    /// if derived classes are involved.
    ///
    /// Unlike [`Object::get_class()`][crate::classes::Object::get_class], this returns `StringName` instead of `GString` and needs no
    /// `Inherits<Object>` bound.
    pub(crate) fn dynamic_class_string(&self) -> StringName {
        unsafe {
            StringName::new_with_string_uninit(|ptr| {
                let success = sys::interface_fn!(object_get_class_name)(
                    self.obj_sys().as_const(),
                    sys::get_library(),
                    ptr,
                );

                let success = sys::conv::bool_from_sys(success);
                assert!(success, "failed to get class name for object {self:?}");
            })
        }
    }

    /// Returns the reference count, if the dynamic object inherits `RefCounted`; and `None` otherwise.
    ///
    /// Returns `Err(())` if obtaining reference count failed, due to being called during init/drop.
    pub(crate) fn maybe_refcount(&self) -> Option<Result<usize, ()>> {
        // May become infallible if implemented via call() on Object, if ref-count bit of instance ID is set.
        // This would likely be more efficient, too.

        // Fast check if ref-counted without downcast.
        if !self.instance_id().is_ref_counted() {
            return None;
        }

        // Optimization: call `get_reference_count()` directly. Might also increase reliability and obviate the need for Result.

        let rc = self
            .raw
            .try_with_ref_counted(|refc| refc.get_reference_count());

        Some(rc.map(|i| i as usize))
    }

    #[cfg(feature = "trace")] // itest only.
    #[doc(hidden)]
    pub fn test_refcount(&self) -> Option<usize> {
        self.maybe_refcount()
            .transpose()
            .expect("failed to obtain refcount")
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
    /// let obj: Gd<MyClass> = MyClass::new_alloc();
    /// let base = obj.clone().upcast::<Node>();
    /// ```
    pub fn upcast<Base>(self) -> Gd<Base>
    where
        Base: GodotClass,
        T: Inherits<Base>,
    {
        self.owned_cast()
            .expect("Upcast failed. This is a bug; please report it.")
    }

    /// Equivalent to [`upcast::<Object>()`][Self::upcast], but without bounds.
    // Not yet public because it might need _mut/_ref overloads, and 6 upcast methods are a bit much...
    #[doc(hidden)] // no public API, but used by #[signal].
    pub fn upcast_object(self) -> Gd<classes::Object> {
        self.owned_cast()
            .expect("Upcast to Object failed. This is a bug; please report it.")
    }

    // /// Equivalent to [`upcast_mut::<Object>()`][Self::upcast_mut], but without bounds.
    // pub(crate) fn upcast_object_ref(&self) -> &classes::Object {
    //     self.raw.as_object_ref()
    // }

    /// Equivalent to [`upcast_mut::<Object>()`][Self::upcast_mut], but without bounds.
    pub(crate) fn upcast_object_mut(&mut self) -> &mut classes::Object {
        self.raw.as_object_mut()
    }

    // pub(crate) fn upcast_object_mut_from_ref(&self) -> &mut classes::Object {
    //     self.raw.as_object_mut()
    // }

    /// **Upcast shared-ref:** access this object as a shared reference to a base class.
    ///
    /// This is semantically equivalent to multiple applications of [`Self::deref()`]. Not really useful on its own, but combined with
    /// generic programming:
    /// ```no_run
    /// # use godot::prelude::*;
    /// fn print_node_name<T>(node: &Gd<T>)
    /// where
    ///     T: Inherits<Node>,
    /// {
    ///     println!("Node name: {}", node.upcast_ref().get_name());
    /// }
    /// ```
    ///
    /// Note that this cannot be used to get a reference to Rust classes, for that you should use [`Gd::bind()`]. For instance this
    /// will fail:
    /// ```compile_fail
    /// # use godot::prelude::*;
    /// #[derive(GodotClass)]
    /// #[class(init, base = Node)]
    /// struct SomeClass {}
    ///
    /// #[godot_api]
    /// impl INode for SomeClass {
    ///     fn ready(&mut self) {
    ///         let other = SomeClass::new_alloc();
    ///         let _ = other.upcast_ref::<SomeClass>();
    ///     }
    /// }
    /// ```
    pub fn upcast_ref<Base>(&self) -> &Base
    where
        Base: GodotClass + Bounds<Declarer = bounds::DeclEngine>,
        T: Inherits<Base>,
    {
        // SAFETY: `Base` is guaranteed to be an engine base class of `T` because of the generic bounds.
        unsafe { self.raw.as_upcast_ref::<Base>() }
    }

    /// **Upcast exclusive-ref:** access this object as an exclusive reference to a base class.
    ///
    /// This is semantically equivalent to multiple applications of [`Self::deref_mut()`]. Not really useful on its own, but combined with
    /// generic programming:
    /// ```no_run
    /// # use godot::prelude::*;
    /// fn set_node_name<T>(node: &mut Gd<T>, name: &str)
    /// where
    ///     T: Inherits<Node>,
    /// {
    ///     node.upcast_mut().set_name(name);
    /// }
    /// ```
    ///
    /// Note that this cannot be used to get a mutable reference to Rust classes, for that you should use [`Gd::bind_mut()`]. For instance this
    /// will fail:
    /// ```compile_fail
    /// # use godot::prelude::*;
    /// #[derive(GodotClass)]
    /// #[class(init, base = Node)]
    /// struct SomeClass {}
    ///
    /// #[godot_api]
    /// impl INode for SomeClass {
    ///     fn ready(&mut self) {
    ///         let mut other = SomeClass::new_alloc();
    ///         let _ = other.upcast_mut::<SomeClass>();
    ///     }
    /// }
    /// ```
    pub fn upcast_mut<Base>(&mut self) -> &mut Base
    where
        Base: GodotClass + Bounds<Declarer = bounds::DeclEngine>,
        T: Inherits<Base>,
    {
        // SAFETY: `Base` is guaranteed to be an engine base class of `T` because of the generic bounds.
        unsafe { self.raw.as_upcast_mut::<Base>() }
    }

    /// **Downcast:** try to convert into a smart pointer to a derived class.
    ///
    /// If `T`'s dynamic type is not `Derived` or one of its subclasses, `Err(self)` is returned, meaning you can reuse the original
    /// object for further casts.
    pub fn try_cast<Derived>(self) -> Result<Gd<Derived>, Self>
    where
        Derived: Inherits<T>,
    {
        // Separate method due to more restrictive bounds.
        self.owned_cast()
    }

    /// ⚠️ **Downcast:** convert into a smart pointer to a derived class. Panics on error.
    ///
    /// # Panics
    /// If the class' dynamic type is not `Derived` or one of its subclasses. Use [`Self::try_cast()`] if you want to check the result.
    pub fn cast<Derived>(self) -> Gd<Derived>
    where
        Derived: Inherits<T>,
    {
        self.owned_cast().unwrap_or_else(|from_obj| {
            panic!(
                "downcast from {from} to {to} failed; instance {from_obj:?}",
                from = T::class_name(),
                to = Derived::class_name(),
            )
        })
    }

    /// Returns `Ok(cast_obj)` on success, `Err(self)` on error.
    // Visibility: used by DynGd.
    pub(crate) fn owned_cast<U>(self) -> Result<Gd<U>, Self>
    where
        U: GodotClass,
    {
        self.raw
            .owned_cast()
            .map(Gd::from_ffi)
            .map_err(Self::from_ffi)
    }

    /// Create default instance for all types that have `GodotDefault`.
    ///
    /// Deliberately more loose than `Gd::default()`, does not require ref-counted memory strategy for user types.
    pub(crate) fn default_instance() -> Self
    where
        T: cap::GodotDefault,
    {
        unsafe {
            // Default value (and compat one) for `p_notify_postinitialize` is true in Godot.
            #[cfg(since_api = "4.4")]
            let object_ptr = callbacks::create::<T>(std::ptr::null_mut(), sys::conv::SYS_TRUE);
            #[cfg(before_api = "4.4")]
            let object_ptr = callbacks::create::<T>(std::ptr::null_mut());

            Gd::from_obj_sys(object_ptr)
        }
    }

    /// Upgrades to a `DynGd<T, D>` pointer, enabling the `D` abstraction.
    ///
    /// The `D` parameter can typically be inferred when there is a single `AsDyn<...>` implementation for `T`.  \
    /// Otherwise, use it as `gd.into_dyn::<dyn MyTrait>()`.
    #[must_use]
    pub fn into_dyn<D>(self) -> DynGd<T, D>
    where
        T: crate::obj::AsDyn<D> + Bounds<Declarer = bounds::DeclUser>,
        D: ?Sized + 'static,
    {
        DynGd::<T, D>::from_gd(self)
    }

    /// Tries to upgrade to a `DynGd<T, D>` pointer, enabling the `D` abstraction.
    ///
    /// If `T`'s dynamic class doesn't implement `AsDyn<D>`, `Err(self)` is returned, meaning you can reuse the original
    /// object for further casts.
    pub fn try_dynify<D>(self) -> Result<DynGd<T, D>, Self>
    where
        T: GodotClass + Bounds<Declarer = bounds::DeclEngine>,
        D: ?Sized + 'static,
    {
        match try_dynify_object(self) {
            Ok(dyn_gd) => Ok(dyn_gd),
            Err((_convert_err, obj)) => Err(obj),
        }
    }

    /// Returns a callable referencing a method from this object named `method_name`.
    ///
    /// This is shorter syntax for [`Callable::from_object_method(self, method_name)`][Callable::from_object_method].
    pub fn callable(&self, method_name: impl AsArg<StringName>) -> Callable {
        Callable::from_object_method(self, method_name)
    }

    /// Creates a new callable linked to the given object from **single-threaded** Rust function or closure.
    /// This is shorter syntax for [`Callable::from_linked_fn()`].
    ///
    /// `name` is used for the string representation of the closure, which helps with debugging.
    ///
    /// Such a callable will be automatically invalidated by Godot when a linked Object is freed.
    /// If you need a Callable which can live indefinitely use [`Callable::from_local_fn()`].
    #[cfg(since_api = "4.2")]
    pub fn linked_callable<F>(&self, method_name: impl AsArg<GString>, rust_function: F) -> Callable
    where
        F: 'static + FnMut(&[&Variant]) -> Result<Variant, ()>,
    {
        Callable::from_linked_fn(method_name, self, rust_function)
    }

    pub(crate) unsafe fn from_obj_sys_or_none(
        ptr: sys::GDExtensionObjectPtr,
    ) -> Result<Self, ConvertError> {
        // Used to have a flag to select RawGd::from_obj_sys_weak(ptr) for Base::to_init_gd(), but solved differently in the end.
        let obj = RawGd::from_obj_sys(ptr);

        Self::try_from_ffi(obj)
    }

    /// Initializes this `Gd<T>` from the object pointer as a **strong ref**, meaning
    /// it initializes/increments the reference counter and keeps the object alive.
    ///
    /// This is the default for most initializations from FFI. In cases where reference counter
    /// should explicitly **not** be updated, [`Self::from_obj_sys_weak`] is available.
    pub(crate) unsafe fn from_obj_sys(ptr: sys::GDExtensionObjectPtr) -> Self {
        debug_assert!(
            !ptr.is_null(),
            "Gd::from_obj_sys() called with null pointer"
        );

        Self::from_obj_sys_or_none(ptr).unwrap()
    }

    pub(crate) unsafe fn from_obj_sys_weak_or_none(
        ptr: sys::GDExtensionObjectPtr,
    ) -> Result<Self, ConvertError> {
        Self::try_from_ffi(RawGd::from_obj_sys_weak(ptr))
    }

    pub(crate) unsafe fn from_obj_sys_weak(ptr: sys::GDExtensionObjectPtr) -> Self {
        Self::from_obj_sys_weak_or_none(ptr).unwrap()
    }

    #[doc(hidden)]
    pub fn obj_sys(&self) -> sys::GDExtensionObjectPtr {
        self.raw.obj_sys()
    }

    #[doc(hidden)]
    pub fn script_sys(&self) -> sys::GDExtensionScriptLanguagePtr
    where
        T: Inherits<classes::ScriptLanguage>,
    {
        self.raw.script_sys()
    }

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
            init_fn(sys::SysPtr::force_init(ptr));
        };

        // Note: see _call_native_mb_ret_obj() in godot-cpp, which does things quite different (e.g. querying the instance binding).

        // Initialize pointer with given function, return Some(ptr) on success and None otherwise
        let object_ptr = super::raw_object_init(init_fn);

        // Do not increment ref-count; assumed to be return value from FFI.
        sys::ptr_then(object_ptr, |ptr| Gd::from_obj_sys_weak(ptr))
    }
}

/// _The methods in this impl block are only available for objects `T` that are manually managed,
/// i.e. anything that is not `RefCounted` or inherited from it._ <br><br>
impl<T> Gd<T>
where
    T: GodotClass + Bounds<Memory = bounds::MemManual>,
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
    /// - When the referred-to object has already been destroyed.
    /// - When this is invoked on an upcast `Gd<Object>` that dynamically points to a reference-counted type (i.e. operation not supported).
    /// - When the object is bound by an ongoing `bind()` or `bind_mut()` call (through a separate `Gd` pointer).
    pub fn free(self) {
        // Note: this method is NOT invoked when the free() call happens dynamically (e.g. through GDScript or reflection).
        // As such, do not use it for operations and validations to perform upon destruction.

        // free() is likely to be invoked in destructors during panic unwind. In this case, we cannot panic again.
        // Instead, we print an error and exit free() immediately. The closure is supposed to be used in a unit return statement.
        let is_panic_unwind = std::thread::panicking();
        let error_or_panic = |msg: String| {
            if is_panic_unwind {
                if crate::private::has_error_print_level(1) {
                    crate::godot_error!(
                        "Encountered 2nd panic in free() during panic unwind; will skip destruction:\n{msg}"
                    );
                }
            } else {
                panic!("{}", msg);
            }
        };

        // TODO disallow for singletons, either only at runtime or both at compile time (new memory policy) and runtime
        use bounds::Declarer;

        // Runtime check in case of T=Object, no-op otherwise
        let ref_counted =
            <<T as Bounds>::DynMemory as bounds::DynMemory>::is_ref_counted(&self.raw);
        if ref_counted == Some(true) {
            return error_or_panic(format!(
                "Called free() on Gd<Object> which points to a RefCounted dynamic type; free() only supported for manually managed types\n\
                Object: {self:?}"
            ));
        }

        // If ref_counted returned None, that means the instance was destroyed
        if ref_counted != Some(false) || !self.is_instance_valid() {
            return error_or_panic("called free() on already destroyed object".to_string());
        }

        // If the object is still alive, make sure the dynamic type matches. Necessary because subsequent checks may rely on the
        // static type information to be correct. This is a no-op in Release mode.
        // Skip check during panic unwind; would need to rewrite whole thing to use Result instead. Having BOTH panic-in-panic and bad type is
        // a very unlikely corner case.
        if !is_panic_unwind {
            self.raw.check_dynamic_type(&CallContext::gd::<T>("free"));
        }

        // SAFETY: object must be alive, which was just checked above. No multithreading here.
        // Also checked in the C free_instance_func callback, however error message can be more precise here, and we don't need to instruct
        // the engine about object destruction. Both paths are tested.
        let bound = unsafe { T::Declarer::is_currently_bound(&self.raw) };
        if bound {
            return error_or_panic(
                "called free() while a bind() or bind_mut() call is active".to_string(),
            );
        }

        // SAFETY: object alive as checked.
        // This destroys the Storage instance, no need to run destructor again.
        unsafe {
            sys::interface_fn!(object_destroy)(self.raw.obj_sys());
        }

        // TODO: this might leak associated data in Gd<T>, e.g. ClassName.
        std::mem::forget(self);
    }
}

/// _The methods in this impl block are only available for objects `T` that are reference-counted,
/// i.e. anything that inherits `RefCounted`._ <br><br>
impl<T> Gd<T>
where
    T: GodotClass + Bounds<Memory = bounds::MemRefCounted>,
{
    /// Makes sure that `self` does not share references with other `Gd` instances.
    ///
    /// Succeeds if the reference count is 1.
    /// Otherwise, returns the shared object and its reference count.
    ///
    /// ## Example
    ///
    /// ```no_run
    /// use godot::prelude::*;
    ///
    /// let obj = RefCounted::new_gd();
    /// match obj.try_to_unique() {
    ///    Ok(unique_obj) => {
    ///        // No other Gd<T> shares a reference with `unique_obj`.
    ///    },
    ///    Err((shared_obj, ref_count)) => {
    ///        // `shared_obj` is the original object `obj`.
    ///        // `ref_count` is the total number of references (including one held by `shared_obj`).
    ///    }
    /// }
    /// ```
    pub fn try_to_unique(self) -> Result<Self, (Self, usize)> {
        use crate::obj::bounds::DynMemory as _;

        match <T as Bounds>::DynMemory::get_ref_count(&self.raw) {
            Some(1) => Ok(self),
            Some(ref_count) => Err((self, ref_count)),
            None => unreachable!(),
        }
    }
}

impl<T> Gd<T>
where
    T: GodotClass + Bounds<Declarer = bounds::DeclEngine>,
{
    /// Represents `null` when passing an object argument to Godot.
    ///
    /// This expression is only intended for function argument lists. It can be used whenever a Godot signature accepts
    /// [`AsObjectArg<T>`][crate::meta::AsObjectArg]. `Gd::null_arg()` as an argument is equivalent to `Option::<Gd<T>>::None`, but less wordy.
    ///
    /// To work with objects that can be null, use `Option<Gd<T>>` instead. For APIs that accept `Variant`, you can pass [`Variant::nil()`].
    ///
    /// # Nullability
    /// <div class="warning">
    /// The GDExtension API does not inform about nullability of its function parameters. It is up to you to verify that the arguments you pass
    /// are only null when this is allowed. Doing this wrong should be safe, but can lead to the function call failing.
    /// </div>
    ///
    /// # Example
    /// ```no_run
    /// # fn some_node() -> Gd<Node> { unimplemented!() }
    /// use godot::prelude::*;
    ///
    /// let mut shape: Gd<Node> = some_node();
    /// shape.set_owner(Gd::null_arg());
    pub fn null_arg() -> impl crate::meta::AsObjectArg<T> {
        crate::meta::ObjectNullArg(std::marker::PhantomData)
    }
}

impl<T> Gd<T>
where
    T: WithSignals,
{
    /// Access user-defined signals of this object.
    ///
    /// For classes that have at least one `#[signal]` defined, returns a collection of signal names. Each returned signal has a specialized
    /// API for connecting and emitting signals in a type-safe way. This method is the equivalent of [`WithUserSignals::signals()`], but when
    /// called externally (not from `self`). Furthermore, this is also available for engine classes, not just user-defined ones.
    ///
    /// When you are within the `impl` of a class, use `self.signals()` directly instead.
    ///
    /// If you haven't already, read the [book chapter about signals](https://godot-rust.github.io/book/register/signals.html) for a
    /// walkthrough.
    ///
    /// [`WithUserSignals::signals()`]: crate::obj::WithUserSignals::signals()
    #[cfg(since_api = "4.2")]
    pub fn signals(&self) -> T::SignalCollection<'_, T> {
        T::__signals_from_external(self)
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Trait impls

/// Dereferences to the nearest engine class, enabling direct calls to its `&self` methods.
///
/// For engine classes, returns `T` itself. For user classes, returns `T::Base` (the direct engine base class).
/// The bound ensures that the target is always an engine-provided class.
impl<T: GodotClass> Deref for Gd<T>
where
    GdDerefTarget<T>: Bounds<Declarer = bounds::DeclEngine>,
{
    // Target is always an engine class:
    // * if T is an engine class => T
    // * if T is a user class => T::Base
    type Target = GdDerefTarget<T>;

    fn deref(&self) -> &Self::Target {
        self.raw.as_target()
    }
}

/// Mutably dereferences to the nearest engine class, enabling direct calls to its `&mut self` methods.
///
/// For engine classes, returns `T` itself. For user classes, returns `T::Base` (the direct engine base class).
/// The bound ensures that the target is always an engine-provided class.
impl<T: GodotClass> DerefMut for Gd<T>
where
    GdDerefTarget<T>: Bounds<Declarer = bounds::DeclEngine>,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.raw.as_target_mut()
    }
}

impl<T: GodotClass> GodotConvert for Gd<T> {
    type Via = Gd<T>;
}

impl<T: GodotClass> ToGodot for Gd<T> {
    // TODO return RefArg here?
    type ToVia<'v> = Gd<T>;

    fn to_godot(&self) -> Self::ToVia<'_> {
        self.raw.check_rtti("to_godot");
        self.clone()
    }
}

impl<T: GodotClass> FromGodot for Gd<T> {
    fn try_from_godot(via: Self::Via) -> Result<Self, ConvertError> {
        Ok(via)
    }
}

// Keep in sync with DynGd.
impl<T: GodotClass> GodotType for Gd<T> {
    // Some #[doc(hidden)] are repeated despite already declared in trait; some IDEs suggest in auto-complete otherwise.
    type Ffi = RawGd<T>;

    type ToFfi<'f>
        = RefArg<'f, RawGd<T>>
    where
        Self: 'f;

    #[doc(hidden)]
    fn to_ffi(&self) -> Self::ToFfi<'_> {
        RefArg::new(&self.raw)
    }

    #[doc(hidden)]
    fn into_ffi(self) -> Self::Ffi {
        self.raw
    }

    fn try_from_ffi(raw: Self::Ffi) -> Result<Self, ConvertError> {
        if raw.is_null() {
            Err(FromFfiError::NullRawGd.into_error(raw))
        } else {
            Ok(Self { raw })
        }
    }

    fn class_name() -> ClassName {
        T::class_name()
    }

    fn godot_type_name() -> String {
        T::class_name().to_string()
    }

    fn qualifies_as_special_none(from_variant: &Variant) -> bool {
        // Behavior in Godot 4.2 when unsetting an #[export]'ed property:
        // 🔁 reset button: passes null object pointer inside Variant (as expected).
        // 🧹 clear button: sends a NodePath with an empty string (!?).

        // We recognize the latter case and return a Gd::null() instead of failing to convert the NodePath.
        if let Ok(node_path) = from_variant.try_to::<NodePath>() {
            if node_path.is_empty() {
                return true;
            }
        }

        false
    }
}

impl<T: GodotClass> ArrayElement for Gd<T> {
    fn element_type_string() -> String {
        // See also impl Export for Gd<T>.
        object_export_element_type_string::<T>(T::class_name())
    }
}

impl<T: GodotClass> ArrayElement for Option<Gd<T>> {
    fn element_type_string() -> String {
        Gd::<T>::element_type_string()
    }
}

/*
// TODO find a way to generalize AsArg to derived->base conversions without breaking type inference in array![].
// Possibly we could use a "canonical type" with unambiguous mapping (&Gd<T> -> &Gd<T>, not &Gd<T> -> &Gd<TBase>).
// See also regression test in array_test.rs.

impl<'r, T, TBase> AsArg<Gd<TBase>> for &'r Gd<T>
where
    T: Inherits<TBase>,
    TBase: GodotClass,
{
    #[doc(hidden)] // Repeated despite already hidden in trait; some IDEs suggest this otherwise.
    fn into_arg<'cow>(self) -> CowArg<'cow, Gd<TBase>>
    where
        'r: 'cow, // Original reference must be valid for at least as long as the returned cow.
    {
        // Performance: clones unnecessarily, which has overhead for ref-counted objects.
        // A result of being generic over base objects and allowing T: Inherits<Base> rather than just T == Base.
        // Was previously `CowArg::Borrowed(self)`. Borrowed() can maybe be specialized for objects, or combined with AsObjectArg.

        CowArg::Owned(self.clone().upcast::<TBase>())
    }
}
*/

impl<T: GodotClass> ParamType for Gd<T> {
    type ArgPassing = ByRef;
}

impl<T: GodotClass> AsArg<Option<Gd<T>>> for Option<&Gd<T>> {
    fn into_arg<'cow>(self) -> CowArg<'cow, Option<Gd<T>>> {
        // TODO avoid cloning.
        match self {
            Some(gd) => CowArg::Owned(Some(gd.clone())),
            None => CowArg::Owned(None),
        }
    }
}

impl<T: GodotClass> ParamType for Option<Gd<T>> {
    type ArgPassing = ByRef;
}

impl<T> Default for Gd<T>
where
    T: cap::GodotDefault + Bounds<Memory = bounds::MemRefCounted>,
{
    /// Creates a default-constructed `T` inside a smart pointer.
    ///
    /// This is equivalent to the GDScript expression `T.new()`, and to the shorter Rust expression `T::new_gd()`.
    ///
    /// This trait is only implemented for reference-counted classes. Classes with manually-managed memory (e.g. `Node`) are not covered,
    /// because they need explicit memory management, and deriving `Default` has a high chance of the user forgetting to call `free()` on those.
    /// `T::new_alloc()` should be used for those instead.
    fn default() -> Self {
        T::__godot_default()
    }
}

impl<T: GodotClass> Clone for Gd<T> {
    fn clone(&self) -> Self {
        out!("Gd::clone");
        Self {
            raw: self.raw.clone(),
        }
    }
}

impl<T: GodotClass> Var for Gd<T> {
    fn get_property(&self) -> Self::Via {
        self.to_godot()
    }

    fn set_property(&mut self, value: Self::Via) {
        *self = FromGodot::from_godot(value)
    }
}

/// See [`Gd` Exporting](struct.Gd.html#exporting) section.
impl<T> Export for Option<Gd<T>>
where
    T: GodotClass + Bounds<Exportable = bounds::Yes>,
    Option<Gd<T>>: Var,
{
    fn export_hint() -> PropertyHintInfo {
        PropertyHintInfo::export_gd::<T>()
    }

    #[doc(hidden)]
    fn as_node_class() -> Option<ClassName> {
        PropertyHintInfo::object_as_node_class::<T>()
    }
}

impl<T: GodotClass> Default for OnEditor<Gd<T>> {
    fn default() -> Self {
        OnEditor::gd_invalid()
    }
}

impl<T> GodotConvert for OnEditor<Gd<T>>
where
    T: GodotClass,
    Option<<Gd<T> as GodotConvert>::Via>: GodotType,
{
    type Via = Option<<Gd<T> as GodotConvert>::Via>;
}

impl<T> Var for OnEditor<Gd<T>>
where
    T: GodotClass,
{
    fn get_property(&self) -> Self::Via {
        Self::get_property_inner(self)
    }

    fn set_property(&mut self, value: Self::Via) {
        Self::set_property_inner(self, value)
    }
}

/// See [`Gd` Exporting](struct.Gd.html#exporting) section.
impl<T> Export for OnEditor<Gd<T>>
where
    Self: Var,
    T: GodotClass + Bounds<Exportable = bounds::Yes>,
{
    fn export_hint() -> PropertyHintInfo {
        PropertyHintInfo::export_gd::<T>()
    }

    #[doc(hidden)]
    fn as_node_class() -> Option<ClassName> {
        PropertyHintInfo::object_as_node_class::<T>()
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
        classes::display_string(self, f)
    }
}

impl<T: GodotClass> Debug for Gd<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        classes::debug_string(self, f, "Gd")
    }
}

impl<T: GodotClass> std::hash::Hash for Gd<T> {
    /// ⚠️ Hashes this object based on its instance ID.
    ///
    /// # Panics
    /// When `self` is dead.
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.instance_id().hash(state);
    }
}

// Gd unwinding across panics does not invalidate any invariants;
// its mutability is anyway present, in the Godot engine.
impl<T: GodotClass> std::panic::UnwindSafe for Gd<T> {}
impl<T: GodotClass> std::panic::RefUnwindSafe for Gd<T> {}
