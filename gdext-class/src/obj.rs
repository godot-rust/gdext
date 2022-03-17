use crate::property_info::PropertyInfoBuilder;
use crate::GodotExtensionClass;
use gdext_builtin::variant::Variant;
use gdext_builtin::PtrCallArg;
use std::marker::PhantomData;

pub struct Obj<T: GodotExtensionClass> {
    data: T,
    _internal: PhantomData<*const T>,
}

impl<T: GodotExtensionClass> Obj<T> {
    // explicit deref for testing purposes
    pub fn inner(&self) -> &T {
        &self.data
    }
}

impl<T: GodotExtensionClass> From<&Variant> for Obj<T> {
    fn from(var: &Variant) -> Self {
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
