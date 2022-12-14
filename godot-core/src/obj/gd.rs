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
use sys::{ffi_methods, interface_fn, static_assert_eq_size, GodotFfi};

use crate::builtin::meta::{ClassName, PropertyInfo, VariantMetadata};
use crate::builtin::{FromVariant, StringName, ToVariant, Variant, VariantConversionError};
use crate::obj::dom::Domain as _;
use crate::obj::mem::Memory as _;
use crate::obj::{cap, dom, mem, GodotClass, Inherits, Share};
use crate::obj::{GdMut, GdRef, InstanceId};
use crate::storage::InstanceStorage;
use crate::{callbacks, engine, out};

/// Smart pointer to objects owned by the Godot engine.
///
/// This smart pointer can only hold _objects_ in the Godot sense: instances of Godot classes (`Node`, `RefCounted`, etc.)
/// or user-declared structs (`#[derive(GodotClass)]`). It does **not** hold built-in types (`Vector3`, `Color`, `i32`).
///
/// This smart pointer behaves differently depending on `T`'s associated types, see [`GodotClass`] for their documentation.
/// In particular, the memory management strategy is fully dependent on `T`:
///
/// * Objects of type [`RefCounted`] or inherited from it are **reference-counted**. This means that every time a smart pointer is
///   shared using [`Share::share()`], the reference counter is incremented, and every time one is dropped, it is decremented.
///   This ensures that the last reference (either in Rust or Godot) will deallocate the obj and call `T`'s destructor.
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
    // The former is the standard FFI type, while the latter is used in obj-specific GDExtension engines.
    // pub(crate) because accessed in obj::dom
    pub(crate) opaque: OpaqueObject,
    _marker: PhantomData<*const T>,
}

// Size equality check (should additionally be covered by mem::transmute())
static_assert_eq_size!(
    sys::GDExtensionObjectPtr,
    sys::types::OpaqueObject,
    "Godot FFI: pointer type `Object*` should have size advertised in JSON extension file"
);

