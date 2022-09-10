use crate::builder::ClassBuilder;
use crate::{sys, Base};
use gdext_builtin::GodotString;
use std::fmt::Debug;

mod private {
    pub trait Sealed {}
}

pub mod dom {
    use super::private::Sealed;
    use crate::{GodotClass, Obj};
    use gdext_sys::types::OpaqueObject;

    pub trait Domain: Sealed {
        fn extract_from_obj<T: GodotClass<Declarer = Self>>(obj: &Obj<T>) -> &T;
        fn extract_from_obj_mut<T: GodotClass<Declarer = Self>>(obj: &mut Obj<T>) -> &mut T;
    }

    pub enum EngineDomain {}
    impl Sealed for EngineDomain {}
    impl Domain for EngineDomain {
        fn extract_from_obj<T: GodotClass<Declarer = Self>>(obj: &Obj<T>) -> &T {
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
            unsafe { std::mem::transmute::<&OpaqueObject, &T>(&obj.opaque) }
        }

        fn extract_from_obj_mut<T: GodotClass<Declarer = Self>>(obj: &mut Obj<T>) -> &mut T {
            unsafe { std::mem::transmute::<&mut OpaqueObject, &mut T>(&mut obj.opaque) }
        }
    }

    pub enum UserDomain {}
    impl Sealed for UserDomain {}
    impl Domain for UserDomain {
        fn extract_from_obj<T: GodotClass<Declarer = Self>>(obj: &Obj<T>) -> &T {
            obj.storage().get()
        }

        fn extract_from_obj_mut<T: GodotClass<Declarer = Self>>(obj: &mut Obj<T>) -> &mut T {
            obj.storage().get_mut()
        }
    }
}
// ----------------------------------------------------------------------------------------------------------------------------------------------

pub mod mem {
    use super::private::Sealed;
    use crate::{out, GodotClass, Obj};

    pub trait Memory: Sealed {
        fn maybe_init_ref<T: GodotClass>(obj: &Obj<T>);
        fn maybe_inc_ref<T: GodotClass>(obj: &Obj<T>);
        fn maybe_dec_ref<T: GodotClass>(obj: &Obj<T>) -> bool;
        fn is_ref_counted<T: GodotClass>(obj: &Obj<T>) -> bool;
    }
    pub trait PossiblyManual {}

    /// Memory managed through Godot reference counter (always present).
    /// This is used for `RefCounted` classes and derived.
    pub struct StaticRefCount {}
    impl Sealed for StaticRefCount {}
    impl Memory for StaticRefCount {
        fn maybe_init_ref<T: GodotClass>(obj: &Obj<T>) {
            out!("  Stat::init  <{}>", std::any::type_name::<T>());
            obj.as_ref_counted(|refc| {
                let success = refc.init_ref();
                assert!(success, "init_ref() failed");
            });
        }

        fn maybe_inc_ref<T: GodotClass>(obj: &Obj<T>) {
            out!("  Stat::inc   <{}>", std::any::type_name::<T>());
            obj.as_ref_counted(|refc| {
                let success = refc.reference();
                assert!(success, "reference() failed");
            });
        }

        fn maybe_dec_ref<T: GodotClass>(obj: &Obj<T>) -> bool {
            out!("  Stat::dec   <{}>", std::any::type_name::<T>());
            obj.as_ref_counted(|refc| {
                let is_last = refc.unreference();
                out!("  +-- was last={is_last}");
                is_last
            })
        }

        fn is_ref_counted<T: GodotClass>(_obj: &Obj<T>) -> bool {
            true
        }
    }

    /// Memory managed through Godot reference counter, if present; otherwise manual.
    /// This is used only for `Object` classes.
    pub struct DynamicRefCount {}
    impl Sealed for DynamicRefCount {}
    impl Memory for DynamicRefCount {
        fn maybe_init_ref<T: GodotClass>(obj: &Obj<T>) {
            out!("  Dyn::init  <{}>", std::any::type_name::<T>());
            if obj.instance_id().is_ref_counted() {
                StaticRefCount::maybe_init_ref(obj);
            }
        }

        fn maybe_inc_ref<T: GodotClass>(obj: &Obj<T>) {
            out!("  Dyn::inc   <{}>", std::any::type_name::<T>());
            if obj.instance_id().is_ref_counted() {
                StaticRefCount::maybe_inc_ref(obj);
            }
        }

        fn maybe_dec_ref<T: GodotClass>(obj: &Obj<T>) -> bool {
            out!("  Dyn::dec   <{}>", std::any::type_name::<T>());
            if obj.instance_id().is_ref_counted() {
                StaticRefCount::maybe_dec_ref(obj)
            } else {
                false
            }
        }

