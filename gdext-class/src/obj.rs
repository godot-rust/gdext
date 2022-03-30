use crate::property_info::PropertyInfoBuilder;
use crate::{sys, ClassName, GodotClass};
use gdext_builtin::godot_ffi::GodotFfi;
use gdext_builtin::impl_ffi_as_pointer;
use gdext_builtin::variant::Variant;
use gdext_sys::interface_fn;
use gdext_sys::types::OpaqueObject;

use std::marker::PhantomData;

// TODO which bounds to add on struct itself?
pub struct Obj<T: GodotClass> {
    opaque: OpaqueObject,
    _marker: PhantomData<*const T>,
}

impl<T: GodotClass> Obj<T> {
    pub fn new(_rust_obj: T) -> Self {
        let class_name = ClassName::new::<T>();

        let ptr = unsafe { interface_fn!(classdb_construct_object)(class_name.c_str()) };

        unsafe { Obj::from_sys(ptr) }
    }

    fn from_opaque(opaque: OpaqueObject) -> Self {
        print!("Obj::from_opaque: opaque={}", opaque);

        let s = Self {
            opaque,
            _marker: PhantomData,
        };

        println!(", self.opaque={}", s.opaque);
        s
    }

    // explicit deref for testing purposes
    pub fn inner(&self) -> &T {
        let callbacks = sys::GDNativeInstanceBindingCallbacks {
            create_callback: None,
            free_callback: None,
            reference_callback: None,
        };

        let binding = unsafe {
            let token = sys::get_library();
            interface_fn!(object_get_instance_binding)(self.sys(), token, &callbacks)
        };

        unsafe {
            let storage = crate::private::as_storage::<T>(binding);
            storage.get()
        }
    }

    pub fn instance_id(&self) -> u64 {
        // Note: bit 'id & (1 << 63)' determines if the instance is ref-counted
        unsafe { interface_fn!(object_get_instance_id)(self.sys()) }
    }

    pub fn from_instance_id(instance_id: u64) -> Option<Self> {
        unsafe {
            let ptr = interface_fn!(object_get_instance_from_id)(instance_id);

            if ptr.is_null() {
                None
            } else {
                Some(Obj::from_sys(ptr))
            }
        }
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
    impl_ffi_as_pointer!();
}

impl<T: GodotClass> From<&Variant> for Obj<T> {
    fn from(variant: &Variant) -> Self {
        unsafe {
            let opaque = OpaqueObject::with_init(|ptr| {
                let converter = sys::get_cache().object_from_variant;
                converter(ptr, variant.sys());
            });

            Obj::from_opaque(opaque)
        }
    }
}

impl<T: GodotClass> From<Obj<T>> for Variant {
    fn from(obj: Obj<T>) -> Self {
        unsafe {
            Self::from_sys_init(|opaque_ptr| {
                let converter = sys::get_cache().object_to_variant;
                converter(opaque_ptr, obj.opaque.to_sys());
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
