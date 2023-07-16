/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::fmt::{Debug, Display, Formatter, Result as FmtResult};
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};
use std::ptr;

use godot_ffi as sys;
use godot_ffi::VariantType;
use sys::types::OpaqueObject;
use sys::{
    ffi_methods, interface_fn, static_assert_eq_size, GodotFfi, GodotNullablePtr, PtrcallType,
};

use crate::builtin::meta::{ClassName, VariantMetadata};
use crate::builtin::{
    Callable, FromVariant, StringName, ToVariant, Variant, VariantConversionError,
};
use crate::obj::dom::Domain as _;
use crate::obj::mem::Memory as _;
use crate::obj::{cap, dom, mem, EngineEnum, GodotClass, Inherits, Share};
use crate::obj::{GdMut, GdRef, InstanceId};
use crate::property::{Export, ExportInfo, Property, TypeStringHint};
use crate::storage::InstanceStorage;
use crate::{callbacks, engine, out};

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
pub struct Gd<T: GodotClass> {
    // Note: `opaque` has the same layout as GDExtensionObjectPtr == Object* in C++, i.e. the bytes represent a pointer
    // To receive a GDExtensionTypePtr == GDExtensionObjectPtr* == Object**, we need to get the address of this
    // Hence separate sys() for GDExtensionTypePtr, and obj_sys() for GDExtensionObjectPtr.
    // The former is the standard FFI type, while the latter is used in object-specific GDExtension engines.
    // pub(crate) because accessed in obj::dom
    pub(crate) opaque: OpaqueObject,

    // Last known instance ID -- this may no longer be valid!
    cached_instance_id: std::cell::Cell<Option<InstanceId>>,
    _marker: PhantomData<*const T>,
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
    /// Moves a user-created object into this smart pointer, submitting ownership to the Godot engine.
    ///
    /// This is only useful for types `T` which do not store their base objects (if they have a base,
    /// you cannot construct them standalone).
    pub fn new(user_object: T) -> Self {
        Self::with_base(move |_base| user_object)
    }

    /// Creates a default-constructed instance of `T` inside a smart pointer.
    ///
    /// This is equivalent to the GDScript expression `T.new()`.
    pub fn new_default() -> Self
    where
        T: cap::GodotInit,
    {
        unsafe {
            let object_ptr = callbacks::create::<T>(ptr::null_mut());
            Gd::from_obj_sys(object_ptr)
        }
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
        let object_ptr = callbacks::create_custom(init);
        unsafe { Gd::from_obj_sys(object_ptr) }
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
        GdRef::from_cell(self.storage().get())
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
        GdMut::from_cell(self.storage().get_mut())
    }

    /// Storage object associated with the extension instance
    // FIXME proper + safe interior mutability, also that Clippy is happy
    #[allow(clippy::mut_from_ref)]
    pub(crate) fn storage(&self) -> &mut InstanceStorage<T> {
        let callbacks = crate::storage::nop_instance_callbacks();

        unsafe {
            let token = sys::get_library() as *mut std::ffi::c_void;
            let binding =
                interface_fn!(object_get_instance_binding)(self.obj_sys(), token, &callbacks);

            debug_assert!(
                !binding.is_null(),
                "Class {} -- null instance; does the class have a Godot creator function?",
                std::any::type_name::<T>()
            );
            crate::private::as_storage::<T>(binding as sys::GDExtensionClassInstancePtr)
        }
    }
}

