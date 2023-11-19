/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::fmt::{Debug, Display, Formatter, Result as FmtResult};
use std::ops::{Deref, DerefMut};

use godot_ffi as sys;
use godot_ffi::VariantType;
use sys::static_assert_eq_size;

use crate::builtin::meta::{FromGodot, GodotConvert, GodotType, ToGodot};
use crate::builtin::{Callable, StringName};
use crate::obj::{cap, dom, mem, EngineEnum, GdDerefTarget, GodotClass, Inherits, Share};
use crate::obj::{GdMut, GdRef, InstanceId};
use crate::property::{Export, Property, PropertyHintInfo, TypeStringHint};
use crate::{callbacks, engine, out};

use super::RawGd;

/// Smart pointer to objects owned by the Godot engine.
///
/// See also [chapter about objects][book] in the book.
///
/// This smart pointer can only hold _objects_ in the Godot sense: instances of Godot classes (`Node`, `RefCounted`, etc.)
/// or user-declared structs (declared with `#[derive(GodotClass)]`). It does **not** hold built-in types (`Vector3`, `Color`, `i32`).
///
/// `Gd<T>` never holds null objects. If you need nullability, use `Option<Gd<T>>`.
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
///   you are responsible of explicitly deallocating such objects using [`free()`][Self::free].<br><br>
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
/// | Type \ Memory Strategy | Ref-counted          | Manually managed      | Singleton               |
/// |------------------------|----------------------|-----------------------|-------------------------|
/// | **Engine type**        | `Resource::new()`    | `Node::new_alloc()`   | `Os::singleton()`       |
/// | **User type**          | `MyClass::new_gd()`  | `MyClass::alloc_gd()` | _(not yet implemented)_ |
///
/// In addition, the smart pointer can be constructed in multiple ways:
///
/// * [`Gd::default()`] for reference-counted types that are constructible. For user types, this means they must expose an `init` function
///   or have a generated one. `Gd::<T>::default()` is equivalent to the shorter `T::new_gd()` and primarily useful for derives or generics.
/// * [`Gd::from_init_fn(function)`][Gd::from_init_fn] for Rust objects with `#[base]` field, which are constructed inside the smart pointer.
///   This is a very handy function if you want to pass extra parameters to your object upon construction.
/// * [`Gd::from_object(rust_obj)`][Gd::from_object] for existing Rust objects without a `#[base]` field that are moved _into_ the smart pointer.
/// * [`Gd::from_instance_id(id)`][Gd::from_instance_id] and [`Gd::try_from_instance_id(id)`][Gd::try_from_instance_id]
///   to obtain a pointer to an object which is already alive in the engine.
///
/// # Binds
///
/// The [`bind()`][Self::bind] and [`bind_mut()`][Self::bind_mut] methods allow you to obtain a shared or exclusive guard to the user instance.
/// These provide interior mutability similar to [`RefCell`][std::cell::RefCell], with the addition that `Gd` simultaneously handles reference
/// counting (for some types `T`).
///
/// When you declare a `#[func]` method on your own class and it accepts `&self` or `&mut self`, an implicit `bind()` or `bind_mut()` call
/// on the owning `Gd<T>` is performed. This is important to keep in mind, as you can get into situations that violate dynamic borrow rules; for
/// example if you are inside a `&mut self` method, make a call to GDScript and indirectly call another method on the same object (re-entrancy).
///
/// [book]: https://godot-rust.github.io/book/intro/objects.html
/// [`Object`]: engine::Object
/// [`RefCounted`]: engine::RefCounted
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
static_assert_eq_size!(
    sys::GDExtensionObjectPtr,
    sys::types::OpaqueObject,
    "Godot FFI: pointer type `Object*` should have size advertised in JSON extension file"
);

