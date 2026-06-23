/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::fmt::{Debug, Display, Formatter, Result as FmtResult};
use std::ops::{Deref, DerefMut};

use godot_ffi as sys;
use godot_ffi::is_main_thread;
use sys::{SysPtr as _, static_assert_eq_size_align};

use crate::builtin::{Callable, ExCall, NodePath, StringName, Variant};
use crate::meta::error::{ConvertError, FromFfiError};
use crate::meta::shape::GodotShape;
use crate::meta::{
    AsArg, ClassId, Element, FromGodot, GodotConvert, GodotNullableType, GodotType, RefArg, ToGodot,
};
use crate::obj::{
    Bounds, DynGd, GdDerefTarget, GdMut, GdRef, GodotClass, Inherits, InstanceId, OnEditor, RawGd,
    WithBaseField, WithSignals, WithUserRpcs, bounds, cap,
};
use crate::private::{PanicPayload, callbacks};
use crate::registry::class::try_dynify_object;
use crate::registry::info::PropertyHintInfo;
use crate::registry::property::{Export, SimpleVar, Var};
use crate::{classes, meta, out};

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
/// For type conversions, please read the [`godot::meta` module docs](../meta/index.html).
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
        let object_ptr = callbacks::create_custom(init, true) // or propagate panic.
            .unwrap_or_else(|payload| PanicPayload::repanic(payload));

        unsafe { Gd::from_constructed_obj_sys(object_ptr) }
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
    /// * If the object is a **placeholder instance** -- i.e. it has no Rust instance attached. This can happen when a non-`#[class(tool)]`
    ///   object is loaded or instantiated in the editor. Since Godot 4.3, non-tool classes are registered as "runtime classes", meaning
    ///   the editor only creates a Godot-side placeholder without invoking the Rust constructor. If you need to `bind()` such an object
    ///   in the editor (e.g. a loaded resource), mark its class as `#[class(tool)]`.
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
    /// * If the object is a placeholder instance with no Rust part. See [`bind()`][Self::bind] for details.
    pub fn bind_mut(&mut self) -> GdMut<'_, T> {
        self.raw.bind_mut()
    }

    /// Returns `true` if this object has no Rust instance attached (placeholder).
    ///
    /// In the Godot editor, classes that are not marked `#[class(tool)]` are replaced with _placeholder instances_ (Godot 4.3+ "runtime classes").
    /// From Godot's perspective the instance still exists, so scenes and script code referring to it do not break, but the Rust side is absent.
    ///
    /// Specifically, the following logic is **disabled** for a placeholder:
    /// * Rust-side objects. As a result, [`bind()`][Self::bind] and [`bind_mut()`][Self::bind_mut] panic on placeholders.
    ///   Use this method to branch, or mark the class `#[class(tool)]` if editor-side Rust state is required.
    /// * `init()` constructor -- *not* called on `new_alloc()` / `new_gd()` / `ClassDB.instantiate()` in the editor. However, Godot _does_
    ///   invoke `init()` exactly once per class at editor startup, to populate its default-value cache.
    /// * Custom property accessors (`#[var(get = ..., set = ...)]`, `IObject::get_property` / `set_property`). Placeholders keep their
    ///   own property map: `set()` stores into it; `get()` returns the stored value or falls back to the class's default-value cache.
    /// * Virtual callbacks (`ready`, `process`, `enter_tree`, `notification`, `on_property_get_revert`, ...) -- replaced with Godot-side stubs
    ///   (e.g. `property_can_revert` always returns `false`, `property_get_revert` always returns nil). Rust overrides never run.
    /// * `#[func]` methods -- callable through GDScript / `Callable`, but they `bind()` the receiver internally and will therefore panic.
    /// * Signal connections wired up in `init()` or `ready()` -- since those methods don't run (except for one-time `init()` filling defaults).
    ///
    /// Note that only `#[export]` fields populate the default-value cache (their `PropertyUsageFlags` include the storage/editor bits).
    /// `#[var]`-only fields do not, so placeholder `get()` returns `nil` for them rather than the value assigned in `init()`. `set()` on the
    /// placeholder accepts both kinds and stores them, but cross-instance state is not shared.
    ///
    /// The following operations still work as usual on a placeholder:
    /// * Holding the `Gd<T>` pointer, cloning it, comparing instance IDs, freeing it.
    /// * Upcasts and downcasts -- the Godot class hierarchy is intact, and `Object::get_class()` reports the user-declared name (not internal
    ///   `PlaceholderExtensionInstance`).
    /// * `get`, `set`, `get_property_list()`, etc. However, they access the static map and don't route to Rust `IObject` virtual methods.
    ///
    /// On Godot versions before 4.3 placeholder substitution does not exist; non-tool classes are instead filtered out at registration when the
    /// `tool_only_in_editor` config option is enabled (the default). This method then always returns `false`.
    #[cfg(all(feature = "itest", feature = "upcoming-editor-placeholders"))]
    pub fn is_editor_placeholder(&self) -> bool {
        self.raw.storage().is_none()
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
                T::class_id(),
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

    /// Returns the dynamic type of the object as [`ClassId`].
    ///
    /// Retrieves the class name of the object at runtime, which can differ from [`T::class_id()`][GodotClass::class_id] if derived
    /// classes are involved (e.g. a `Gd<Node>` whose dynamic type is `Sprite2D`, or a GDScript class inheriting `T`).
    ///
    /// Unlike [`Object::get_class()`][crate::classes::Object::get_class], this needs no `Inherits<Object>` bound and returns a
    /// comparable [`ClassId`] instead of `GString`.
    ///
    /// To test whether the dynamic class _inherits_ a given class (not just equals it), use [`is_dynamic_class()`][Self::is_dynamic_class] or
    ///  [`is_dynamic_class_of()`][Self::is_dynamic_class_of].
    pub fn dynamic_class(&self) -> ClassId {
        ClassId::new_dynamic(self.dynamic_class_string().to_string())
    }

    /// Returns whether the dynamic type of the object is `class_id` or a subclass thereof.
    ///
    /// Corresponds to GDScript's `is_class()` / [`Object::is_class()`][crate::classes::Object::is_class], but accepts a typed [`ClassId`]
    /// argument and needs no `Inherits<Object>` bound. See also [`is_dynamic_class_of()`][Self::is_dynamic_class_of] for compile-time.
    ///
    /// Note that `class_id` is matched by name only; this is a runtime check based on Godot's class hierarchy. For a strict equality
    /// check against the dynamic class without walking the hierarchy, compare against [`dynamic_class()`][Self::dynamic_class] directly.
    pub fn is_dynamic_class(&self, class_id: ClassId) -> bool {
        self.raw.is_dynamic_class(class_id)
    }

    /// Returns whether the dynamic type of the object is `U` or a subclass thereof.
    ///
    /// See also [`is_dynamic_class()`][Self::is_dynamic_class] for runtime arguments, and [`cast()`][Self::cast]/
    /// [`try_cast()`][Self::try_cast] for obtaining the result of this check.
    pub fn is_dynamic_class_of<U: GodotClass>(&self) -> bool {
        self.is_dynamic_class(U::class_id())
    }

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

    /// Create a non-owning pointer from this.
    ///
    /// # Safety
    /// Must be destroyed with [`drop_weak()`][Self::drop_weak]; regular `Drop` will cause use-after-free.
    pub(crate) unsafe fn clone_weak(&self) -> Self {
        // SAFETY: delegated to caller.
        unsafe { Gd::from_obj_sys_weak(self.obj_sys()) }
    }

    /// Drop without decrementing ref-counter.
    ///
    /// Needed in situations where the instance should effectively be forgotten, but without leaking other associated data.
    pub(crate) fn drop_weak(self) {
        // As soon as fields need custom Drop, this won't be enough anymore.
        std::mem::forget(self);
    }

    #[cfg(feature = "itest")]
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
    pub fn __upcast_object(self) -> Gd<classes::Object> {
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
                from = T::class_id(),
                to = Derived::class_id(),
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
        // Behavior of default instance creation -- see also https://github.com/godot-rust/gdext/issues/1404.
        //
        // With `upcoming-editor-placeholders` (future v0.6 default):
        // * Editor: use ClassDB.instantiate() -> C++ instantiate_internal().
        //   * Tool class    -> Godot creates instance regularly (extra Variant roundtrip, but editor usually not perf-critical).
        //   * Runtime class -> Godot substitutes placeholder instance.
        // * Runtime: directly invoke `create` callback.
        //   * Any class     -> Godot creates instance regularly (optimized).
        // * Unknown (for Godot < 4.4 && stage < Scene) -> behave like Runtime.
        //   Editor/ClassDb::instantiate path would be correct in all cases, but ClassDB isn't available on all levels. Thus we can only do
        //   the runtime path. It means that if runtime classes are constructed in level < Scene, they will not be placeholdered (rare case).
        //
        // Without the feature (v0.5-compatible default): editor branch is skipped; all states fall through to the direct `create` callback
        // below, returning a real Rust instance even for non-tool classes in the editor. Migration warning below flags the v0.6 change.
        #[cfg(feature = "upcoming-editor-placeholders")]
        if sys::is_editor_or_unknown().unwrap_or(false) {
            let class_name = T::class_id().to_string_name();

            // Note: C API classdb_construct_object[2|3] calls C++ instantiate_no_placeholders(), which skips placeholder substitution.
            // Instead we use ClassDB.instantiate() -> C++ _instantiate_internal().
            use crate::obj::Singleton as _;
            let variant = classes::ClassDb::singleton().instantiate(&class_name);
            return variant.try_to::<Self>().unwrap_or_else(|_| {
                panic!("ClassDB.instantiate({class_name}) failed -- class not registered or not instantiable")
            });
        }

        // v0.6 migration: under the legacy path (no `upcoming-editor-placeholders`), `T::new_alloc()` / `T::new_gd()` returns a real Rust
        // instance even for non-`#[class(tool)]` classes in the editor. In v0.6 this becomes a placeholder, silently losing Rust-side
        // logic (init/ready/...). One warning per class id, then backtrace printed to stderr so user can locate caller.
        #[cfg(not(feature = "upcoming-editor-placeholders"))]
        let class_id = T::class_id();
        #[cfg(not(feature = "upcoming-editor-placeholders"))]
        if sys::is_editor_or_unknown().unwrap_or(false)
            && crate::registry::class::is_class_tool(class_id) == Some(false)
        {
            use std::collections::HashSet;

            // Persists for the process lifetime, including across hot reloads -- one warning per class per process, not per reload.
            static WARNED: sys::Global<HashSet<ClassId>> = sys::Global::default();

            let is_new = WARNED.lock().insert(class_id);
            if is_new {
                sys::defer_startup_warn!(
                    id: "EditorPlaceholderV06",
                    "godot-rust v0.6 will change editor behavior for non-`#[class(tool)]` runtime classes.\n\
                    Class `{class_id}` creation in editor now returns real Rust instance; v0.6 will return a placeholder (details with RUST_BACKTRACE=1).\n\
                    Opt in early via the `upcoming-editor-placeholders` feature, or mark the class as `#[class(tool)]` if it runs in the editor.",
                );

                // If RUST_BACKTRACE is set, print backtrace.
                let bt = std::backtrace::Backtrace::capture();
                if bt.status() == std::backtrace::BacktraceStatus::Captured {
                    eprintln!(
                        "Backtrace for `{class_id}` (v0.6 editor-placeholder migration):\n{bt}"
                    );
                }
            }
        }

        // Fast path if not running in the editor: bypass substitution and directly call creation func.
        unsafe {
            // Default value (and compat one) for `p_notify_postinitialize` is true in Godot.
            #[cfg(since_api = "4.4")]
            let object_ptr = callbacks::create::<T>(std::ptr::null_mut(), sys::conv::SYS_TRUE);
            #[cfg(before_api = "4.4")]
            let object_ptr = callbacks::create::<T>(std::ptr::null_mut());

            Gd::from_constructed_obj_sys(object_ptr)
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

    /// Builder for advanced calls: deferred, fallible, async or `Array`-based argument passing.
    ///
    /// This is the unified successor to `Object::call`, `Object::callv`, `Object::call_deferred` and `Object::try_call`. See
    /// [`ExCall`][crate::builtin::ExCall] for the available terminal operations.
    ///
    /// For the common case of an immediate synchronous call, `Object::call(method, args)` (through `Deref`) remains the shorthand.
    pub fn call_ex<'ex>(&self, method: impl AsArg<StringName>) -> ExCall<'ex> {
        crate::meta::arg_into_owned!(method);
        ExCall::on_owned_variant(self.to_variant(), method)
    }

    /// Creates a new callable linked to the given object from **single-threaded** Rust function or closure.
    /// This is shorter syntax for [`Callable::from_linked_fn()`].
    ///
    /// `name` is used for the string representation of the closure, which helps with debugging.
    ///
    /// Such a callable will be automatically invalidated by Godot when a linked Object is freed.
    /// If you need a Callable which can live indefinitely, use [`Callable::from_fn()`].
    pub fn linked_callable<R, F>(
        &self,
        method_name: impl Into<crate::builtin::CowStr>,
        rust_function: F,
    ) -> Callable
    where
        R: ToGodot,
        F: 'static + FnMut(&[&Variant]) -> R,
    {
        Callable::from_linked_fn(method_name, self, rust_function)
    }

    /// Used by caller to transform pointer of freshly created instance into `Gd<T>`. This is default in most initializations from FFI.
    ///
    /// Before 4.7 Godot (including GDExtension layer) returns not fully-initialized instance and initializing it is a caller
    /// responsibility, which is done with [`Self::from_obj_sys`].
    ///
    /// After 4.7 Godot (and GDExtension layer too) returns fully-initialized instance to the caller, and [`Self::from_obj_sys_weak`]
    /// is used instead.
    ///
    /// In other words, before 4.7 it was something along the lines of:
    /// construct base -> do init/postinit -> CALLER initializes instance
    ///
    /// While afterwards we ended with:
    /// construct initialized base -> do init/postinit -> CALLER receives initialized instance.
    ///
    /// # Safety
    /// `ptr` must point to a valid object of this type.
    pub(crate) unsafe fn from_constructed_obj_sys(ptr: sys::GDExtensionObjectPtr) -> Self {
        #[cfg(before_api = "4.7")]
        let obj = unsafe { Gd::<T>::from_obj_sys(ptr) };

        #[cfg(since_api = "4.7")]
        let obj = unsafe { Gd::<T>::from_obj_sys_weak(ptr) };

        obj
    }

    pub(crate) unsafe fn from_obj_sys_or_none(
        ptr: sys::GDExtensionObjectPtr,
    ) -> Result<Self, ConvertError> {
        unsafe {
            // Used to have a flag to select RawGd::from_obj_sys_weak(ptr) for Base::to_init_gd(), but solved differently in the end.
            let obj = RawGd::from_obj_sys(ptr);

            Self::try_from_ffi(obj)
        }
    }

    /// Initializes this `Gd<T>` from the object pointer as a **strong ref**, meaning it initializes/increments the reference counter and keeps
    /// the object alive.
    ///
    /// This is the default for most initializations from FFI. In cases where the reference counter should explicitly **not** be updated,
    /// [`Self::from_obj_sys_weak`] is available.
    ///
    /// # Safety
    /// `ptr` must point to a valid object of this type.
    pub(crate) unsafe fn from_obj_sys(ptr: sys::GDExtensionObjectPtr) -> Self {
        sys::strict_assert!(
            !ptr.is_null(),
            "Gd::from_obj_sys() called with null pointer"
        );

        unsafe { Self::from_obj_sys_or_none(ptr) }.unwrap()
    }

    /// # Safety
    /// `ptr` must point to a valid object of this type, or null.
    pub(crate) unsafe fn from_obj_sys_weak_or_none(
        ptr: sys::GDExtensionObjectPtr,
    ) -> Result<Self, ConvertError> {
        unsafe { Self::try_from_ffi(RawGd::from_obj_sys_weak(ptr)) }
    }

    /// # Safety
    /// `ptr` must point to a valid object of this type.
    pub(crate) unsafe fn from_obj_sys_weak(ptr: sys::GDExtensionObjectPtr) -> Self {
        unsafe { Self::from_obj_sys_weak_or_none(ptr).unwrap() }
    }

    #[cfg(feature = "itest")]
    #[doc(hidden)]
    pub unsafe fn __from_obj_sys_weak(ptr: sys::GDExtensionObjectPtr) -> Self {
        unsafe { Self::from_obj_sys_weak(ptr) }
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

        // Initialize pointer with given function. Return Some(ptr) on success, and None otherwise.
        // SAFETY: init_fn takes a type-ptr pointing to an object-ptr.
        let object_ptr = unsafe { super::raw_object_init(init_fn) };

        // Do not increment ref-count; assumed to be return value from FFI.
        sys::ptr_then(object_ptr, |ptr| unsafe { Gd::from_obj_sys_weak(ptr) })
    }

    /// Defers the given closure to run during [idle time](https://docs.godotengine.org/en/stable/classes/class_object.html#class-object-method-call-deferred).
    ///
    /// This is a type-safe alternative to [`Object::call_deferred()`][crate::classes::Object::call_deferred]. The closure receives
    /// `&mut Self` allowing direct access to Rust fields and methods.
    ///
    /// This method is only available for user-defined classes with a `Base<T>` field.
    /// For engine classes, use [`run_deferred_gd()`][Self::run_deferred_gd] instead.
    ///
    /// See also [`WithBaseField::run_deferred()`] if you are within an `impl` block and have access to `self`.
    ///
    /// # Panics
    /// If called outside the main thread.
    pub fn run_deferred<F>(&mut self, mut_self_method: F)
    where
        T: WithBaseField,
        F: FnOnce(&mut T) + 'static,
    {
        self.run_deferred_gd(move |mut gd| {
            let mut guard = gd.bind_mut();
            mut_self_method(&mut *guard);
        });
    }

    /// Defers the given closure to run during [idle time](https://docs.godotengine.org/en/stable/classes/class_object.html#class-object-method-call-deferred).
    ///
    /// This is a type-safe alternative to [`Object::call_deferred()`][crate::classes::Object::call_deferred]. The closure receives
    /// `Gd<T>`, which can be used to call engine methods or [`bind()`][Gd::bind]/[`bind_mut()`][Gd::bind_mut] to access the Rust object.
    ///
    /// See also [`WithBaseField::run_deferred_gd()`] if you are within an `impl` block and have access to `self`.
    ///
    /// # Panics
    /// If called outside the main thread.
    pub fn run_deferred_gd<F>(&mut self, gd_function: F)
    where
        F: FnOnce(Gd<T>) + 'static,
    {
        let obj = self.clone();
        assert!(
            is_main_thread(),
            "`run_deferred` must be called on the main thread"
        );

        let callable = Callable::from_once_fn("run_deferred", move |_| {
            // Skip if the engine is exiting: the deferred call would otherwise run after `SceneTree` teardown, where accessing freed objects
            // (e.g. autoloads) panics. This matches Godot's own `call_deferred()`, which drops queued calls to freed objects at shutdown.
            // See `async_runtime::is_engine_exiting()`.
            if crate::task::is_engine_exiting() {
                return;
            }
            gd_function(obj);
        });
        callable.call_deferred(&[]);
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
                use crate::private::{ErrorPrintLevel, has_error_print_level};
                if has_error_print_level(ErrorPrintLevel::Reduced) {
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
        if ref_counted != Some(false) || (cfg!(safeguards_balanced) && !self.is_instance_valid()) {
            return error_or_panic("called free() on already destroyed object".to_string());
        }

        // If the object is still alive, make sure the dynamic type matches. Necessary because subsequent checks may rely on the
        // static type information to be correct. This is a no-op in Release mode.
        // Skip check during panic unwind; would need to rewrite whole thing to use Result instead. Having BOTH panic-in-panic and bad type is
        // a very unlikely corner case.
        #[cfg(safeguards_strict)]
        if !is_panic_unwind {
            self.raw
                .check_dynamic_type(&crate::meta::CallContext::gd::<T>("free"));
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

        // Deallocate associated data in Gd, without destroying the object pointer itself (already done above).
        self.drop_weak()
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

impl Gd<classes::Object> {
    /// Whether the object inherits `RefCounted`.
    ///
    /// This is a very fast check that involves no FFI roundtrip.
    ///
    /// Implemented only on `Object` because for all other classes, this property is statically known.
    pub fn is_ref_counted(&self) -> bool {
        self.instance_id_unchecked().is_ref_counted()
    }
}

impl<T> Gd<T>
where
    T: GodotClass + Bounds<Declarer = bounds::DeclEngine>,
{
    /// Represents `null` when passing an object argument to Godot.
    ///
    /// This expression is only intended for function argument lists. It can be used whenever a Godot signature accepts
    /// [`AsArg<Option<Gd<T>>>`][crate::meta::AsArg]. `Gd::null_arg()` as an argument is equivalent to `Option::<Gd<T>>::None`, but less wordy.
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
    pub fn null_arg() -> impl AsArg<Option<Gd<T>>> {
        meta::NullArg(std::marker::PhantomData)
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
    pub fn signals(&self) -> T::SignalCollection<'_, T> {
        T::__signals_from_external(self)
    }
}

impl<T> Gd<T>
where
    T: WithUserRpcs,
{
    /// Access type-safe RPCs of this object.
    ///
    /// For classes that have at least one `#[rpc]` defined, returns a collection with one method per RPC, allowing them to be called in a
    /// type-safe way. This method is the equivalent of [`WithUserRpcs::rpcs()`][crate::obj::WithUserRpcs::rpcs], but when called externally
    /// (not from `self`).
    ///
    /// When you are within the `impl` of a class, use `self.rpcs()` directly instead.
    pub fn rpcs(&self) -> T::RpcCollection<'_> {
        T::__rpcs_from_external(self)
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

    fn godot_shape() -> GodotShape {
        use crate::meta::shape::ClassHeritage;

        let heritage = if T::inherits::<classes::Resource>() {
            ClassHeritage::Resource
        } else if T::inherits::<classes::Node>() {
            ClassHeritage::Node
        } else {
            ClassHeritage::Other
        };

        let class_id = T::class_id();
        GodotShape::Class {
            class_id,
            heritage,
            is_nullable: false,
        }
    }
}

impl<T: GodotClass> ToGodot for Gd<T> {
    type Pass = meta::ByObject;

    fn to_godot(&self) -> &Self {
        // Note: Gd<T> never null, so no need to check raw.is_null().
        self.raw.check_rtti("to_godot");
        self
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

    /// Recognizes the Godot inspector's "clear" action on an `#[export]`ed `Option<Gd<T>>` property.
    ///
    /// When unsetting such a property in the editor, Godot 4.2 behaves inconsistently:
    /// - 🔁 reset button: passes null object pointer inside the variant (as expected, handled by the regular nil check).
    /// - 🧹 clear button: sends a `NodePath` with an empty string, rather than a nil variant.
    ///
    /// We detect the latter case and return `Gd::null()` instead of failing to convert the `NodePath` (i.e. panic in `from_variant()` or
    /// error in `try_from_variant()`).
    fn qualifies_as_special_none(from_variant: &Variant) -> bool {
        if let Ok(node_path) = from_variant.try_to::<NodePath>()
            && node_path.is_empty()
        {
            return true;
        }

        false
    }

    fn as_object_arg(&self) -> meta::ObjectArg<'_> {
        meta::ObjectArg::from_gd(self)
    }
}

impl<T: GodotClass> Element for Gd<T> {}

impl<T: GodotClass> GodotNullableType for Gd<T> {
    fn ffi_null() -> RawGd<T> {
        RawGd::null()
    }

    fn ffi_null_ref<'f>() -> RefArg<'f, RawGd<T>>
    where
        Self: 'f,
    {
        RefArg::null_ref()
    }

    fn ffi_is_null(ffi: &RawGd<T>) -> bool {
        ffi.is_null()
    }
}

impl<T: GodotClass> Element for Option<Gd<T>> {}

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

impl<T: GodotClass> SimpleVar for Gd<T> {}

/// See [`Gd` Exporting](struct.Gd.html#exporting) section.
impl<T> Export for Option<Gd<T>>
where
    T: GodotClass + Bounds<Exportable = bounds::Yes>,
    Option<Gd<T>>: Var,
{
    #[doc(hidden)]
    fn as_node_class() -> Option<ClassId> {
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

    fn godot_shape() -> GodotShape {
        Gd::<T>::godot_shape()
    }
}

impl<T> Var for OnEditor<Gd<T>>
where
    T: GodotClass,
{
    // Not Option<...> -- accessing from Rust through Var trait should not expose larger API than OnEditor itself.
    type PubType = <Gd<T> as GodotConvert>::Via;

    fn var_get(field: &Self) -> Self::Via {
        Self::get_property_inner(field)
    }

    fn var_set(field: &mut Self, value: Self::Via) {
        Self::set_property_inner(field, value);
    }

    fn var_pub_get(field: &Self) -> Self::PubType {
        Self::var_get(field).expect("generated #[var(pub)] getter: uninitialized OnEditor<Gd<T>>")
    }

    fn var_pub_set(field: &mut Self, value: Self::PubType) {
        Self::var_set(field, Some(value))
    }
}

/// See [`Gd` Exporting](struct.Gd.html#exporting) section.
impl<T> Export for OnEditor<Gd<T>>
where
    Self: Var,
    T: GodotClass + Bounds<Exportable = bounds::Yes>,
{
    #[doc(hidden)]
    fn as_node_class() -> Option<ClassId> {
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
