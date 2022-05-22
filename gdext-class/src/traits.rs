use crate::sys;
use gdext_builtin::GodotString;
use std::fmt::Debug;

pub mod marker {
    use crate::{GodotClass, Obj};

    pub trait ClassDeclarer {
        fn extract_from_obj<T: GodotClass>(obj: &Obj<T>) -> &T;
    }

    pub enum EngineClass {}
    impl ClassDeclarer for EngineClass {
        fn extract_from_obj<T: GodotClass>(obj: &Obj<T>) -> &T {
            // This relies on Obj<Node3D> having the layout as Node3D (as an example),
            // which also needs #[repr(transparent)]:
            //
            // struct Obj<T: GodotClass> {
            //     opaque: OpaqueObject,         <- size of GDNativeObjectPtr
            //     _marker: PhantomData,         <- ZST
            // }
            // struct Node3D {
            //     object_ptr: sys::GDNativeObjectPtr,
            // }
            unsafe { std::mem::transmute::<&Obj<T>, &T>(obj) }
        }
    }

    pub enum UserClass {}
    impl ClassDeclarer for UserClass {
        fn extract_from_obj<T: GodotClass>(obj: &Obj<T>) -> &T {
            obj.storage().get()
        }
    }
}

pub trait EngineClass {
    fn from_object_ptr(object_ptr: sys::GDNativeObjectPtr) -> Self;
    fn as_object_ptr(&self) -> sys::GDNativeObjectPtr;
    fn as_type_ptr(&self) -> sys::GDNativeTypePtr;
}

pub trait GodotClass: Debug
where
    Self: Sized,
{
    type Base: GodotClass;
    type Declarer: marker::ClassDeclarer;

    fn class_name() -> String;
}

impl GodotClass for () {
    type Base = ();
    type Declarer = marker::EngineClass;

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
