use crate::property_info::PropertyInfoBuilder;
use crate::{sys, sys::interface_fn, GodotExtensionClass};
use gdext_builtin::variant::Variant;
use gdext_builtin::PtrCallArg;
use gdext_sys::GDNativeObjectPtr;
use once_cell::sync::Lazy;
use std::marker::PhantomData;

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
    fn from(v: &Variant) -> Self {
        unsafe {
            static CONSTR: Lazy<
                unsafe extern "C" fn(sys::GDNativeTypePtr, sys::GDNativeVariantPtr),
            > = Lazy::new(|| unsafe {
                interface_fn!(get_variant_to_type_constructor)(
                    sys::GDNativeVariantType_GDNATIVE_VARIANT_TYPE_OBJECT,
                )
                .unwrap()
            });

            let mut opaque = std::mem::MaybeUninit::<GDNativeObjectPtr>::uninit();
            CONSTR(opaque.as_mut_ptr() as *mut _, v.as_ptr());
            Obj::from_sys(opaque.assume_init())
        }
    }
}

impl<T: GodotExtensionClass> From<Obj<T>> for Variant {
    fn from(obj: Obj<T>) -> Self {
        unsafe {
            static CONSTR: Lazy<
                unsafe extern "C" fn(sys::GDNativeVariantPtr, sys::GDNativeTypePtr),
            > = Lazy::new(|| unsafe {
                interface_fn!(get_variant_from_type_constructor)(
                    sys::GDNativeVariantType_GDNATIVE_VARIANT_TYPE_OBJECT,
                )
                .unwrap()
            });
            let mut v = Variant::uninit();

            println!("Convert to variant: {:?}", obj.opaque_ptr);
            CONSTR(v.as_mut_ptr(), &obj.opaque_ptr as *const _ as *mut _);

            v
        }
    }
}
impl<T: GodotExtensionClass> From<&Obj<T>> for Variant {
    fn from(obj: &Obj<T>) -> Self {
        todo!()
    }
}

impl<T: GodotExtensionClass> PtrCallArg for Obj<T> {
    unsafe fn from_ptr_call_arg(arg: *const gdext_sys::GDNativeTypePtr) -> Self {
        todo!()
    }

    unsafe fn to_ptr_call_arg(self, arg: gdext_sys::GDNativeTypePtr) {
        todo!()
    }
}
impl<T: GodotExtensionClass> PropertyInfoBuilder for Obj<T> {
    fn variant_type() -> gdext_sys::GDNativeVariantType {
        gdext_sys::GDNativeVariantType_GDNATIVE_VARIANT_TYPE_NIL
    }
}