/// _The methods in this impl block are only available for user-declared `T`, that is,
/// structs with `#[derive(GodotClass)]` but not Godot classes like `Node` or `RefCounted`._ <br><br>
impl<T> Gd<T>
where
    T: GodotClass<Declarer = dom::UserDomain>,
{
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
    /// let obj = Gd::from_init_fn(|my_base| {
    ///     // accepts the base and returns a constructed object containing it
    ///     MyClass { my_base, other_field: 732 }
    /// });
    /// ```
    pub fn from_init_fn<F>(init: F) -> Self
    where
        F: FnOnce(crate::obj::Base<T::Base>) -> T,
    {
        let object_ptr = callbacks::create_custom(init);
        unsafe { Gd::from_obj_sys(object_ptr) }
    }

    /// Moves a user-created object into this smart pointer, submitting ownership to the Godot engine.
    ///
    /// This is only useful for types `T` which do not store their base objects (if they have a base,
    /// you cannot construct them standalone).
    pub fn from_object(user_object: T) -> Self {
        Self::from_init_fn(move |_base| user_object)
    }

    #[deprecated = "Use `Gd::from_object()` instead."]
    pub fn new(user_object: T) -> Self {
        Self::from_object(user_object)
    }

    #[deprecated = "Use `Gd::default()` or the short-hands `T::new_gd()` and `T::alloc_gd()` instead."]
    pub fn new_default() -> Self
    where
        T: cap::GodotDefault,
    {
        Self::default_instance()
    }

    #[deprecated = "Use `Gd::from_init_fn()` instead."]
    pub fn with_base<F>(init: F) -> Self
    where
        F: FnOnce(crate::obj::Base<T::Base>) -> T,
    {
        Self::from_init_fn(init)
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
    /// Looks up the given instance ID and returns the associated object, if possible.
    ///
    /// If no such instance ID is registered, or if the dynamic type of the object behind that instance ID
    /// is not compatible with `T`, then `None` is returned.
    pub fn try_from_instance_id(instance_id: InstanceId) -> Option<Self> {
        let ptr = engine::object_ptr_from_id(instance_id);

        // SAFETY: assumes that the returned GDExtensionObjectPtr is convertible to Object* (i.e. C++ upcast doesn't modify the pointer)
        let untyped = unsafe { Gd::<engine::Object>::from_obj_sys_or_none(ptr)? };
        untyped.owned_cast::<T>().ok()
    }

    /// ⚠️ Looks up the given instance ID and returns the associated object.
    ///
    /// # Panics
    /// If no such instance ID is registered, or if the dynamic type of the object behind that instance ID
    /// is not compatible with `T`.
    pub fn from_instance_id(instance_id: InstanceId) -> Self {
        Self::try_from_instance_id(instance_id).unwrap_or_else(|| {
            panic!(
                "Instance ID {} does not belong to a valid object of class '{}'",
                instance_id,
                T::class_name()
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
        // SAFETY:
        // A `Gd` can only be created from a non-null `RawGd`. Meaning `raw.instance_id_unchecked()` will
        // always return `Some`.
        unsafe { self.raw.instance_id_unchecked().unwrap_unchecked() }
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
        self.instance_id_or_none().is_some()
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
    /// let obj: Gd<MyClass> = MyClass::alloc_gd();
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
                from = T::class_name(),
                to = Derived::class_name(),
            )
        })
    }

    /// Returns `Ok(cast_obj)` on success, `Err(self)` on error
    fn owned_cast<U>(self) -> Result<Gd<U>, Self>
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
            let object_ptr = crate::callbacks::create::<T>(std::ptr::null_mut());
            Gd::from_obj_sys(object_ptr)
        }
    }

    pub(crate) unsafe fn from_obj_sys_or_none(ptr: sys::GDExtensionObjectPtr) -> Option<Self> {
        Self::try_from_ffi(RawGd::from_obj_sys(ptr))
    }

    /// Initializes this `Gd<T>` from the object pointer as a **strong ref**, meaning
    /// it initializes/increments the reference counter and keeps the object alive.
    ///
    /// This is the default for most initializations from FFI. In cases where reference counter
    /// should explicitly **not** be updated, [`Self::from_obj_sys_weak`] is available.
    pub(crate) unsafe fn from_obj_sys(ptr: sys::GDExtensionObjectPtr) -> Self {
        Self::from_obj_sys_or_none(ptr).unwrap()
    }

    pub(crate) unsafe fn from_obj_sys_weak_or_none(ptr: sys::GDExtensionObjectPtr) -> Option<Self> {
        Self::try_from_ffi(RawGd::from_obj_sys_weak(ptr))
    }

    pub(crate) unsafe fn from_obj_sys_weak(ptr: sys::GDExtensionObjectPtr) -> Self {
        Self::from_obj_sys_weak_or_none(ptr).unwrap()
    }

    pub(crate) fn obj_sys(&self) -> sys::GDExtensionObjectPtr {
        self.raw.obj_sys()
    }
    /// Returns a callable referencing a method from this object named `method_name`.
    pub fn callable<S: Into<StringName>>(&self, method_name: S) -> Callable {
        Callable::from_object_method(self.clone(), method_name)
    }
}

impl<T: GodotClass> Deref for Gd<T> {
    // Target is always an engine class:
    // * if T is an engine class => T
    // * if T is a user class => T::Base
    type Target = GdDerefTarget<T>;

    fn deref(&self) -> &Self::Target {
        self.raw.as_target().expect("`Gd` is never null")
    }
}

