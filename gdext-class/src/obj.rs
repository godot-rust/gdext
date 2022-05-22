use crate::property_info::PropertyInfoBuilder;
use crate::storage::InstanceStorage;
use crate::{ClassName, GodotClass};

use gdext_builtin::Variant;
use gdext_sys as sys;

use sys::types::OpaqueObject;
use sys::{impl_ffi_as_opaque_pointer, interface_fn, static_assert_eq_size, GodotFfi};

use std::marker::PhantomData;

// TODO which bounds to add on struct itself?
#[repr(transparent)] // needed for safe transmute between object and a field, see EngineClass
pub struct Obj<T: GodotClass> {
    // Note: `opaque` has the same layout as GDNativeObjectPtr == Object* in C++, i.e. the bytes represent a pointer
    // To receive a GDNativeTypePtr == GDNativeObjectPtr* == Object**, we need to get the address of this
    // Hence separate sys() for GDNativeTypePtr, and obj_sys() for GDNativeObjectPtr.
    // The former is the standard FFI type, while the latter is used in object-specific GDExtension APIs.
    opaque: OpaqueObject,
    _marker: PhantomData<*const T>,
}

// Size equality check (should additionally be covered by mem::transmute())
static_assert_eq_size!(
    sys::GDNativeObjectPtr,
    sys::types::OpaqueObject,
    "Godot FFI: pointer type `Object*` should have size advertised in JSON extension file"
);

impl<T: GodotClass> Obj<T> {
    pub fn new(_rust_obj: T) -> Self {
        let class_name = ClassName::new::<T>();
        let ptr = unsafe { interface_fn!(classdb_construct_object)(class_name.c_str()) };

        unsafe { Obj::from_obj_sys(ptr) }
    }

    fn from_opaque(opaque: OpaqueObject) -> Self {
        Self {
            opaque,
            _marker: PhantomData,
        }
    }

    // explicit deref for testing purposes
    pub fn inner(&self) -> &T {
        use crate::marker::ClassDeclarer as _;
        T::Declarer::extract_from_obj(self)
    }

    pub fn inner_mut(&self) -> &mut T {
        // TODO
        self.storage().get_mut()
    }

    pub fn instance_id(&self) -> u64 {
        // Note: bit 'id & (1 << 63)' determines if the instance is ref-counted
        unsafe { interface_fn!(object_get_instance_id)(self.obj_sys()) }
    }

    pub fn from_instance_id(instance_id: u64) -> Option<Self> {
        unsafe {
            let ptr = interface_fn!(object_get_instance_from_id)(instance_id);

            if ptr.is_null() {
                None
            } else {
                Some(Obj::from_obj_sys(ptr))
            }
        }
    }

    pub(crate) fn storage(&self) -> &mut InstanceStorage<T> {
        let callbacks = crate::storage::nop_instance_callbacks();

        unsafe {
            let token = sys::get_library();
            let binding =
                interface_fn!(object_get_instance_binding)(self.obj_sys(), token, &callbacks);
            crate::private::as_storage::<T>(binding)
        }
    }

    /// Returns FFI pointer for contexts where C++ expects `Object*`
    /// This is different from `sys()` which returns sys::GDNativeTypePtr, a `void*` pointing to different types depending on context
    #[doc(hidden)]
    pub fn obj_sys(&self) -> sys::GDNativeObjectPtr {
        unsafe { std::mem::transmute::<OpaqueObject, sys::GDNativeObjectPtr>(self.opaque) }
    }

    /// Construct from FFI pointer where C++ returns `Object*`
    #[doc(hidden)]
    pub unsafe fn from_obj_sys(object_ptr: sys::GDNativeObjectPtr) -> Self {
        let r = std::mem::transmute::<sys::GDNativeObjectPtr, OpaqueObject>(object_ptr);
        Self::from_opaque(r)
    }
}

/*
// TODO enable once ownership is clear -- see also forget() in ptrcall_write()
impl<T: GodotClass> Drop for Obj<T>{
    fn drop(&mut self) {
        println!("Obj::drop()");
        unsafe { interface_fn!(object_destroy)(self.sys_mut()); }
    }
}
*/

impl<T: GodotClass> GodotFfi for Obj<T> {
    //impl_ffi_as_opaque_inplace_pointer!(sys::GDNativeObjectPtr);
    impl_ffi_as_opaque_pointer!(sys::GDNativeTypePtr);
}

impl<T: GodotClass> From<&Variant> for Obj<T> {
    fn from(variant: &Variant) -> Self {
        println!("!!TODO!! Variant to Obj<T>");
        unsafe {
            Self::from_sys_init(|type_ptr| {
                let converter = sys::get_cache().object_from_variant;
                converter(type_ptr, variant.sys());
            })
        }
    }
}

impl<T: GodotClass> From<Obj<T>> for Variant {
    fn from(obj: Obj<T>) -> Self {
        println!("!!TODO!! Variant from Obj<T>");
        unsafe {
            Self::from_sys_init(|variant_ptr| {
                let converter = sys::get_cache().object_to_variant;
                converter(variant_ptr, obj.sys());
            })
        }
    }
}

impl<T: GodotClass> From<&Obj<T>> for Variant {
    fn from(_obj: &Obj<T>) -> Self {
        todo!()
    }
}

impl<T: GodotClass> PropertyInfoBuilder for Obj<T> {
    fn variant_type() -> gdext_sys::GDNativeVariantType {
        gdext_sys::GDNativeVariantType_GDNATIVE_VARIANT_TYPE_OBJECT
    }
}
