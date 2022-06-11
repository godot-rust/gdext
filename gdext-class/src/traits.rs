use crate::sys;
use gdext_builtin::GodotString;
use std::fmt::Debug;

pub mod marker {
    use crate::{GodotClass, Obj};

    pub trait ClassDeclarer {
        fn extract_from_obj<T: GodotClass>(obj: &Obj<T>) -> &T;
        fn extract_from_obj_mut<T: GodotClass>(obj: &mut Obj<T>) -> &mut T;
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

        fn extract_from_obj_mut<T: GodotClass>(obj: &mut Obj<T>) -> &mut T {
            unsafe { std::mem::transmute::<&mut Obj<T>, &mut T>(obj) }
        }
    }

    pub enum UserClass {}
    impl ClassDeclarer for UserClass {
        fn extract_from_obj<T: GodotClass>(obj: &Obj<T>) -> &T {
            obj.storage().get()
        }
        fn extract_from_obj_mut<T: GodotClass>(obj: &mut Obj<T>) -> &mut T {
            obj.storage().get_mut()
        }
    }
}
// ----------------------------------------------------------------------------------------------------------------------------------------------

#[allow(dead_code)]
pub mod mem {
    pub trait Memory {}

    pub struct StaticRefCount {}
    impl Memory for StaticRefCount {}

    pub struct DynamicRefCount {
        is_refcounted: bool,
    }
    impl Memory for DynamicRefCount {}

    pub struct ManualMemory {}
    impl Memory for ManualMemory {}
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

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
    //type Memory: mem::Memory;

    fn class_name() -> String;
}

impl GodotClass for () {
    type Base = ();
    type Declarer = marker::EngineClass;
    //type Memory = mem::ManualMemory;

    fn class_name() -> String {
        "(no base)".to_string()
    }
}

pub trait DefaultConstructible: GodotClass {
    //fn construct(base: Obj<Self::Base>) -> Self;
    fn construct(base: sys::GDNativeObjectPtr) -> Self;
}

pub trait GodotExtensionClass: GodotClass {
    // fn reference(&mut self) {}
    // fn unreference(&mut self) {}
    fn has_to_string() -> bool {
        false
    }

    fn virtual_call(name: &str) -> sys::GDNativeExtensionClassCallVirtual;
    fn register_methods();
    fn to_string(&self) -> GodotString {
        GodotString::new()
    }
}