/// _The methods in this impl block are only available for user-declared `T`, that is,
/// structs with `#[derive(GodotClass)]` but not Godot classes like `Node` or `RefCounted`._
impl<T> Gd<T>
where
    T: GodotClass<Declarer = dom::UserDomain>,
{
    /// Moves a user-created obj into this smart pointer, submitting ownership to the Godot engine.
    ///
    /// This is only useful for types `T` which do not store their base objects (if they have a base,
    /// you cannot construct them standalone).
    pub fn new(user_object: T) -> Self {
        /*let result = unsafe {
            //let ptr = interface_fn!(classdb_construct_object)(class_name.c_str());
            let ptr = callbacks::create::<T>(ptr::null_mut());
            Obj::from_obj_sys(ptr)
        };

        result.storage().initialize(user_object);*/

        let object_ptr = callbacks::create_custom(move |_base| user_object);
        let result = unsafe { Gd::from_obj_sys(object_ptr) };

        T::Mem::maybe_init_ref(&result);
        result
    }

    /// Creates a default-constructed instance of `T` inside a smart pointer.
    ///
    /// This is equivalent to the GDScript expression `T.new()`.
    pub fn new_default() -> Self
    where
        T: cap::GodotInit,
    {
        /*let class_name = ClassName::new::<T>();
        let result = unsafe {
            let ptr = interface_fn!(classdb_construct_object)(class_name.c_str());
            Obj::from_obj_sys(ptr)
        };

        result.storage().initialize_default();
        T::Mem::maybe_init_ref(&result);
        result*/

        let result = unsafe {
            let object_ptr = callbacks::create::<T>(ptr::null_mut());
            Gd::from_obj_sys(object_ptr)
        };

        T::Mem::maybe_init_ref(&result);
        result
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

    /// Storage obj associated with the extension instance
    pub(crate) fn storage(&self) -> &mut InstanceStorage<T> {
        let callbacks = crate::storage::nop_instance_callbacks();

        unsafe {
            let token = sys::get_library();
            let binding =
                interface_fn!(object_get_instance_binding)(self.obj_sys(), token, &callbacks);

            debug_assert!(
                !binding.is_null(),
                "Class {} -- null instance; does the class have a Godot creator function?",
                std::any::type_name::<T>()
            );
            crate::private::as_storage::<T>(binding)
        }
    }
}

/// _The methods in this impl block are available for any `T`._
impl<T: GodotClass> Gd<T> {
    /// Looks up the given instance ID and returns the associated obj, if possible.
    ///
    /// If no such instance ID is registered, or if the dynamic type of the obj behind that instance ID
    /// is not compatible with `T`, then `None` is returned.
    pub fn try_from_instance_id(instance_id: InstanceId) -> Option<Self> {
        // SAFETY: Godot looks up ID in ObjectDB and returns null if not found
        let ptr = unsafe { interface_fn!(object_get_instance_from_id)(instance_id.to_u64()) };

        if ptr.is_null() {
            None
        } else {
            // SAFETY: assumes that the returned GDExtensionObjectPtr is convertible to Object* (i.e. C++ upcast doesn't modify the pointer)
            let untyped = unsafe { Gd::<engine::Object>::from_obj_sys(ptr).ready() };
            untyped.owned_cast::<T>().ok()
        }
    }

    /// Looks up the given instance ID and returns the associated obj.
    ///
    /// # Panics
    /// If no such instance ID is registered, or if the dynamic type of the obj behind that instance ID
    /// is not compatible with `T`.
    #[cfg(feature = "convenience")]
    pub fn from_instance_id(instance_id: InstanceId) -> Self {
        Self::try_from_instance_id(instance_id).unwrap_or_else(|| {
            panic!(
                "Instance ID {} does not belong to a valid obj of class '{}'",
                instance_id,
                T::CLASS_NAME
            )
        })
    }

    fn from_opaque(opaque: OpaqueObject) -> Self {
        Self {
            opaque,
            _marker: PhantomData,
        }
    }

    /// Returns the instance ID of this obj, or `None` if the obj is dead.
    ///
    pub fn instance_id_or_none(&self) -> Option<InstanceId> {
        // Note: bit 'id & (1 << 63)' determines if the instance is ref-counted
        let id = unsafe { interface_fn!(object_get_instance_id)(self.obj_sys()) };
        InstanceId::try_from_u64(id)
    }

    /// Returns the instance ID of this obj (panics when dead).
    ///
    /// # Panics
    /// If this obj is no longer alive (registered in Godot's obj database).
    #[cfg(feature = "convenience")]
    pub fn instance_id(&self) -> InstanceId {
        self.instance_id_or_none().unwrap_or_else(|| {
            panic!(
                "failed to call instance_id() on destroyed obj; \
                use instance_id_or_none() or keep your objects alive"
            )
        })
    }

    /// Checks if this smart pointer points to a live obj (read description!).
    ///
    /// Using this method is often indicative of bad design -- you should dispose of your pointers once an obj is
    /// destroyed. However, this method exists because GDScript offers it and there may be **rare** use cases.
    ///
    /// Do not use this method to check if you can safely access an obj. Accessing dead objects is generally safe
    /// and will panic in a defined manner. Encountering such panics is almost always a bug you should fix, and not a
    /// runtime condition to check against.
    pub fn is_instance_valid(&self) -> bool {
        // TODO Is this really necessary, or is Godot's instance_id() guaranteed to return 0 for destroyed objects?
        if let Some(id) = self.instance_id_or_none() {
            engine::utilities::is_instance_id_valid(id.to_i64())
        } else {
            false
        }
    }

    /// Needed to initialize ref count -- must be explicitly invoked.
    ///
    /// Could be made part of FFI methods, but there are some edge cases where this is not intended.
    pub(crate) fn ready(self) -> Self {
        T::Mem::maybe_inc_ref(&self);
        self
    }

    /// **Upcast:** convert into a smart pointer to a base class. Always succeeds.
    ///
    /// Moves out of this value. If you want to create _another_ smart pointer instance,
    /// use this idiom:
    /// ```ignore
    /// let obj: Gd<T> = ...;
    /// let base = obj.share().upcast::<Base>();
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
    // TODO consider Result<Gd<Derived>, Self> so that user can still use original obj (e.g. to free if manual)
    pub fn try_cast<Derived>(self) -> Option<Gd<Derived>>
    where
        Derived: GodotClass + Inherits<T>,
    {
        self.owned_cast().ok()
    }

    /// **Downcast:** convert into a smart pointer to a derived class. Panics on error.
    ///
    /// # Panics
    /// If the class' dynamic type is not `Derived` or one of its subclasses. Use [`Self::try_cast()`] if you want to check the result.
    #[cfg(feature = "convenience")]
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
        // The unsafe { std::mem::transmute<&T, &Base>(self.inner()) } relies on the C++ static_cast class casts
        // to return the same pointer, however in theory those may yield a different pointer (VTable offset,
        // virtual inheritance etc.). It *seems* to work so far, but this is no indication it's not UB.
        //
        // The Deref/DerefMut impls for T implement an "implicit upcast" on the obj (not Gd) level and
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
        let class_name = ClassName::new::<U>();
        let class_tag = interface_fn!(classdb_get_class_tag)(class_name.string_sys());
        let cast_object_ptr = interface_fn!(object_cast_to)(self.obj_sys(), class_tag);

        if cast_object_ptr.is_null() {
            None
        } else {
            Some(Gd::from_obj_sys(cast_object_ptr))
        }
    }

    pub(crate) fn as_ref_counted<R>(&self, apply: impl Fn(&mut engine::RefCounted) -> R) -> R {
        debug_assert!(
            self.is_instance_valid(),
            "as_ref_counted() on freed instance; maybe forgot to increment reference count?"
        );

        let tmp = unsafe { self.ffi_cast::<engine::RefCounted>() };
        let mut tmp = tmp.expect("obj expected to inherit RefCounted");
        let return_val =
            <engine::RefCounted as GodotClass>::Declarer::scoped_mut(&mut tmp, |obj| apply(obj));

        std::mem::forget(tmp); // no ownership transfer
        return_val
    }

    pub(crate) fn as_object<R>(&self, apply: impl Fn(&mut engine::Object) -> R) -> R {
        // Note: no validity check; this could be called by to_string(), which can be called on dead instances

        let tmp = unsafe { self.ffi_cast::<engine::Object>() };
        let mut tmp = tmp.expect("obj expected to inherit Object; should never fail");
        // let return_val = apply(tmp.inner_mut());
        let return_val =
            <engine::Object as GodotClass>::Declarer::scoped_mut(&mut tmp, |obj| apply(obj));

        std::mem::forget(tmp); // no ownership transfer
        return_val
    }

    // Conversions from/to Godot C++ `Object*` pointers
    ffi_methods! {
        type sys::GDExtensionObjectPtr = Opaque;

        fn from_obj_sys = from_sys;
        fn from_obj_sys_init = from_sys_init;
        fn obj_sys = sys;
        fn write_obj_sys = write_sys;
    }
}