/// _The methods in this impl block are available for any `T`._ <br><br>
impl<T: GodotClass> Gd<T> {
    /// Looks up the given instance ID and returns the associated object, if possible.
    ///
    /// If no such instance ID is registered, or if the dynamic type of the object behind that instance ID
    /// is not compatible with `T`, then `None` is returned.
    pub fn try_from_instance_id(instance_id: InstanceId) -> Option<Self> {
        // SAFETY: Godot looks up ID in ObjectDB and returns null if not found
        let ptr = unsafe { interface_fn!(object_get_instance_from_id)(instance_id.to_u64()) };

        if ptr.is_null() {
            None
        } else {
            // SAFETY: assumes that the returned GDExtensionObjectPtr is convertible to Object* (i.e. C++ upcast doesn't modify the pointer)
            let untyped = unsafe { Gd::<engine::Object>::from_obj_sys(ptr) };
            untyped.owned_cast::<T>().ok()
        }
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
                T::CLASS_NAME
            )
        })
    }

    fn from_opaque(opaque: OpaqueObject) -> Self {
        let obj = Self {
            opaque,
            cached_instance_id: std::cell::Cell::new(None),
            _marker: PhantomData,
        };

        // Initialize instance ID cache
        let id = unsafe { interface_fn!(object_get_instance_id)(obj.obj_sys()) };
        let instance_id = InstanceId::try_from_u64(id)
            .expect("Gd initialization failed; did you call share() on a dead instance?");
        obj.cached_instance_id.set(Some(instance_id));

        obj
    }

    /// Returns the instance ID of this object, or `None` if the object is dead.
    pub fn instance_id_or_none(&self) -> Option<InstanceId> {
        let known_id = match self.cached_instance_id.get() {
            // Already dead
            None => return None,

            // Possibly alive
            Some(id) => id,
        };

        // Refreshes the internal cached ID on every call, as we cannot be sure that the object has not been
        // destroyed since last time. The only reliable way to find out is to call is_instance_id_valid().
        if engine::utilities::is_instance_id_valid(known_id.to_i64()) {
            Some(known_id)
        } else {
            self.cached_instance_id.set(None);
            None
        }
    }

    /// ⚠️ Returns the instance ID of this object, or `None` if no instance ID is cached.
    ///
    /// This function does not check that the returned instance ID points to a valid instance!
    /// Unless performance is a problem, use [`instance_id_or_none`].
    pub fn instance_id_or_none_unchecked(&self) -> Option<InstanceId> {
        self.cached_instance_id.get()
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
    /// let obj: Gd<MyClass> = Gd::new_default();
    /// let base = obj.share().upcast::<Node>();
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
                from = T::CLASS_NAME,
                to = Derived::CLASS_NAME,
            )
        })
    }

    // See use-site for explanation.
    fn is_cast_valid<U>(&self) -> bool
    where
        U: GodotClass,
    {
        let as_obj =
            unsafe { self.ffi_cast::<engine::Object>() }.expect("Everything inherits object");
        let cast_is_valid = as_obj.is_class(crate::builtin::GodotString::from(U::CLASS_NAME));
        std::mem::forget(as_obj);
        cast_is_valid
    }

    /// Returns `Ok(cast_obj)` on success, `Err(self)` on error
    fn owned_cast<U>(self) -> Result<Gd<U>, Self>
    where
        U: GodotClass,
    {
        // Workaround for bug in Godot 4.0 that makes casts always succeed (https://github.com/godot-rust/gdext/issues/158).
        // TODO once fixed in Godot, use #[cfg(before_api = "4.1")]
        if !self.is_cast_valid::<U>() {
            return Err(self);
        }

        // The unsafe { std::mem::transmute<&T, &Base>(self.inner()) } relies on the C++ static_cast class casts
        // to return the same pointer, however in theory those may yield a different pointer (VTable offset,
        // virtual inheritance etc.). It *seems* to work so far, but this is no indication it's not UB.
        //
        // The Deref/DerefMut impls for T implement an "implicit upcast" on the object (not Gd) level and
        // rely on this (e.g. &Node3D -> &Node).

        let result = unsafe { self.ffi_cast::<U>() };
        match result {
            Some(cast_obj) => {
                // duplicated ref, one must be wiped
                std::mem::forget(self);
                Ok(cast_obj)
            }
            None => Err(self),
        }
    }

    // Note: does not transfer ownership and is thus unsafe. Also operates on shared ref.
    // Either the parameter or the return value *must* be forgotten (since reference counts are not updated).
    unsafe fn ffi_cast<U>(&self) -> Option<Gd<U>>
    where
        U: GodotClass,
    {
        let class_name = ClassName::of::<U>();
        let class_tag = interface_fn!(classdb_get_class_tag)(class_name.string_sys());
        let cast_object_ptr = interface_fn!(object_cast_to)(self.obj_sys(), class_tag);

        // Create weak object, as ownership will be moved and reference-counter stays the same
        sys::ptr_then(cast_object_ptr, |ptr| Gd::from_obj_sys_weak(ptr))
    }

    pub(crate) fn as_ref_counted<R>(&self, apply: impl Fn(&mut engine::RefCounted) -> R) -> R {
        debug_assert!(
            self.is_instance_valid(),
            "as_ref_counted() on freed instance; maybe forgot to increment reference count?"
        );

        let tmp = unsafe { self.ffi_cast::<engine::RefCounted>() };
        let mut tmp = tmp.expect("object expected to inherit RefCounted");
        let return_val =
            <engine::RefCounted as GodotClass>::Declarer::scoped_mut(&mut tmp, |obj| apply(obj));

        std::mem::forget(tmp); // no ownership transfer
        return_val
    }

    pub(crate) fn as_object<R>(&self, apply: impl Fn(&mut engine::Object) -> R) -> R {
        // Note: no validity check; this could be called by to_string(), which can be called on dead instances

        let tmp = unsafe { self.ffi_cast::<engine::Object>() };
        let mut tmp = tmp.expect("object expected to inherit Object; should never fail");
        // let return_val = apply(tmp.inner_mut());
        let return_val =
            <engine::Object as GodotClass>::Declarer::scoped_mut(&mut tmp, |obj| apply(obj));

        std::mem::forget(tmp); // no ownership transfer
        return_val
    }

    // Conversions from/to Godot C++ `Object*` pointers
    ffi_methods! {
        type sys::GDExtensionObjectPtr = Opaque;

        fn from_obj_sys_weak = from_sys;
        fn obj_sys = sys;
    }

    /// Initializes this `Gd<T>` from the object pointer as a **strong ref**, meaning
    /// it initializes/increments the reference counter and keeps the object alive.
    ///
    /// This is the default for most initializations from FFI. In cases where reference counter
    /// should explicitly **not** be updated, [`Self::from_obj_sys_weak`] is available.
    #[doc(hidden)]
    pub unsafe fn from_obj_sys(ptr: sys::GDExtensionObjectPtr) -> Self {
        // Initialize reference counter, if needed
        Self::from_obj_sys_weak(ptr).with_inc_refcount()
    }

    /// Returns `self` but with initialized ref-count.
    fn with_inc_refcount(self) -> Self {
        // Note: use init_ref and not inc_ref, since this might be the first reference increment.
        // Godot expects RefCounted::init_ref to be called instead of RefCounted::reference in that case.
        // init_ref also doesn't hurt (except 1 possibly unnecessary check).
        T::Mem::maybe_init_ref(&self);
        self
    }

    /// Returns a callable referencing a method from this object named `method_name`.
    pub fn callable<S: Into<StringName>>(&self, method_name: S) -> Callable {
        Callable::from_object_method(self.share(), method_name)
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
    pub fn free(self) {
        // TODO disallow for singletons, either only at runtime or both at compile time (new memory policy) and runtime

        // Runtime check in case of T=Object, no-op otherwise
        let ref_counted = T::Mem::is_ref_counted(&self);
        assert_ne!(
            ref_counted, Some(true),
            "called free() on Gd<Object> which points to a RefCounted dynamic type; free() only supported for manually managed types."
        );

        // If ref_counted returned None, that means the instance was destroyed
        assert!(
            ref_counted == Some(false) && self.is_instance_valid(),
            "called free() on already destroyed object"
        );

        // This destroys the Storage instance, no need to run destructor again
        unsafe {
            interface_fn!(object_destroy)(self.obj_sys());
        }

        std::mem::forget(self);
    }
}

