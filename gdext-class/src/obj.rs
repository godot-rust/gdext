use crate::property_info::PropertyInfoBuilder;
use crate::{sys, sys::interface_fn, GodotExtensionClass};
use gdext_builtin::godot_ffi::GodotFfi;
use gdext_builtin::variant::Variant;
use gdext_builtin::{impl_ffi_as_pointer, PtrCallArg};
use gdext_sys::types::OpaqueObject;
use std::marker::PhantomData;

// TODO which bounds to add on struct itself?
pub struct Obj<T> {
    // Note: this may not be a pointer behind the scenes -- consider using an opaque [u8; SIZE_FROM_JSON]
    opaque: OpaqueObject,
    _marker: PhantomData<*const T>,
}

impl<T: GodotExtensionClass> Obj<T> {
    pub fn new(_rust_obj: T) -> Self {
        todo!()
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
        todo!()
    }

    pub fn instance_id(&self) -> u64 {
        println!("Obj::instance_id: opaque={}", self.opaque);

        //unsafe { interface_fn!(object_get_instance_id)(self.opaque_ptr) }
        // unsafe { interface_fn!(object_get_instance_id)(transmute(self.opaque)) }
        //unsafe { interface_fn!(object_get_instance_id)(self.opaque.to_value_sys()) }
        unsafe { interface_fn!(object_get_instance_id)(std::mem::transmute(self.opaque)) }
    }

    /*pub fn from_instance_id(instance_id: u64) -> Self {
        unsafe {
            let ptr = interface_fn!(object_get_instance_from_id)(instance_id);
            Obj::from_sys(ptr)
        }
    }*/
}

impl<T: GodotExtensionClass> GodotFfi for Obj<T> {
    impl_ffi_as_pointer!();
}

impl<T: GodotExtensionClass> From<&Variant> for Obj<T> {
    fn from(variant: &Variant) -> Self {
        unsafe {
            let opaque = OpaqueObject::with_init(|ptr| {
                let converter = sys::get_cache().variant_to_object;
                converter(ptr, variant.sys());
            });

            Obj::from_opaque(opaque)
        }
    }
}

impl<T: GodotExtensionClass> From<Obj<T>> for Variant {
    fn from(obj: Obj<T>) -> Self {
        unsafe {
            Self::from_sys_init(|opaque_ptr| {
                let converter = sys::get_cache().variant_from_object;
                converter(opaque_ptr, obj.opaque.to_sys());
            })
        }
    }
}

impl<T: GodotExtensionClass> From<&Obj<T>> for Variant {
    fn from(_obj: &Obj<T>) -> Self {
        todo!()
    }
}

impl<T: GodotExtensionClass> PtrCallArg for Obj<T> {
    unsafe fn from_ptr_call_arg(_arg: *const gdext_sys::GDNativeTypePtr) -> Self {
        //Clone::clone(&*(arg as *mut Obj<T>))
        todo!()
    }

    unsafe fn to_ptr_call_arg(self, arg: gdext_sys::GDNativeTypePtr) {
        // arg: Object** in C++

        println!("to_ptr_call_arg: opaque={}", self.opaque);
        std::ptr::write(arg as *mut OpaqueObject, self.opaque);

        //todo!("not impl: ptr={:?}", _arg)
        //std::ptr::write(arg as *mut Obj<T>, self);
    }
}
impl<T: GodotExtensionClass> PropertyInfoBuilder for Obj<T> {
    fn variant_type() -> gdext_sys::GDNativeVariantType {
        gdext_sys::GDNativeVariantType_GDNATIVE_VARIANT_TYPE_OBJECT
    }
}
