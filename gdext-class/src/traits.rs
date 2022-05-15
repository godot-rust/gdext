use crate::{sys, Obj};
use gdext_builtin::GodotString;
use std::fmt::Debug;

pub mod marker {
    use crate::{GodotClass, Obj};

    pub trait ClassType {
        fn extract_from_obj<T: GodotClass>(obj: &Obj<T>) -> &T;
    }

    pub enum EngineClass {}
    impl ClassType for EngineClass {
        fn extract_from_obj<T: GodotClass>(obj: &Obj<T>) -> &T {
            unsafe { std::mem::transmute(&obj.opaque) }
        }
    }

    pub enum UserClass {}
    impl ClassType for UserClass {
        fn extract_from_obj<T: GodotClass>(obj: &Obj<T>) -> &T {
            obj.storage().get()
        }
    }
}

pub trait EngineClass: GodotClass {
    fn from_object_ptr(object_ptr: sys::GDNativeObjectPtr) -> Self;
    fn as_object_ptr(&self) -> sys::GDNativeObjectPtr;
    fn as_type_ptr(&self) -> sys::GDNativeTypePtr;

    fn from_obj(obj: &Obj<Self>) -> &Self {
        let inst = Self::from_object_ptr(obj.obj_sys());

        Box::leak(Box::new(inst))

        /*unsafe {
            std::mem::transmute(obj.opaque)
        }*/
    }
}

pub trait GodotClass: Debug
where
    Self: Sized,
{
    type Base: GodotClass;
    type ClassType: marker::ClassType;

    fn class_name() -> String;

    // fn native_object_ptr(&self) -> sys::GDNativeObjectPtr {
    //     self.upcast().native_object_ptr()
    // }
    //fn upcast(&self) -> &Self::Base;
    //fn upcast_mut(&mut self) -> &mut Self::Base;
}

impl GodotClass for () {
    type Base = ();
    type ClassType = marker::EngineClass;

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