impl<T> Deref for Gd<T>
where
    T: GodotClass<Declarer = dom::EngineDomain>,
{
    type Target = T;

    fn deref(&self) -> &Self::Target {
        // SAFETY:
        // This relies on Gd<Node3D> having the layout as Node3D (as an example),
        // which also needs #[repr(transparent)]:
        //
        // struct Gd<T: GodotClass> {
        //     opaque: OpaqueObject,         <- size of GDExtensionObjectPtr
        //     _marker: PhantomData,         <- ZST
        // }
        // struct Node3D {
        //     object_ptr: sys::GDExtensionObjectPtr,
        // }
        unsafe { std::mem::transmute::<&OpaqueObject, &T>(&self.opaque) }
    }
}

impl<T> DerefMut for Gd<T>
where
    T: GodotClass<Declarer = dom::EngineDomain>,
{
    fn deref_mut(&mut self) -> &mut T {
        // SAFETY: see also Deref
        //
        // The resulting &mut T is transmuted from &mut OpaqueObject, i.e. a *pointer* to the `opaque` field.
        // `opaque` itself has a different *address* for each Gd instance, meaning that two simultaneous
        // DerefMut borrows on two Gd instances will not alias, *even if* the underlying Godot object is the
        // same (i.e. `opaque` has the same value, but not address).
        unsafe { std::mem::transmute::<&mut OpaqueObject, &mut T>(&mut self.opaque) }
    }
}
// SAFETY:
// - `move_return_ptr`
//   When the `call_type` is `PtrcallType::Virtual`, and the current type is known to inherit from `RefCounted`
//   then we use `ref_get_object`. Otherwise we use `Gd::from_obj_sys`.
// - `from_arg_ptr`
//   When the `call_type` is `PtrcallType::Virtual`, and the current type is known to inherit from `RefCounted`
//   then we use `ref_set_object`. Otherwise we use `std::ptr::write`. Finally we forget `self` as we pass
//   ownership to the caller.
unsafe impl<T> GodotFfi for Gd<T>
where
    T: GodotClass,
{
    ffi_methods! { type sys::GDExtensionTypePtr = Opaque;
        fn from_sys;
        fn from_sys_init;
        fn sys;
    }

    // For more context around `ref_get_object` and `ref_set_object`, see:
    // https://github.com/godotengine/godot-cpp/issues/954

    unsafe fn from_arg_ptr(ptr: sys::GDExtensionTypePtr, call_type: PtrcallType) -> Self {
        let obj_ptr = if T::Mem::pass_as_ref(call_type) {
            // ptr is `Ref<T>*`
            // See the docs for `PtrcallType::Virtual` for more info on `Ref<T>`.
            interface_fn!(ref_get_object)(ptr as sys::GDExtensionRefPtr)
        } else if cfg!(since_api = "4.1") || matches!(call_type, PtrcallType::Virtual) {
            // ptr is `T**`
            *(ptr as *mut sys::GDExtensionObjectPtr)
        } else {
            // ptr is `T*`
            ptr as sys::GDExtensionObjectPtr
        };

        // obj_ptr is `T*`
        Self::from_obj_sys(obj_ptr)
    }

    unsafe fn move_return_ptr(self, ptr: sys::GDExtensionTypePtr, call_type: PtrcallType) {
        if T::Mem::pass_as_ref(call_type) {
            interface_fn!(ref_set_object)(ptr as sys::GDExtensionRefPtr, self.obj_sys())
        } else {
            std::ptr::write(ptr as *mut _, self.opaque)
        }
        // We've passed ownership to caller.
        std::mem::forget(self);
    }

    fn as_arg_ptr(&self) -> sys::GDExtensionConstTypePtr {
        // We're passing a reference to the object to the callee. If the reference count needs to be
        // incremented then the callee will do so. We do not need to prematurely do so.
        //
        // In Rust terms, if `T` is refcounted then we are effectively passing a `&Arc<T>`, and the callee
        // would need to call `.clone()` if desired.

        // In 4.0, argument pointers are passed to godot as `T*`, except for in virtual method calls. We
        // can't perform virtual method calls currently, so they are always `T*`.
        //
        // In 4.1 argument pointers were standardized to always be `T**`.
        #[cfg(before_api = "4.1")]
        {
            self.sys_const()
        }

        #[cfg(since_api = "4.1")]
        {
            std::ptr::addr_of!(self.opaque) as sys::GDExtensionConstTypePtr
        }
    }
}