        fn is_ref_counted<T: GodotClass>(obj: &Obj<T>) -> bool {
            obj.instance_id().is_ref_counted()
        }
    }
    impl PossiblyManual for DynamicRefCount {}

    /// No memory management, user responsible for not leaking.
    /// This is used for all `Object` derivates, which are not `RefCounted`. `Object` itself is also excluded.
    pub struct ManualMemory {}
    impl Sealed for ManualMemory {}
    impl Memory for ManualMemory {
        fn maybe_init_ref<T: GodotClass>(_obj: &Obj<T>) {}
        fn maybe_inc_ref<T: GodotClass>(_obj: &Obj<T>) {}
        fn maybe_dec_ref<T: GodotClass>(_obj: &Obj<T>) -> bool {
            false
        }
        fn is_ref_counted<T: GodotClass>(_obj: &Obj<T>) -> bool {
            false
        }
    }
    impl PossiblyManual for ManualMemory {}
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

/// Makes `T` eligible to be managed by Godot and stored in [`Obj<T>`][crate::Obj] pointers.
///
/// The behavior of types implementing this trait is influenced by the associated types; check their documentation for information.
pub trait GodotClass: Debug + 'static
where
    Self: Sized,
{
    /// The immediate superclass of `T`. This is always a Godot engine class.
    type Base: GodotClass; // not EngineClass because it can be ()

    /// Whether this class is a core Godot class provided by the engine, or declared by the user as a Rust struct.
    // TODO what about GDScript user classes?
    type Declarer: dom::Domain;

    /// Defines the memory strategy.
    type Mem: mem::Memory;

    fn class_name() -> String;
}

/// Unit impl only exists to represent "no base", and is used for exactly one class: `Object`.
impl GodotClass for () {
    type Base = ();
    type Declarer = dom::EngineDomain;
    type Mem = mem::ManualMemory;

    fn class_name() -> String {
        "(no base)".to_string()
    }
}

pub trait EngineClass: GodotClass {
    fn as_object_ptr(&self) -> sys::GDNativeObjectPtr;
    fn as_type_ptr(&self) -> sys::GDNativeTypePtr;
}

#[allow(unused_variables)]
pub trait GodotMethods
where
    Self: GodotClass,
{
    // Note: keep in sync with VIRTUAL_METHOD_NAMES in godot_api.rs

    // Some methods that were called:
    // _enter_tree
    // _input
    // _shortcut_input
    // _unhandled_input
    // _unhandled_key_input
    // _process
    // _physics_process
    // _ready

    fn register_class(builder: &mut ClassBuilder<Self>) {}

    fn init(base: Base<Self::Base>) -> Self {
        unimplemented!()
    }

    fn ready(&mut self) {
        unreachable!()
    }
    fn process(&mut self, delta: f64) {
        unimplemented!()
    }
    fn physics_process(&mut self, delta: f64) {
        unimplemented!()
    }
    fn to_string(&self) -> GodotString {
        unimplemented!()
    }
}

/// Capability traits, providing dedicated functionalities for Godot classes
pub mod cap {
    use super::*;

    /// Trait for all classes that are constructible from the Godot engine.
    ///
    /// Godot can only construct user-provided classes in one way: with the default
    /// constructor. This is what happens when you write `MyClass.new()` in GDScript.
    /// You can disable this constructor by not providing an `init` method for your
    /// class; in that case construction fails.
    ///
    /// This trait is not manually implemented, and you cannot call its method.
    /// Instead, the trait will be provided to you by the proc macros, and you can
    /// use it as a bound.
    pub trait GodotInit: GodotClass {
        fn __godot_init(base: Base<Self::Base>) -> Self;
    }
}

pub trait UserMethodBinds: GodotClass {
    fn register_methods();
}

pub trait UserVirtuals: GodotClass {
    fn virtual_call(_name: &str) -> sys::GDNativeExtensionClassCallVirtual {
        None // TODO
    }
}

/// Trait to create more references from a smart pointer or collection.
pub trait Share {
    /// Creates a new reference that points to the same object.
    ///
    /// If the referred-to object is reference-counted, this will increment the count.
    fn share(&self) -> Self;
}

/// A struct `Derived` implementing `Inherits<Base>` expresses that `Derived` _strictly_ inherits `Base` in the Godot hierarchy.
///
/// This trait is implemented for all Godot engine classes, even for non-direct relations (e.g. `Node3D` implements `Inherits<Object>`). Deriving [`GodotClass`] for custom classes will achieve the same: all direct and indirect base
/// classes of your extension class will be wired up using the `Inherits` relation.
///
/// The trait is not reflexive: `T` never implements `Inherits<T>`.
pub trait Inherits<Base> {}