impl<T: GodotClass> DerefMut for Gd<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.raw.as_target_mut().expect("`Gd` is never null")
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
        sys::ptr_then(object_ptr, |ptr| Gd::from_obj_sys_weak(ptr))
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
    /// - When the referred-to object has already been destroyed.
    /// - When this is invoked on an upcast `Gd<Object>` that dynamically points to a reference-counted type (i.e. operation not supported).
    /// - When the object is bound by an ongoing `bind()` or `bind_mut()` call (through a separate `Gd` pointer).
    pub fn free(self) {
        // Note: this method is NOT invoked when the free() call happens dynamically (e.g. through GDScript or reflection).
        // As such, do not use it for operations and validations to perform upon destruction.

        // TODO disallow for singletons, either only at runtime or both at compile time (new memory policy) and runtime
        use dom::Domain;

        // Runtime check in case of T=Object, no-op otherwise
        let ref_counted = T::Mem::is_ref_counted(&self.raw);
        assert_ne!(
            ref_counted, Some(true),
            "called free() on Gd<Object> which points to a RefCounted dynamic type; free() only supported for manually managed types\n\
            object: {self:?}"
        );

        // If ref_counted returned None, that means the instance was destroyed
        assert!(
            ref_counted == Some(false) && self.is_instance_valid(),
            "called free() on already destroyed object"
        );

        // SAFETY: object must be alive, which was just checked above. No multithreading here.
        // Also checked in the C free_instance_func callback, however error message can be more precise here and we don't need to instruct
        // the engine about object destruction. Both paths are tested.
        let bound = unsafe { T::Declarer::is_currently_bound(&self.raw) };
        assert!(
            !bound,
            "called free() while a bind() or bind_mut() call is active"
        );

        // SAFETY: object alive as checked.
        // This destroys the Storage instance, no need to run destructor again.
        unsafe {
            sys::interface_fn!(object_destroy)(self.raw.obj_sys());
        }

        std::mem::forget(self);
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Trait impls

impl<T: GodotClass> GodotConvert for Gd<T> {
    type Via = Gd<T>;
}

impl<T: GodotClass> ToGodot for Gd<T> {
    fn to_godot(&self) -> Self::Via {
        self.clone()
    }

    fn into_godot(self) -> Self::Via {
        self
    }
}

impl<T: GodotClass> FromGodot for Gd<T> {
    fn try_from_godot(via: Self::Via) -> Option<Self> {
        Some(via)
    }
}

impl<T: GodotClass> GodotType for Gd<T> {
    type Ffi = RawGd<T>;

    fn to_ffi(&self) -> Self::Ffi {
        self.raw.clone()
    }

    fn into_ffi(self) -> Self::Ffi {
        self.raw
    }

    fn try_from_ffi(raw: Self::Ffi) -> Option<Self> {
        if raw.is_null() {
            None
        } else {
            Some(Self { raw })
        }
    }

    fn class_name() -> crate::builtin::meta::ClassName {
        T::class_name()
    }

    fn godot_type_name() -> String {
        T::class_name().to_string()
    }
}

impl<T> Default for Gd<T>
where
    T: cap::GodotDefault + GodotClass<Mem = mem::StaticRefCount>,
{
    /// Creates a default-constructed `T` inside a smart pointer.
    ///
    /// This is equivalent to the GDScript expression `T.new()`, and to the shorter Rust expression `T::new_gd()`.
    ///
    /// This trait is only implemented for reference-counted classes. Classes with manually-managed memory (e.g. `Node`) are not covered,
    /// because they need explicit memory management, and deriving `Default` has a high chance of the user forgetting to call `free()` on those.
    /// `T::alloc_gd()` should be used for those instead.
    fn default() -> Self {
        T::__godot_default()
    }
}

impl<T: GodotClass> Clone for Gd<T> {
    fn clone(&self) -> Self {
        out!("Gd::clone");
        Self::from_ffi(self.raw.clone())
    }
}

impl<T: GodotClass> Share for Gd<T> {
    fn share(&self) -> Self {
        self.clone()
    }
}

impl<T: GodotClass> TypeStringHint for Gd<T> {
    fn type_string() -> String {
        use engine::global::PropertyHint;

        match Self::default_export_info().hint {
            hint @ (PropertyHint::PROPERTY_HINT_RESOURCE_TYPE
            | PropertyHint::PROPERTY_HINT_NODE_TYPE) => {
                format!(
                    "{}/{}:{}",
                    VariantType::Object as i32,
                    hint.ord(),
                    T::class_name()
                )
            }
            _ => format!("{}:", VariantType::Object as i32),
        }
    }
}

impl<T: GodotClass> Property for Gd<T> {
    type Intermediate = Self;

    fn get_property(&self) -> Self {
        self.clone()
    }

    fn set_property(&mut self, value: Self) {
        *self = value;
    }
}

impl<T: GodotClass> Export for Gd<T> {
    fn default_export_info() -> PropertyHintInfo {
        let hint = if T::inherits::<engine::Resource>() {
            engine::global::PropertyHint::PROPERTY_HINT_RESOURCE_TYPE
        } else if T::inherits::<engine::Node>() {
            engine::global::PropertyHint::PROPERTY_HINT_NODE_TYPE
        } else {
            engine::global::PropertyHint::PROPERTY_HINT_NONE
        };

        // Godot does this by default too; the hint is needed when the class is a resource/node,
        // but doesn't seem to make a difference otherwise.
        let hint_string = T::class_name().to_godot_string();

        PropertyHintInfo { hint, hint_string }
    }
}

// Trait impls Property, Export and TypeStringHint for Option<Gd<T>> are covered by blanket impl for Option<T>

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
        engine::display_string(self, f)
    }
}

impl<T: GodotClass> Debug for Gd<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        engine::debug_string(self, f, "Gd")
    }
}

// Gd unwinding across panics does not invalidate any invariants;
// its mutability is anyway present, in the Godot engine.
impl<T: GodotClass> std::panic::UnwindSafe for Gd<T> {}
impl<T: GodotClass> std::panic::RefUnwindSafe for Gd<T> {}