// SAFETY:
// `Gd<T: GodotClass>` will only contain types that inherit from `crate::engine::Object`.
// Godots `Object` in turn is known to be nullable and always a pointer.
unsafe impl<T: GodotClass> GodotNullablePtr for Gd<T> {}

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
        let object_ptr = raw_object_init(init_fn);

        // Do not increment ref-count; assumed to be return value from FFI.
        sys::ptr_then(object_ptr, |ptr| Gd::from_obj_sys_weak(ptr))
    }
}

/// Runs `init_fn` on the address of a pointer (initialized to null), then returns that pointer, possibly still null.
///
/// # Safety
/// `init_fn` must be a function that correctly handles a _type pointer_ pointing to an _object pointer_.
#[doc(hidden)]
pub unsafe fn raw_object_init(
    init_fn: impl FnOnce(sys::GDExtensionUninitializedTypePtr),
) -> sys::GDExtensionObjectPtr {
    // return_ptr has type GDExtensionTypePtr = GDExtensionObjectPtr* = OpaqueObject* = Object**
    // (in other words, the type-ptr contains the _address_ of an object-ptr).
    let mut object_ptr: sys::GDExtensionObjectPtr = ptr::null_mut();
    let return_ptr: *mut sys::GDExtensionObjectPtr = ptr::addr_of_mut!(object_ptr);

    init_fn(return_ptr as sys::GDExtensionUninitializedTypePtr);

    // We don't need to know if Object** is null, but if Object* is null; return_ptr has the address of a local (never null).
    object_ptr
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
        let is_last = T::Mem::maybe_dec_ref(self); // may drop
        if is_last {
            unsafe {
                interface_fn!(object_destroy)(self.obj_sys());
            }
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
        Self::from_opaque(self.opaque).with_inc_refcount()
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
                    T::CLASS_NAME
                )
            }
            _ => format!("{}:", VariantType::Object as i32),
        }
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
        let hint = if T::inherits::<engine::Resource>() {
            engine::global::PropertyHint::PROPERTY_HINT_RESOURCE_TYPE
        } else if T::inherits::<engine::Node>() {
            engine::global::PropertyHint::PROPERTY_HINT_NODE_TYPE
        } else {
            engine::global::PropertyHint::PROPERTY_HINT_NONE
        };

        // Godot does this by default too, it doesn't seem to make a difference when not a resource/node
        // but is needed when it is a resource/node.
        let hint_string = T::CLASS_NAME.into();

        ExportInfo { hint, hint_string }
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Trait impls

