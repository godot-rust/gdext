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
    use crate::{GodotClass, Obj};

    pub trait Memory {
        fn maybe_inc_ref<T: GodotClass>(obj: &Obj<T>);
    }

    pub struct StaticRefCount {}
    impl Memory for StaticRefCount {
        fn maybe_inc_ref<T: GodotClass>(obj: &Obj<T>) {
            obj.as_ref_counted(|refc| {
                let success = refc.reference();
                assert!(success);
            });
        }
    }

    pub struct DynamicRefCount {
        is_refcounted: bool,
    }
    impl Memory for DynamicRefCount {
        fn maybe_inc_ref<T: GodotClass>(obj: &Obj<T>) {
            todo!()
        }
    }

    pub struct ManualMemory {}
    impl Memory for ManualMemory {
        fn maybe_inc_ref<T: GodotClass>(_obj: &Obj<T>) {}
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

pub trait EngineClass {
    fn as_object_ptr(&self) -> sys::GDNativeObjectPtr;
    fn as_type_ptr(&self) -> sys::GDNativeTypePtr;
}

pub trait GodotClass: Debug
where
    Self: Sized,
{
    type Base: GodotClass;
    type Declarer: marker::ClassDeclarer;
    type Mem: mem::Memory;

    fn class_name() -> String;
}

impl GodotClass for () {
    type Base = ();
    type Declarer = marker::EngineClass;
    type Mem = mem::ManualMemory;

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

pub trait Share {
    fn share(&self) -> Self;
}

/// A struct `Derived` implementing `Inherits<Base>` expresses that `Derived` inherits `Base` in the Godot hierarchy.
///
/// This trait is implemented for all Godot engine classes, even for non-direct relations (e.g. `Node3D` implements `Inherits<Object>`).
// note: could also be named `SubclassOf`
pub trait Inherits<Base> {}
