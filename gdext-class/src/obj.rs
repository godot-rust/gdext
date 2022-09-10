use std::marker::PhantomData;
use std::ptr;

use gdext_builtin::Variant;
use gdext_sys as sys;
use sys::types::OpaqueObject;
use sys::{ffi_methods, interface_fn, static_assert_eq_size, GodotFfi};

use crate::dom::Domain as _;
use crate::mem::Memory as _;
use crate::property_info::PropertyInfoBuilder;
use crate::storage::InstanceStorage;
use crate::{api, cap, dom, mem, out, ClassName, GodotClass, Inherits, InstanceId, Share};

/// Smart pointer to objects owned by the Godot engine.
///
/// This smart pointer can only hold _objects_ in the Godot sense: instances of Godot classes (`Node`, `RefCounted`, etc.)
/// or user-declared structs (`#[derive(GodotClass)]`). It does **not** hold built-in types (`Vector3`, `Color`, `i32`).
///
/// This smart pointer behaves differently depending on `T`'s associated types, see [`GodotClass`] for their documentation.
/// In particular, the memory management strategy is fully dependent on `T`:
///
/// * Objects of type `RefCounted` or inherited from it are **reference-counted**. This means that every time a smart pointer is
///   shared using [`Share::share()`], the reference counter is incremented, and every time one is dropped, it is decremented.
///   This ensures that the last reference (either in Rust or Godot) will deallocate the object and call `T`'s destructor.
///
/// * Objects inheriting from `Object` which are not `RefCounted` (or inherited) are **manually-managed**.
///   Their destructor is not automatically called (unless they are part of the scene tree). Creating an `Obj<T>` means that
///   you are responsible of explicitly deallocating such objects using [`Obj::free()`].
///
/// * For `T=Object`, the memory strategy is determined **dynamically**. Due to polymorphism, an `Obj<T>` can point to either
///   reference-counted or manually-managed types at runtime. The behavior corresponds to one of the two previous points.
///   Note that if the dynamic type is also `Object`, the memory is manually-managed.
pub struct Obj<T: GodotClass> {
    // Note: `opaque` has the same layout as GDNativeObjectPtr == Object* in C++, i.e. the bytes represent a pointer
    // To receive a GDNativeTypePtr == GDNativeObjectPtr* == Object**, we need to get the address of this
    // Hence separate sys() for GDNativeTypePtr, and obj_sys() for GDNativeObjectPtr.
    // The former is the standard FFI type, while the latter is used in object-specific GDExtension APIs.
    // pub(crate) because accessed in traits::dom
    pub(crate) opaque: OpaqueObject,
    _marker: PhantomData<*const T>,
}

// Size equality check (should additionally be covered by mem::transmute())
static_assert_eq_size!(
    sys::GDNativeObjectPtr,
    sys::types::OpaqueObject,
    "Godot FFI: pointer type `Object*` should have size advertised in JSON extension file"
);

/// The methods in this impl block are only available for user-declared `T`, that is,
/// structs with `#[derive(GodotClass)]` but not Godot classes like `Node` or `RefCounted`.
impl<T> Obj<T>
where
    T: GodotClass<Declarer = dom::UserDomain>,
{
    /// Moves a user-created object into this smart pointer, submitting ownership to the Godot engine.
    pub fn new(user_object: T) -> Self {
        let class_name = ClassName::new::<T>();
        let result = unsafe {
            let ptr = interface_fn!(classdb_construct_object)(class_name.c_str());
            Obj::from_obj_sys(ptr)
        };

        result.storage().initialize(user_object);
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
        let class_name = ClassName::new::<T>();
        let result = unsafe {
            let ptr = interface_fn!(classdb_construct_object)(class_name.c_str());
            Obj::from_obj_sys(ptr)
        };

        result.storage().initialize_default();
        T::Mem::maybe_init_ref(&result);
        result
    }

    /// Storage object associated with the extension instance
    pub(crate) fn storage(&self) -> &mut InstanceStorage<T> {
        let callbacks = crate::storage::nop_instance_callbacks();

        unsafe {
            let token = sys::get_library();
            let binding =
                interface_fn!(object_get_instance_binding)(self.obj_sys(), token, &callbacks);

            debug_assert!(!binding.is_null(), "null instance binding");
            crate::private::as_storage::<T>(binding)
        }
    }
}

/// The methods in this impl block are available for any `T`.
impl<T: GodotClass> Obj<T> {
    /// Looks up the given instance ID and returns the associated object, if possible.
    ///
    /// If no such instance ID is registered, or if the dynamic type of the object behind that instance ID
    /// is not compatible with `T`, then `None` is returned.
    pub fn try_from_instance_id(instance_id: InstanceId) -> Option<Self> {
        // FIXME: check dynamic type
        unsafe {
            let ptr = interface_fn!(object_get_instance_from_id)(instance_id.to_u64());

            if ptr.is_null() {
                None
            } else {
                Some(Obj::from_obj_sys(ptr).ready())
            }
        }
    }

