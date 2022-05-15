use crate::{sys, Obj};
use gdext_builtin::GodotString;
use std::fmt::Debug;

pub trait EngineClass: GodotClass {
    fn from_object_ptr(object_ptr: sys::GDNativeObjectPtr) -> Self;
    fn as_object_ptr(&self) -> sys::GDNativeObjectPtr;
    fn as_type_ptr(&self) -> sys::GDNativeTypePtr;

    fn from_obj(obj: &Obj<Self>) -> &Self {
        Self::from_object_ptr(obj.obj_sys())
    }
}

pub trait GodotClass: Debug
where
    Self: Sized,
{
    type Base: GodotClass;
    //type ClassType: marker::ClassType;

    fn class_name() -> String;
    fn from_obj(obj: &Obj<Self>) -> &Self {
        obj.storage().get()
    }

    // fn native_object_ptr(&self) -> sys::GDNativeObjectPtr {
    //     self.upcast().native_object_ptr()
    // }
    //fn upcast(&self) -> &Self::Base;
    //fn upcast_mut(&mut self) -> &mut Self::Base;
}

impl GodotClass for () {
    type Base = ();
    //type ClassType = marker::TagEngineClass;

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
