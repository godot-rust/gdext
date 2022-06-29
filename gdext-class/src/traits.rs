use crate::{sys, Obj};
use gdext_builtin::GodotString;
use std::fmt::Debug;

pub mod dom {
    use crate::{GodotClass, Obj};

    pub trait Domain {
        fn extract_from_obj<T: GodotClass>(obj: &Obj<T>) -> &T;
        fn extract_from_obj_mut<T: GodotClass>(obj: &mut Obj<T>) -> &mut T;
    }

    pub enum EngineDomain {}
    impl Domain for EngineDomain {
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

    pub enum UserDomain {}
    impl Domain for UserDomain {
        fn extract_from_obj<T: GodotClass>(obj: &Obj<T>) -> &T {
            obj.storage().get()
        }

        fn extract_from_obj_mut<T: GodotClass>(obj: &mut Obj<T>) -> &mut T {
            obj.storage().get_mut()
        }
    }
}
// ----------------------------------------------------------------------------------------------------------------------------------------------

pub mod mem {
    use crate::{out, GodotClass, Obj};

    pub trait Memory {
        fn maybe_init_ref<T: GodotClass>(obj: &Obj<T>);
        fn maybe_inc_ref<T: GodotClass>(obj: &Obj<T>);
        fn maybe_dec_ref<T: GodotClass>(obj: &Obj<T>) -> bool;
    }

    /// Memory managed through Godot reference counter (always present).
    /// This is used for `RefCounted` classes and derived.
    pub struct StaticRefCount {}
    impl Memory for StaticRefCount {
        fn maybe_init_ref<T: GodotClass>(obj: &Obj<T>) {
            out!("  Stat::init");
            obj.as_ref_counted(|refc| {
                let success = refc.init_ref();
                assert!(success, "init_ref() failed");
            });
        }

        fn maybe_inc_ref<T: GodotClass>(obj: &Obj<T>) {
            out!("  Stat::inc");
            obj.as_ref_counted(|refc| {
                let success = refc.reference();
                assert!(success, "reference() failed");
            });
        }

        fn maybe_dec_ref<T: GodotClass>(obj: &Obj<T>) -> bool {
            out!("  Stat::dec");
            obj.as_ref_counted(|refc| {
                let is_last = refc.unreference();
                out!("  +-- was last={is_last}");
                is_last
            })
        }
    }

    /// Memory managed through Godot reference counter, if present; otherwise manual.
    /// This is used only for `Object` classes.
    pub struct DynamicRefCount {}
    impl Memory for DynamicRefCount {
        fn maybe_init_ref<T: GodotClass>(obj: &Obj<T>) {
            out!("  Dyn::init");
            if obj.instance_id().is_ref_counted() {
                StaticRefCount::maybe_init_ref(obj);
            }
        }

        fn maybe_inc_ref<T: GodotClass>(obj: &Obj<T>) {
            out!("  Dyn::inc");
            if obj.instance_id().is_ref_counted() {
                StaticRefCount::maybe_inc_ref(obj);
            }
        }

        fn maybe_dec_ref<T: GodotClass>(obj: &Obj<T>) -> bool {
            out!("  Dyn::dec");
            if obj.instance_id().is_ref_counted() {
                StaticRefCount::maybe_dec_ref(obj)
            } else {
                false
            }
        }
    }

    /// No memory management, user responsible for not leaking.
    /// This is used for all `Object` derivates, except `RefCounted` (and except `Object` itself).
    pub struct ManualMemory {}
    impl Memory for ManualMemory {
        fn maybe_init_ref<T: GodotClass>(_obj: &Obj<T>) {}
        fn maybe_inc_ref<T: GodotClass>(_obj: &Obj<T>) {}
        fn maybe_dec_ref<T: GodotClass>(_obj: &Obj<T>) -> bool {
            false
        }
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
    type Declarer: dom::Domain;
    type Mem: mem::Memory;

    fn class_name() -> String;
}

impl GodotClass for () {
    type Base = ();
    type Declarer = dom::EngineDomain;
    type Mem = mem::ManualMemory;

    fn class_name() -> String {
        "(no base)".to_string()
    }
}

pub trait GodotDefault: GodotClass {
    fn construct(base: Obj<Self::Base>) -> Self;
}

pub trait GodotMethods: GodotClass {
    // Some methods that were called:
    // _enter_tree
    // _input
    // _shortcut_input
    // _unhandled_input
    // _unhandled_key_input
    // _process
    // _physics_process
    // _ready

    fn construct(base: Obj<Self::Base>) -> Self {
        unimplemented!()
    }
    fn ready(&mut self) {
        unimplemented!()
    }
    fn process(&mut self, delta: f64) {
        unimplemented!()
    }
    fn to_string(&self) -> GodotString {
        unimplemented!()
    }
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