    /// Looks up the given instance ID and returns the associated object.
    ///
    /// # Panics
    /// If no such instance ID is registered, or if the dynamic type of the object behind that instance ID
    /// is not compatible with `T`.
    pub fn from_instance_id(instance_id: InstanceId) -> Self {
        Self::try_from_instance_id(instance_id).expect(&format!(
            "Instance ID {} does not belong to a valid object of class '{}'",
            instance_id,
            T::class_name()
        ))
    }

    fn from_opaque(opaque: OpaqueObject) -> Self {
        Self {
            opaque,
            _marker: PhantomData,
        }
    }

    /// Returns the instance ID of this object.
    ///
    /// # Panics
    /// If this object is no longer alive (registered in Godot's object database).
    pub fn instance_id(&self) -> InstanceId {
        // FIXME panic when freed
        // Note: bit 'id & (1 << 63)' determines if the instance is ref-counted
        let id = unsafe { interface_fn!(object_get_instance_id)(self.obj_sys()) };
        InstanceId::from_u64(id)
    }

    // explicit deref for testing purposes
    pub fn inner(&self) -> &T {
        T::Declarer::extract_from_obj(self)
    }

    pub fn inner_mut(&mut self) -> &mut T {
        T::Declarer::extract_from_obj_mut(self)
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
    /// let obj: Obj<T> = ...;
    /// let base = obj.share().upcast::<Base>()
    /// ```
    pub fn upcast<Base>(self) -> Obj<Base>
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
    // TODO consider Result<Obj<Derived>, Self> so that user can still use original object (e.g. to free if manual)
    pub fn try_cast<Derived>(self) -> Option<Obj<Derived>>
    where
        Derived: GodotClass + Inherits<T>,
    {
        self.owned_cast()
    }

    /// **Downcast:** convert into a smart pointer to a derived class. Always succeeds.
    ///
    /// # Panics
    /// If the class' dynamic type is not `Derived` or one of its subclasses. Use [`Self::try_cast()`] if you want to check the result.
    pub fn cast<Derived>(self) -> Obj<Derived>
    where
        Derived: GodotClass + Inherits<T>,
    {
        self.owned_cast().unwrap_or_else(|| {
            panic!(
                "downcast from {from} to {to} failed; correct the code or use try_cast()",
                from = T::class_name(),
                to = Derived::class_name()
            )
        })
    }

    fn owned_cast<U>(self) -> Option<Obj<U>>
    where
        U: GodotClass,
    {
        // Transmuting unsafe { std::mem::transmute<&T, &Base>(self.inner()) } is probably not sound, since
        // C++ static_cast class casts *may* yield a different pointer (VTable offset, virtual inheritance etc.).
        // It *seems* to work at the moment (June 2022), but this is no indication it's not UB.
        // If this were sound, we could also provide an upcast on &Node etc. directly, as the resulting &Base could
        // point to the same instance (not allowed for &mut!). But the pointer needs to be stored somewhere, and
        // Obj<T> provides the storage -- &Node on its own doesn't have any.

        let result = unsafe { self.ffi_cast::<U>() };
        if result.is_some() {
            // duplicated ref, one must be wiped
            std::mem::forget(self);
        }

        result
    }

    // Note: does not transfer ownership and is thus unsafe. Also operates on shared ref.
    // Either the parameter or the return value *must* be forgotten (since reference counts are not updated).
    unsafe fn ffi_cast<U>(&self) -> Option<Obj<U>>
    where
        U: GodotClass,
    {
        let class_name = ClassName::new::<U>();
        let class_tag = interface_fn!(classdb_get_class_tag)(class_name.c_str());
        let cast_object_ptr = interface_fn!(object_cast_to)(self.obj_sys(), class_tag);

        if cast_object_ptr.is_null() {
            None
        } else {
            Some(Obj::from_obj_sys(cast_object_ptr))
        }
    }

    pub(crate) fn as_ref_counted<R>(&self, apply: impl Fn(&mut api::RefCounted) -> R) -> R {
        debug_assert!(
            self.is_valid(),
            "as_ref_counted() on freed instance; maybe forgot to increment reference count?"
        );

        let tmp = unsafe { self.ffi_cast::<api::RefCounted>() };
        let mut tmp = tmp.expect("object expected to inherit RefCounted");
        let return_val = apply(tmp.inner_mut());
        std::mem::forget(tmp); // no ownership transfer
        return_val
    }

    // pub(crate) fn as_object<R>(&self, apply: impl Fn(&mut api::Object) -> R) -> R {
    //     let tmp = unsafe { self.ffi_cast::<api::Object>() };
    //     let mut tmp = tmp.expect("object expected to inherit Object; should never fail");
    //     let return_val = apply(tmp.inner_mut());
    //     std::mem::forget(tmp); // no ownership transfer
    //     return_val
    // }

    fn is_valid(&self) -> bool {
        api::utilities::is_instance_id_valid(self.instance_id().to_i64())
    }

    // Conversions from/to Godot C++ `Object*` pointers
    ffi_methods! {
        type sys::GDNativeObjectPtr = Opaque;

        fn from_obj_sys = from_sys;
        fn from_obj_sys_init = from_sys_init;
        fn obj_sys = sys;
        fn write_obj_sys = write_sys;
    }
}

/// The methods in this impl block are only available for objects `T` that are manually managed,
/// i.e. anything that is not `RefCounted` or inherited from it.
impl<T, M> Obj<T>
where
    T: GodotClass<Mem = M>,
    M: mem::PossiblyManual + mem::Memory,
{
    /// Destroy the manually-managed Godot object.
    ///
    /// Consumes this smart pointer and renders all other `Obj` smart pointers (as well as any GDScript references) to the same object
    /// immediately invalid. Using those `Obj` instances will lead to panics, but not undefined behavior.
    ///
    /// This operation is **safe** and effectively prevents double-free.
    ///
    /// # Panics
    /// When the referred-to object has already been destroyed, or when this is invoked on an upcast `Obj<Object>`
    /// that dynamically points to a reference-counted type.
    pub fn free(self) {
        // Runtime check in case of T=Object, no-op otherwise
        assert!(
            !T::Mem::is_ref_counted(&self),
            "called free() on Obj<Object> which points to a RefCounted dynamic type; free() only supported for manually managed types."
        );

        //assert!(self.is_valid(), "called free() on already destroyed object");

        if !self.is_valid() {
            panic!("called free() on already destroyed object");
        }

        unsafe {
            interface_fn!(object_destroy)(self.obj_sys());
        }
    }
}

impl<T: GodotClass> GodotFfi for Obj<T> {
    ffi_methods! { type sys::GDNativeTypePtr = Opaque; .. }
}

/// Destructor with semantics depending on memory strategy.
///
/// * If this `Obj` smart pointer holds a reference-counted type, this will decrement the reference counter.
///   If this was the last remaining reference, dropping it will invoke `T`'s destructor.
///
/// * If the held object is manually-managed, **nothing happens**.
///   To destroy manually-managed `Obj` pointers, you need to call [`Self::free()`].
impl<T: GodotClass> Drop for Obj<T> {
    fn drop(&mut self) {
        out!("Obj::drop   <{}>", std::any::type_name::<T>());
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

impl<T: GodotClass> Share for Obj<T> {
    fn share(&self) -> Self {
        out!("Obj::share");
        Self::from_opaque(self.opaque).ready()
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Trait impls

impl<T: GodotClass> From<&Variant> for Obj<T> {
    fn from(variant: &Variant) -> Self {
        unsafe {
            let result = Self::from_sys_init(|self_ptr| {
                let converter = sys::method_table().object_from_variant;
                converter(self_ptr, variant.var_sys());
            });
            result.ready()
        }
    }
}

impl<T: GodotClass> From<Obj<T>> for Variant {
    fn from(obj: Obj<T>) -> Self {
        Variant::from(&obj)
        // drops original object here
    }
}

impl<T: GodotClass> From<&Obj<T>> for Variant {
    fn from(obj: &Obj<T>) -> Self {
        unsafe {
            Self::from_var_sys_init(|variant_ptr| {
                let converter = sys::method_table().object_to_variant;

                // Note: this is a special case because of an inconsistency in Godot, where sometimes the equivalency is
                // GDNativeTypePtr == Object** and sometimes GDNativeTypePtr == Object*. Here, it is the former, thus extra pointer.
                // Reported at https://github.com/godotengine/godot/issues/61967
                let type_ptr = obj.sys();
                converter(variant_ptr, ptr::addr_of!(type_ptr) as *mut _);
            })
        }
    }
}

impl<T: GodotClass> std::fmt::Debug for Obj<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Obj {{ instance_id: {} }}", self.instance_id())
    }
}

impl<T: GodotClass> PropertyInfoBuilder for Obj<T> {
    fn variant_type() -> sys::GDNativeVariantType {
        gdext_sys::GDNativeVariantType_GDNATIVE_VARIANT_TYPE_OBJECT
    }

    fn property_info(name: &str) -> sys::GDNativePropertyInfo {
        // Note: filling this information properly is important so that Godot can use ptrcalls instead of varcalls
        // (requires typed GDScript + sufficient information from the extension side)
        let reg = unsafe { sys::get_registry() };

        let property_name = reg.c_string(name);
        let class_name = reg.c_string(&T::class_name());

        sys::GDNativePropertyInfo {
            type_: Self::variant_type() as u32,
            name: property_name,
            class_name,
            hint: 0,
            hint_string: ptr::null_mut(),
            usage: 7, // Default, TODO generate global enums
        }
    }
}
