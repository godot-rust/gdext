use crate::sys;
use gdext_builtin::GodotString;
use std::fmt::Debug;

pub trait EngineClass {
    fn as_object_ptr(&self) -> sys::GDNativeObjectPtr;
}

pub trait GodotClass: Debug {
    const ENGINE_CLASS: bool = false;
    type Base: GodotClass;

    fn class_name() -> String;

    // fn native_object_ptr(&self) -> sys::GDNativeObjectPtr {
    //     self.upcast().native_object_ptr()
    // }
    //fn upcast(&self) -> &Self::Base;
    //fn upcast_mut(&mut self) -> &mut Self::Base;
}

impl GodotClass for () {
    type Base = ();

    fn class_name() -> String {
        "(no base)".to_string()
    }
}

pub trait GodotMethods: GodotClass {
    //fn construct(base: Obj<Self::Base>) -> Self;
    fn construct(base: sys::GDNativeObjectPtr) -> Self;
}

pub trait GodotExtensionClass: GodotClass {
    //fn construct(base: sys::GDNativeObjectPtr) -> Self;

    fn reference(&mut self) {}
    fn unreference(&mut self) {}
    fn has_to_string() -> bool {
        false
    }
}

pub trait GodotExtensionClassMethods: GodotClass {
    fn virtual_call(name: &str) -> sys::GDNativeExtensionClassCallVirtual;
    fn register_methods();
    fn to_string(&self) -> GodotString {
        GodotString::new()
    }
}
