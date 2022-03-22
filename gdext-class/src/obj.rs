use crate::property_info::PropertyInfoBuilder;
use crate::{sys, sys::interface_fn, GodotExtensionClass};
use gdext_builtin::variant::Variant;
use gdext_builtin::PtrCallArg;
use std::marker::PhantomData;
use std::mem::MaybeUninit;

pub struct Obj<T: GodotExtensionClass> {
    // Note: this may not be a pointer behind the scenes -- consider using an opaque [u8; SIZE_FROM_JSON]
    opaque_ptr: *mut std::ffi::c_void, // this is a sys::GDNativeObjectPtr
    _marker: PhantomData<*const T>,
}

impl<T: GodotExtensionClass> Obj<T> {
    pub fn from_sys(ptr: sys::GDNativeObjectPtr) -> Self {
        Self {
            opaque_ptr: ptr,
            _marker: PhantomData,
        }
    }

    // explicit deref for testing purposes
    pub fn inner(&self) -> &T {
        todo!()
    }

    pub fn instance_id(&self) -> u64 {
        unsafe { interface_fn!(object_get_instance_id)(self.opaque_ptr) }
    }

    pub fn from_instance_id(instance_id: u64) -> Self {
        unsafe {
            let ptr = interface_fn!(object_get_instance_from_id)(instance_id);
            Obj::from_sys(ptr)
        }
    }
}

impl<T: GodotExtensionClass> From<&Variant> for Obj<T> {
    fn from(variant: &Variant) -> Self {
        unsafe {
            let converter = sys::get_cache().variant_to_object;

            let mut opaque = MaybeUninit::<sys::GDNativeObjectPtr>::uninit();
            converter(opaque.as_mut_ptr() as *mut _, variant.as_ptr());

            Obj::from_sys(opaque.assume_init())
        }
    }
}

impl<T: GodotExtensionClass> From<Obj<T>> for Variant {
    fn from(obj: Obj<T>) -> Self {
        let opaque = unsafe {
            let converter = sys::get_cache().variant_from_object;
            let mut raw = MaybeUninit::<sys::types::OpaqueVariant>::uninit();
            converter(
                raw.as_mut_ptr() as sys::GDNativeVariantPtr,
                &obj.opaque_ptr as *const _ as *mut _,
            );
            raw.assume_init()
        };

        Self::from_sys(opaque)
    }
}
impl<T: GodotExtensionClass> From<&Obj<T>> for Variant {
    fn from(_obj: &Obj<T>) -> Self {
        todo!()
    }
}

impl<T: GodotExtensionClass> PtrCallArg for Obj<T> {
    unsafe fn from_ptr_call_arg(_arg: *const gdext_sys::GDNativeTypePtr) -> Self {
        todo!()
    }

    unsafe fn to_ptr_call_arg(self, _arg: gdext_sys::GDNativeTypePtr) {
        todo!()
    }
}
impl<T: GodotExtensionClass> PropertyInfoBuilder for Obj<T> {
    fn variant_type() -> gdext_sys::GDNativeVariantType {
        gdext_sys::GDNativeVariantType_GDNATIVE_VARIANT_TYPE_NIL
    }
}
