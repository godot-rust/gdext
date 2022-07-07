use std::marker::PhantomData;
use std::ptr;

use gdext_builtin::Variant;
use gdext_sys as sys;
use sys::types::OpaqueObject;
use sys::{ffi_methods, interface_fn, static_assert_eq_size, GodotFfi};

use crate::dom::Domain;
use crate::mem::Memory;
use crate::property_info::PropertyInfoBuilder;
use crate::storage::InstanceStorage;
use crate::{api, dom, out, ClassName, GodotClass, GodotDefault, Inherits, InstanceId, Share};

// TODO which bounds to add on struct itself?
//#[repr(transparent)] // needed for safe transmute between object and a field, see EngineClass
// FIXME repr-transparent
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

/// The methods in this impl block are available for any `T`.
impl<T: GodotClass> Obj<T> {
    pub fn try_from_instance_id(instance_id: InstanceId) -> Option<Self> {
        unsafe {
            let ptr = interface_fn!(object_get_instance_from_id)(instance_id.to_u64());

            if ptr.is_null() {
                None
            } else {
                Some(Obj::from_obj_sys(ptr))
            }
        }
    }

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

    pub fn instance_id(&self) -> InstanceId {
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

    /// Upcast: onvert into a smart pointer to a base class. Always succeeds.
    pub fn upcast<Base>(self) -> Obj<Base>
    where
        Base: GodotClass,
        T: Inherits<Base>,
    {
        self.owned_cast()
            .expect("Upcast failed. This is a bug; please report it.")
    }

    /// Downcast: try to convert into a smart pointer to a derived class.
    ///
    /// If `T`'s dynamic type is not `Derived` or one of its subclasses, `None` is returned
    /// and the reference is dropped. Otherwise, `Some` is returned and the ownership is moved
    /// to the returned value.
    pub fn try_cast<Derived>(self) -> Option<Obj<Derived>>
    where
        Derived: GodotClass + Inherits<T>,
    {
        self.owned_cast()
    }

    /// Downcast: convert into a smart pointer to a derived class. Always succeeds.
    ///
    /// # Panics
    /// If the class' dynamic type is not `Derived` or one of its subclasses. Use [`Self::try_cast()`] if you want to check the result.
    pub fn cast<Derived>(self) -> Obj<Derived>
    where
        Derived: GodotClass + Inherits<T>,
    {
        self.owned_cast().unwrap_or_else(|| {
            panic!(
                "Downcast from {from} to {to} failed; correct the code or use try_cast().",
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
        let tmp = unsafe { self.ffi_cast::<api::RefCounted>() };
        let mut tmp = tmp.expect("object expected to inherit RefCounted");
        let return_val = apply(tmp.inner_mut());
        std::mem::forget(tmp); // no ownership transfer
        return_val
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

/// The methods in this impl block are only available for user-declared `T`, that is,
/// structs with `#[derive(GodotClass)]` but not Godot classes like `Node` or `RefCounted`.
impl<T> Obj<T>
where
    T: GodotClass<Declarer = dom::UserDomain>,
{
    pub fn new_default() -> Self
    where
        T: GodotDefault,
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

    /// Storage object associated with the extension instance
    pub(crate) fn storage(&self) -> &mut InstanceStorage<T> {
        let callbacks = crate::storage::nop_instance_callbacks();

        unsafe {
            let token = sys::get_library();
            let binding =
                interface_fn!(object_get_instance_binding)(self.obj_sys(), token, &callbacks);
            crate::private::as_storage::<T>(binding)
        }
    }
}

impl<T: GodotClass> GodotFfi for Obj<T> {
    ffi_methods! { type sys::GDNativeTypePtr = Opaque; .. }
}

impl<T: GodotClass> Share for Obj<T> {
    fn share(&self) -> Self {
        out!("Obj::share");
        Self::from_opaque(self.opaque).ready()
    }
}

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
