/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::builder::ClassBuilder;
use crate::builtin::GodotString;
use crate::obj::Base;

use godot_ffi as sys;

/// Makes `T` eligible to be managed by Godot and stored in [`Gd<T>`][crate::obj::Gd] pointers.
///
/// The behavior of types implementing this trait is influenced by the associated types; check their documentation for information.
///
/// # Safety
///
/// Internal.
/// You **must not** implement this trait yourself; use [`#[derive(GodotClass)`](../bind/derive.GodotClass.html) instead.
// Above intra-doc link to the derive-macro only works as HTML, not as symbol link.
pub unsafe trait GodotClass: 'static
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

    /// The name of the class, under which it is registered in Godot.
    ///
    /// This may deviate from the Rust struct name: `HttpRequest::CLASS_NAME == "HTTPRequest"`.
    const CLASS_NAME: &'static str;

    /// Returns whether `Self` inherits from `U`.
    ///
    /// This is reflexive, i.e `Self` inherits from itself.
    ///
    /// See also [`Inherits`] for a trait bound.
    fn inherits<U: GodotClass>() -> bool {
        if Self::CLASS_NAME == U::CLASS_NAME {
            true
        } else if Self::Base::CLASS_NAME == <()>::CLASS_NAME {
            false
        } else {
            Self::Base::inherits::<U>()
        }
    }
}

/// Unit impl only exists to represent "no base", and is used for exactly one class: `Object`.
unsafe impl GodotClass for () {
    type Base = ();
    type Declarer = dom::EngineDomain;
    type Mem = mem::ManualMemory;

    const CLASS_NAME: &'static str = "(no base)";
}

/// Trait to create more references from a smart pointer or collection.
pub trait Share {
    /// Creates a new reference that points to the same object.
    ///
    /// If the referred-to object is reference-counted, this will increment the count.
    fn share(&self) -> Self;
}

/// Non-strict inheritance relationship in the Godot class hierarchy.
///
/// `Derived: Inherits<Base>` means that either `Derived` is a subclass of `Base`, or the class `Base` itself (hence "non-strict").
///
/// This trait is automatically implemented for all Godot engine classes and user-defined classes that derive [`GodotClass`].
/// It has `GodotClass` as a supertrait, allowing your code to have bounds solely on `Derived: Inherits<Base>` rather than
/// `Derived: Inherits<Base> + GodotClass`.
///
/// Inheritance is transitive across indirect base classes: `Node3D` implements `Inherits<Node>` and `Inherits<Object>`.
///
/// The trait is also reflexive: `T` always implements `Inherits<T>`.
///
/// # Usage
///
/// The primary use case for this trait is polymorphism: you write a function that accepts anything that derives from a certain class
/// (including the class itself):
/// ```no_run
/// # use godot::prelude::*;
/// fn print_node<T>(node: Gd<T>)
/// where
///     T: Inherits<Node>,
/// {
///     let up = node.upcast(); // type Gd<Node> inferred
///     println!("Node #{} with name {}", up.instance_id(), up.get_name());
///     up.free();
/// }
///
/// // Call with different types
/// print_node(Node::new_alloc());   // works on T=Node as well
/// print_node(Node2D::new_alloc()); // or derived classes
/// print_node(Node3D::new_alloc());
/// ```
///
/// A variation of the above pattern works without `Inherits` or generics, if you move the `upcast()` into the call site:
/// ```no_run
/// # use godot::prelude::*;
/// fn print_node(node: Gd<Node>) { /* ... */ }
///
/// // Call with different types
/// print_node(Node::new_alloc());            // no upcast needed
/// print_node(Node2D::new_alloc().upcast());
/// print_node(Node3D::new_alloc().upcast());
/// ```
///
pub trait Inherits<Base>: GodotClass {}

impl<T: GodotClass> Inherits<T> for T {}