/// _The methods in this impl block are only available for objects `T` that are manually managed,
/// i.e. anything that is not `RefCounted` or inherited from it._
impl<T, M> Gd<T>
where
    T: GodotClass<Mem = M>,
    M: mem::PossiblyManual + mem::Memory,
{
    /// Destroy the manually-managed Godot obj.
    ///
    /// Consumes this smart pointer and renders all other `Gd` smart pointers (as well as any GDScript references) to the same obj
    /// immediately invalid. Using those `Gd` instances will lead to panics, but not undefined behavior.
    ///
    /// This operation is **safe** and effectively prevents double-free.
    ///
    /// Not calling `free()` on manually-managed instances causes memory leaks, unless their ownership is delegated, for
    /// example to the node tree in case of nodes.
    ///
    /// # Panics
    /// * When the referred-to obj has already been destroyed.
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
            "called free() on already destroyed obj"
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
        // DerefMut borrows on two Gd instances will not alias, *even if* the underlying Godot obj is the
        // same (i.e. `opaque` has the same value, but not address).
        unsafe { std::mem::transmute::<&mut OpaqueObject, &mut T>(&mut self.opaque) }
    }
}

impl<T: GodotClass> GodotFfi for Gd<T> {
    ffi_methods! { type sys::GDExtensionTypePtr = Opaque; .. }
}

impl<T: GodotClass> Gd<T> {
    pub unsafe fn from_sys_init_opt(init_fn: impl FnOnce(sys::GDExtensionTypePtr)) -> Option<Self> {
        // Note: see _call_native_mb_ret_obj() in godot-cpp, which does things quite different (e.g. querying the instance binding).

        // return_ptr has type GDExtensionTypePtr = GDExtensionObjectPtr* = OpaqueObject* = Object**
        // (in other words, the type-ptr contains the _address_ of an object-ptr).
        let mut object_ptr: sys::GDExtensionObjectPtr = ptr::null_mut();
        let return_ptr: *mut sys::GDExtensionObjectPtr = ptr::addr_of_mut!(object_ptr);

        init_fn(return_ptr as sys::GDExtensionTypePtr);

        // We don't need to know if Object** is null, but if Object* is null; return_ptr has the address of a local (never null).
        if object_ptr.is_null() {
            None
        } else {
            let obj = Gd::from_obj_sys(object_ptr); // equivalent to Gd::from_sys(return_ptr)
            Some(obj)
        }
    }
}

/// Destructor with semantics depending on memory strategy.
///
/// * If this `Gd` smart pointer holds a reference-counted type, this will decrement the reference counter.
///   If this was the last remaining reference, dropping it will invoke `T`'s destructor.
///
/// * If the held obj is manually-managed, **nothing happens**.
///   To destroy manually-managed `Gd` pointers, you need to call [`Self::free()`].
impl<T: GodotClass> Drop for Gd<T> {
    fn drop(&mut self) {
        // No-op for manually managed objects

        out!("Gd::drop   <{}>", std::any::type_name::<T>());
        let is_last = T::Mem::maybe_dec_ref(&self); // may drop
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
        Self::from_opaque(self.opaque).ready()
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Trait impls

impl<T: GodotClass> FromVariant for Gd<T> {
    fn try_from_variant(variant: &Variant) -> Result<Self, VariantConversionError> {
        let result = unsafe {
            let result = Self::from_sys_init(|self_ptr| {
                let converter = sys::builtin_fn!(object_from_variant);
                converter(self_ptr, variant.var_sys());
            });
            result.ready()
        };

        Ok(result)
    }
}

impl<T: GodotClass> ToVariant for Gd<T> {
    fn to_variant(&self) -> Variant {
        let variant = unsafe {
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
        };

        variant
    }
}

impl<T> Display for Gd<T>
where
    T: GodotClass<Declarer = dom::EngineDomain>,
{
    // TODO support for user objects? should it return the engine repr, or a custom <T as Display>::fmt()?
    // If the latter, we would need to do something like impl<T> Display for Gd<T> where T: Display,
    // and thus implement it for each class separately (or blanket GodotClass/EngineClass/...).

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

    fn property_info(property_name: &str) -> PropertyInfo {
        PropertyInfo::new(
            Self::variant_type(),
            ClassName::new::<T>(),
            StringName::from(property_name),
        )
    }
}
