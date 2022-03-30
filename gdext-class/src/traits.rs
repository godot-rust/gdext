use crate::sys;
use gdext_builtin::string::GodotString;

pub trait GodotClass {
    type Base: GodotClass;

    fn class_name() -> String;

    // fn native_object_ptr(&self) -> sys::GDNativeObjectPtr {
    //     self.upcast().native_object_ptr()
    // }
    //fn upcast(&self) -> &Self::Base;
    //fn upcast_mut(&mut self) -> &mut Self::Base;
}

pub trait GodotExtensionClass: GodotClass {
    //fn construct(base: sys::GDNativeObjectPtr) -> Self;

    fn reference(&mut self) {}
    fn unreference(&mut self) {}
    fn has_to_string() -> bool {
        false
    }
}

pub trait GodotExtensionClassMethods {
    fn virtual_call(name: &str) -> sys::GDNativeExtensionClassCallVirtual;
    fn register_methods();
    fn to_string(&self) -> GodotString {
        GodotString::new()
    }
}
