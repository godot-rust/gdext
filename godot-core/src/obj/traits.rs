/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::obj::Base;

use godot_ffi as sys;

/// Makes `T` eligible to be managed by Godot and stored in [`Gd<T>`][crate::obj::Gd] pointers.
///
/// The behavior of types implementing this trait is influenced by the associated types; check their documentation for information.
pub trait GodotClass: 'static
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

    const CLASS_NAME: &'static str;
}

/// Unit impl only exists to represent "no base", and is used for exactly one class: `Object`.
impl GodotClass for () {
    type Base = ();
    type Declarer = dom::EngineDomain;
    type Mem = mem::ManualMemory;

    const CLASS_NAME: &'static str = "(no base)";
}

/// Trait to create more references from a smart pointer or collection.
pub trait Share {
    /// Creates a new reference that points to the same obj.
    ///
    /// If the referred-to obj is reference-counted, this will increment the count.
    fn share(&self) -> Self;
}

/// A struct `Derived` implementing `Inherits<Base>` expresses that `Derived` _strictly_ inherits `Base` in the Godot hierarchy.
///
/// This trait is implemented for all Godot engine classes, even for non-direct relations (e.g. `Node3D` implements `Inherits<Object>`). Deriving [`GodotClass`] for custom classes will achieve the same: all direct and indirect base
/// classes of your extension class will be wired up using the `Inherits` relation.
///
/// The trait is not reflexive: `T` never implements `Inherits<T>`.
pub trait Inherits<Base> {}

/// Auto-implemented for all engine-provided classes
pub trait EngineClass: GodotClass {
    fn as_object_ptr(&self) -> sys::GDExtensionObjectPtr;
    fn as_type_ptr(&self) -> sys::GDExtensionTypePtr;
}

/// Auto-implemented for all engine-provided enums
pub trait EngineEnum: Copy {
    fn try_from_ord(ord: i32) -> Option<Self>;

    /// Ordinal value of the enumerator, as specified in Godot.
    /// This is not necessarily unique.
    fn ord(self) -> i32;

