use crate::{sys, ClassName};
use gdext_builtin::GodotString;
use gdext_sys::{interface_fn};
use std::fmt::Debug;
use std::ptr::addr_of;

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
    fn as_object_ptr(&self) -> sys::GDNativeObjectPtr;
    fn as_type_ptr(&self) -> sys::GDNativeTypePtr;

    fn upcast<Base>(&self) -> &Base
        where
            Base: GodotClass,
            Self: Inherits<Base>,
    {
        // Transmuting unsafe { std::mem::transmute<&Self, &Base>(self) } is probably not safe, since
        // C++ static_cast class casts may yield a different pointer (VTable offset, virtual inheritance etc.)

        let class_name = ClassName::new::<Base>();
        unsafe {
            let class_tag = interface_fn!(classdb_get_class_tag)(class_name.c_str());
            let cast_object_ptr = interface_fn!(object_cast_to)(self.as_object_ptr(), class_tag);

            let cast_struct_ptr = addr_of!(cast_object_ptr) as *const Base;
            &*cast_struct_ptr // -> &Base
        }
        // FIXME this can't work because the pointer needs to be stored somewhere, and &Base points to something that goes out of scope -> UB
    }
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

/// A struct `Derived` implementing `Inherits<Base>` expresses that `Derived` inherits `Base` in the Godot hierarchy.
///
/// This trait is implemented for all Godot engine classes, even for non-direct relations (e.g. `Node3D` implements `Inherits<Object>`).
// note: could also be named `SubclassOf`
pub trait Inherits<Base> {}
