use crate::property_info::PropertyInfoBuilder;
use crate::{sys, GodotExtensionClass};
use gdext_builtin::variant::Variant;
use gdext_builtin::PtrCallArg;
use gdext_sys::interface_fn;
use once_cell::sync::Lazy;
use std::marker::PhantomData;

pub struct Obj<T: GodotExtensionClass> {
    opaque_ptr: *mut std::ffi::c_void,
    _internal: PhantomData<*const T>,
}

impl<T: GodotExtensionClass> Obj<T> {
    pub fn from_sys(ptr: gdext_sys::GDNativeObjectPtr) -> Self {
        Self {
            opaque_ptr: ptr,
            _internal: PhantomData,
        }
    }

    // explicit deref for testing purposes
    pub fn inner(&self) -> &T {
        todo!()
    }
}

impl<T: GodotExtensionClass> From<&Variant> for Obj<T> {
    fn from(var: &Variant) -> Self {
        todo!()
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
            //CONSTR(v.as_mut_ptr(), &obj as *const _ as *mut _);

            println!("Convert to variant: {:?}", obj.opaque_ptr);

            // CONSTR(v.as_mut_ptr(), &obj.opaque_ptr as *mut _);
            CONSTR(v.as_mut_ptr(), &obj.opaque_ptr as *const _ as *mut _);

            //todo!("variant not yet impl");
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