    fn from_ord(ord: i32) -> Self {
        Self::try_from_ord(ord)
            .unwrap_or_else(|| panic!("ordinal {ord} does not map to any enumerator"))
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

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
    /// This trait is not manually implemented, and you cannot call any methods.
    /// Instead, the trait will be provided to you by the proc macros, and you can
    /// use it as a bound.
    pub trait GodotInit: GodotClass {
        #[doc(hidden)]
        fn __godot_init(base: Base<Self::Base>) -> Self;
    }

    /// Auto-implemented for `#[godot_api] impl MyClass` blocks
    pub trait ImplementsGodotApi: GodotClass {
        #[doc(hidden)]
        fn __register_methods();
    }

    pub trait ImplementsGodotExports: GodotClass {
        #[doc(hidden)]
        fn __register_exports();
    }

    /// Auto-implemented for `#[godot_api] impl GodotExt for MyClass` blocks
    pub trait ImplementsGodotExt: GodotClass {
        #[doc(hidden)]
        fn __virtual_call(_name: &str) -> sys::GDExtensionClassCallVirtual;
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Domain + Memory classifiers

mod private {
    pub trait Sealed {}
}

pub mod dom {
    use super::private::Sealed;
    use crate::obj::{Gd, GodotClass};
    use std::ops::DerefMut;

    pub trait Domain: Sealed {
        fn scoped_mut<T, F, R>(obj: &mut Gd<T>, closure: F) -> R
        where
            T: GodotClass<Declarer = Self>,
            F: FnOnce(&mut T) -> R;
    }

    pub enum EngineDomain {}
    impl Sealed for EngineDomain {}
    impl Domain for EngineDomain {
        fn scoped_mut<T, F, R>(obj: &mut Gd<T>, closure: F) -> R
        where
            T: GodotClass<Declarer = EngineDomain>,
            F: FnOnce(&mut T) -> R,
        {
            closure(obj.deref_mut())
        }
    }

    pub enum UserDomain {}
    impl Sealed for UserDomain {}
    impl Domain for UserDomain {
        fn scoped_mut<T, F, R>(obj: &mut Gd<T>, closure: F) -> R
        where
            T: GodotClass<Declarer = Self>,
            F: FnOnce(&mut T) -> R,
        {
            let mut guard = obj.bind_mut();
            closure(guard.deref_mut())
        }
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

pub mod mem {
    use super::private::Sealed;
    use crate::obj::{Gd, GodotClass};
    use crate::out;

    pub trait Memory: Sealed {
        /// Initialize reference counter
        fn maybe_init_ref<T: GodotClass>(obj: &Gd<T>);

        /// If ref-counted, then increment count
        fn maybe_inc_ref<T: GodotClass>(obj: &Gd<T>);

        /// If ref-counted, then decrement count
        fn maybe_dec_ref<T: GodotClass>(obj: &Gd<T>) -> bool;

        /// Check if ref-counted, return `None` if information is not available (dynamic and obj dead)
        fn is_ref_counted<T: GodotClass>(obj: &Gd<T>) -> Option<bool>;
    }
    pub trait PossiblyManual {}

    /// Memory managed through Godot reference counter (always present).
    /// This is used for `RefCounted` classes and derived.
    pub struct StaticRefCount {}
    impl Sealed for StaticRefCount {}
    impl Memory for StaticRefCount {
        fn maybe_init_ref<T: GodotClass>(obj: &Gd<T>) {
            out!("  Stat::init  <{}>", std::any::type_name::<T>());
            obj.as_ref_counted(|refc| {
                let success = refc.init_ref();
                assert!(success, "init_ref() failed");
            });
        }

        fn maybe_inc_ref<T: GodotClass>(obj: &Gd<T>) {
            out!("  Stat::inc   <{}>", std::any::type_name::<T>());
            obj.as_ref_counted(|refc| {
                let success = refc.reference();
                assert!(success, "reference() failed");
            });
        }

        fn maybe_dec_ref<T: GodotClass>(obj: &Gd<T>) -> bool {
            out!("  Stat::dec   <{}>", std::any::type_name::<T>());
            obj.as_ref_counted(|refc| {
                let is_last = refc.unreference();
                out!("  +-- was last={is_last}");
                is_last
            })
        }

        fn is_ref_counted<T: GodotClass>(_obj: &Gd<T>) -> Option<bool> {
            Some(true)
        }
    }

    /// Memory managed through Godot reference counter, if present; otherwise manual.
    /// This is used only for `Object` classes.
    pub struct DynamicRefCount {}
    impl Sealed for DynamicRefCount {}
    impl Memory for DynamicRefCount {
        fn maybe_init_ref<T: GodotClass>(obj: &Gd<T>) {
            out!("  Dyn::init  <{}>", std::any::type_name::<T>());
            if obj.instance_id().is_ref_counted() {
                StaticRefCount::maybe_init_ref(obj);
            }
        }

        fn maybe_inc_ref<T: GodotClass>(obj: &Gd<T>) {
            out!("  Dyn::inc   <{}>", std::any::type_name::<T>());
            if obj.instance_id().is_ref_counted() {
                StaticRefCount::maybe_inc_ref(obj);
            }
        }

        fn maybe_dec_ref<T: GodotClass>(obj: &Gd<T>) -> bool {
            out!("  Dyn::dec   <{}>", std::any::type_name::<T>());
            if obj.instance_id().is_ref_counted() {
                StaticRefCount::maybe_dec_ref(obj)
            } else {
                false
            }
        }

        fn is_ref_counted<T: GodotClass>(obj: &Gd<T>) -> Option<bool> {
            // Return `None` if obj is dead
            obj.instance_id_or_none().map(|id| id.is_ref_counted())
        }
    }

    impl PossiblyManual for DynamicRefCount {}

    /// No memory management, user responsible for not leaking.
    /// This is used for all `Object` derivates, which are not `RefCounted`. `Object` itself is also excluded.
    pub struct ManualMemory {}
    impl Sealed for ManualMemory {}
    impl Memory for ManualMemory {
        fn maybe_init_ref<T: GodotClass>(_obj: &Gd<T>) {}
        fn maybe_inc_ref<T: GodotClass>(_obj: &Gd<T>) {}
        fn maybe_dec_ref<T: GodotClass>(_obj: &Gd<T>) -> bool {
            false
        }
        fn is_ref_counted<T: GodotClass>(_obj: &Gd<T>) -> Option<bool> {
            Some(false)
        }
    }
    impl PossiblyManual for ManualMemory {}
}