/// Trait implemented for all objects that inherit from `Resource` or `Node`. As those are the only objects
/// you can export to the editor.
pub trait ExportableObject: GodotClass {}

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

    // TODO Evaluate whether we want this public or not
    #[doc(hidden)]
    pub trait GodotToString: GodotClass {
        #[doc(hidden)]
        fn __godot_to_string(&self) -> GodotString;
    }

    // TODO Evaluate whether we want this public or not
    #[doc(hidden)]
    pub trait GodotNotification: GodotClass {
        #[doc(hidden)]
        fn __godot_notification(&mut self, what: i32);
    }

    // TODO Evaluate whether we want this public or not
    #[doc(hidden)]
    pub trait GodotRegisterClass: GodotClass {
        #[doc(hidden)]
        fn __godot_register_class(builder: &mut ClassBuilder<Self>);
    }

    /// Auto-implemented for `#[godot_api] impl MyClass` blocks
    pub trait ImplementsGodotApi: GodotClass {
        #[doc(hidden)]
        fn __register_methods();
        #[doc(hidden)]
        fn __register_constants();
    }

    pub trait ImplementsGodotExports: GodotClass {
        #[doc(hidden)]
        fn __register_exports();
    }

    /// Auto-implemented for `#[godot_api] impl XyVirtual for MyClass` blocks
    pub trait ImplementsGodotVirtual: GodotClass {
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

    /// Trait that specifies who declares a given `GodotClass`.
    pub trait Domain: Sealed {
        #[doc(hidden)]
        fn scoped_mut<T, F, R>(obj: &mut Gd<T>, closure: F) -> R
        where
            T: GodotClass<Declarer = Self>,
            F: FnOnce(&mut T) -> R;
    }

    /// Expresses that a class is declared by the Godot engine.
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

    /// Expresses that a class is declared by the user.
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
    use godot_ffi::PtrcallType;

    use super::private::Sealed;
    use crate::obj::{Gd, GodotClass};
    use crate::out;

    /// Specifies the memory
    pub trait Memory: Sealed {
        /// Initialize reference counter
        #[doc(hidden)]
        fn maybe_init_ref<T: GodotClass>(obj: &Gd<T>);

        /// If ref-counted, then increment count
        #[doc(hidden)]
        fn maybe_inc_ref<T: GodotClass>(obj: &Gd<T>);

        /// If ref-counted, then decrement count
        #[doc(hidden)]
        fn maybe_dec_ref<T: GodotClass>(obj: &Gd<T>) -> bool;

        /// Check if ref-counted, return `None` if information is not available (dynamic and obj dead)
        #[doc(hidden)]
        fn is_ref_counted<T: GodotClass>(obj: &Gd<T>) -> Option<bool>;

        /// Returns `true` if argument and return pointers are passed as `Ref<T>` pointers given this
        /// [`PtrcallType`].
        ///
        /// See [`PtrcallType::Virtual`] for information about `Ref<T>` objects.
        #[doc(hidden)]
        fn pass_as_ref(_call_type: PtrcallType) -> bool {
            false
        }
    }

    #[doc(hidden)]
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

        fn pass_as_ref(call_type: PtrcallType) -> bool {
            matches!(call_type, PtrcallType::Virtual)
        }
    }

    /// Memory managed through Godot reference counter, if present; otherwise manual.
    /// This is used only for `Object` classes.
    pub struct DynamicRefCount {}
    impl Sealed for DynamicRefCount {}
    impl Memory for DynamicRefCount {
        fn maybe_init_ref<T: GodotClass>(obj: &Gd<T>) {
            out!("  Dyn::init  <{}>", std::any::type_name::<T>());
            if obj
                .instance_id_or_none()
                .map(|id| id.is_ref_counted())
                .unwrap_or(false)
            {
                StaticRefCount::maybe_init_ref(obj)
            }
        }

        fn maybe_inc_ref<T: GodotClass>(obj: &Gd<T>) {
            out!("  Dyn::inc   <{}>", std::any::type_name::<T>());
            if obj
                .instance_id_or_none()
                .map(|id| id.is_ref_counted())
                .unwrap_or(false)
            {
                StaticRefCount::maybe_inc_ref(obj)
            }
        }

        fn maybe_dec_ref<T: GodotClass>(obj: &Gd<T>) -> bool {
            out!("  Dyn::dec   <{}>", std::any::type_name::<T>());
            if obj
                .instance_id_or_none()
                .map(|id| id.is_ref_counted())
                .unwrap_or(false)
            {
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