impl<T: GodotClass> FromVariant for Gd<T> {
    fn try_from_variant(variant: &Variant) -> Result<Self, VariantConversionError> {
        let result_or_none = unsafe {
            // TODO(#234) replace Gd::<Object> with Self when Godot stops allowing illegal conversions
            // See https://github.com/godot-rust/gdext/issues/158

            // TODO(uninit) - see if we can use from_sys_init()
            use ::godot_ffi::AsUninit;

            Gd::<engine::Object>::from_sys_init_opt(|self_ptr| {
                let converter = sys::builtin_fn!(object_from_variant);
                converter(self_ptr.as_uninit(), variant.var_sys());
            })
        };

        // The conversion method `variant_to_object` does NOT increment the reference-count of the object; we need to do that manually.
        // (This behaves differently in the opposite direction `object_to_variant`.)
        result_or_none
            .map(|obj| obj.with_inc_refcount())
            // TODO(#234) remove this cast when Godot stops allowing illegal conversions
            // (See https://github.com/godot-rust/gdext/issues/158)
            .and_then(|obj| obj.owned_cast().ok())
            .ok_or(VariantConversionError::BadType)
    }
}

impl<T: GodotClass> ToVariant for Gd<T> {
    fn to_variant(&self) -> Variant {
        // The conversion method `object_to_variant` DOES increment the reference-count of the object; so nothing to do here.
        // (This behaves differently in the opposite direction `variant_to_object`.)

        unsafe {
            Variant::from_var_sys_init(|variant_ptr| {
                let converter = sys::builtin_fn!(object_to_variant);

                // Note: this is a special case because of an inconsistency in Godot, where sometimes the equivalency is
                // GDExtensionTypePtr == Object** and sometimes GDExtensionTypePtr == Object*. Here, it is the former, thus extra pointer.
                // Reported at https://github.com/godotengine/godot/issues/61967
                let type_ptr = self.sys();
                converter(
                    variant_ptr,
                    ptr::addr_of!(type_ptr) as sys::GDExtensionTypePtr,
                );
            })
        }
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
        engine::display_string(self, f)
    }
}

impl<T: GodotClass> Debug for Gd<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        engine::debug_string(self, f, "Gd")
    }
}

impl<T: GodotClass> VariantMetadata for Gd<T> {
    fn variant_type() -> VariantType {
        VariantType::Object
    }

    fn class_name() -> ClassName {
        ClassName::of::<T>()
    }
}

// Gd unwinding across panics does not invalidate any invariants;
// its mutability is anyway present, in the Godot engine.
impl<T: GodotClass> std::panic::UnwindSafe for Gd<T> {}
impl<T: GodotClass> std::panic::RefUnwindSafe for Gd<T> {}
