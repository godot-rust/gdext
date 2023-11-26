/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::builtin::meta::ClassName;
use crate::builtin::GString;
use crate::init::InitLevel;
use crate::obj::Gd;
use crate::{builder::ClassBuilder, storage::Storage};

use godot_ffi as sys;

use super::{Base, BaseMut, BaseRef};

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

    /// During which initialization level this class is available/should be initialized with Godot.
    ///
    /// Is `None` if the class has complicated initialization requirements, and generally cannot be inherited
    /// from (currently only for `()`, the "base" of `Object`).
    const INIT_LEVEL: Option<InitLevel>;

    /// The name of the class, under which it is registered in Godot.
    ///
    /// This may deviate from the Rust struct name: `HttpRequest::class_name().as_str() == "HTTPRequest"`.
    fn class_name() -> ClassName;

    /// Returns whether `Self` inherits from `U`.
    ///
    /// This is reflexive, i.e `Self` inherits from itself.
    ///
    /// See also [`Inherits`] for a trait bound.
    fn inherits<U: GodotClass>() -> bool {
        if Self::class_name() == U::class_name() {
            true
        } else if Self::Base::class_name() == <()>::class_name() {
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
    const INIT_LEVEL: Option<InitLevel> = None;

    fn class_name() -> ClassName {
        ClassName::none()
    }
}

/// Trait to create more references from a smart pointer or collection.
pub trait Share {
    /// Creates a new reference that points to the same object.
    ///
    /// If the referred-to object is reference-counted, this will increment the count.
    #[deprecated = "Replaced with `Clone::clone()`."]
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

/// Trait implemented for all objects that inherit from `Resource` or `Node`.
///
/// Those are the only objects you can export to the editor.
pub trait ExportableObject: GodotClass {}

/// Implemented for all user-defined classes, providing extensions on the raw object to interact with `Gd`.
pub trait UserClass: GodotClass<Declarer = dom::UserDomain> {
    /// Return a new Gd which contains a default-constructed instance.
    ///
    /// `MyClass::new_gd()` is equivalent to `Gd::<MyClass>::default()`.
    fn new_gd() -> Gd<Self>
    where
        Self: cap::GodotDefault + GodotClass<Mem = mem::StaticRefCount>,
    {
        Gd::default()
    }

    /// Return a new Gd which contains a default-constructed instance.
    ///
    /// `MyClass::new_gd()` is equivalent to `Gd::<MyClass>::default()`.
    #[must_use]
    fn alloc_gd<U>() -> Gd<Self>
    where
        Self: cap::GodotDefault + GodotClass<Mem = U>,
        U: mem::PossiblyManual,
    {
        Gd::default_instance()
    }

    #[doc(hidden)]
    fn __config() -> crate::private::ClassConfig;

    #[doc(hidden)]
    fn __before_ready(&mut self);

    #[doc(hidden)]
    fn __default_virtual_call(_method_name: &str) -> sys::GDExtensionClassCallVirtual {
        None
    }
}

/// Auto-implemented for all engine-provided classes.
pub trait EngineClass: GodotClass {
    fn as_object_ptr(&self) -> sys::GDExtensionObjectPtr;
    fn as_type_ptr(&self) -> sys::GDExtensionTypePtr;
}

/// Auto-implemented for all engine-provided enums.
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

/// Auto-implemented for all engine-provided bitfields.
pub trait EngineBitfield: Copy {
    fn try_from_ord(ord: u64) -> Option<Self>;

    /// Ordinal value of the bit flag, as specified in Godot.
    fn ord(self) -> u64;

    fn from_ord(ord: u64) -> Self {
        Self::try_from_ord(ord)
            .unwrap_or_else(|| panic!("ordinal {ord} does not map to any valid bit flag"))
    }
}

/// Trait for enums that can be used as indices in arrays.
///
/// The conditions for a Godot enum to be "index-like" are:
/// - Contains an enumerator ending in `_MAX`, which has the highest ordinal (denotes the size).
/// - All other enumerators are consecutive integers inside 0..max (no negative ordinals, no gaps).
///
/// Duplicates are explicitly allowed, to allow for renamings/deprecations. The order in which Godot exposes
/// the enumerators in the JSON is irrelevant.
pub trait IndexEnum: EngineEnum {
    /// Number of **distinct** enumerators in the enum.
    ///
    /// All enumerators are guaranteed to be in the range `0..ENUMERATOR_COUNT`, so you can use them
    /// as indices in an array of size `ENUMERATOR_COUNT`.
    ///
    /// Keep in mind that two enumerators with the same ordinal are only counted once.
    const ENUMERATOR_COUNT: usize;

    /// Converts the enumerator to `usize`, which can be used as an array index.
    ///
    /// Note that two enumerators may have the same index, if they have the same ordinal.
    fn to_index(self) -> usize {
        self.ord() as usize
    }
}

/// Trait that's implemented for user-defined classes that provide a `#[base]` field.
///
/// Gives direct access to the containing `Gd<Self>` from `Self`.
// Possible alternative for builder APIs, although even less ergonomic: Base<T> could be Base<T, Self> and return Gd<Self>.
pub trait WithBaseField: GodotClass<Declarer = dom::UserDomain> {
    /// Returns the `Gd` pointer containing this object.
    ///
    /// This is intended to be stored or passed to engine methods. You cannot call `bind()` or `bind_mut()` on it, while the method
    /// calling `to_gd()` is still running; that would lead to a double borrow panic.
    fn to_gd(&self) -> Gd<Self>;

    /// Returns a reference to the `Base` stored by this object.
    fn base_field(&self) -> &Base<Self::Base>;

    /// Returns a shared reference suitable for calling engine methods on this object.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use godot::prelude::*;
    ///
    /// #[derive(GodotClass)]
    /// #[class(init, base = Node)]
    /// struct MyClass {
    ///     #[base]
    ///     base: Base<Node>,
    /// }
    ///
    /// #[godot_api]
    /// impl INode for MyClass {
    ///     fn process(&mut self, _delta: f64) {
    ///         let name = self.base().get_name();
    ///         godot_print!("name is {name}");
    ///     }
    /// }
    ///
    /// # pub struct Test;
    ///
    /// # #[gdextension]
    /// # unsafe impl ExtensionLibrary for Test {}
    /// ```
    ///
    /// However we cannot call methods that require `&mut Base`, such as
    /// [`Node::add_child()`](crate::engine::Node::add_child).
    ///
    /// ```compile_fail
    /// use godot::prelude::*;
    ///
    /// #[derive(GodotClass)]
    /// #[class(init, base = Node)]
    /// struct MyClass {
    ///     #[base]
    ///     base: Base<Node>,
    /// }
    ///
    /// #[godot_api]
    /// impl INode for MyClass {
    ///     fn process(&mut self, _delta: f64) {
    ///         let node = Node::new_alloc();
    ///         // fails because `add_child` requires a mutable reference.
    ///         self.base().add_child(node);
    ///     }
    /// }
    ///
    /// # pub struct Test;
    ///
    /// # #[gdextension]
    /// # unsafe impl ExtensionLibrary for Test {}
    /// ```
    ///
    /// For this, use [`base_mut()`](WithBaseField::base_mut()) instead.
    fn base(&self) -> BaseRef<'_, Self> {
        let gd = self.base_field().to_gd();

        BaseRef::new(gd, self)
    }

    /// Returns a mutable reference suitable for calling engine methods on this object.
    ///
    /// This method will allow you to call back into the same object from Godot, unlike what would happen
    /// if you used [`to_gd()`](WithBaseField::to_gd).
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use godot::prelude::*;
    ///
    /// #[derive(GodotClass)]
    /// #[class(init, base = Node)]
    /// struct MyClass {
    ///     #[base]
    ///     base: Base<Node>,
    /// }
    ///
    /// #[godot_api]
    /// impl INode for MyClass {
    ///     fn process(&mut self, _delta: f64) {
    ///         let node = Node::new_alloc();
    ///         self.base_mut().add_child(node);
    ///     }
    /// }
    ///
    /// # pub struct Test;
    ///
    /// # #[gdextension]
    /// # unsafe impl ExtensionLibrary for Test {}
    /// ```
    ///
    /// We can call back into `self` through Godot:
    ///
    /// ```
    /// use godot::prelude::*;
    ///
    /// #[derive(GodotClass)]
    /// #[class(init, base = Node)]
    /// struct MyClass {
    ///     #[base]
    ///     base: Base<Node>,
    /// }
    ///
    /// #[godot_api]
    /// impl INode for MyClass {
    ///     fn process(&mut self, _delta: f64) {
    ///         self.base_mut().call("other_method".into(), &[]);
    ///     }
    /// }
    ///
    /// #[godot_api]
    /// impl MyClass {
    ///     #[func]
    ///     fn other_method(&mut self) {}
    /// }
    ///
    /// # pub struct Test;
    ///
    /// # #[gdextension]
    /// # unsafe impl ExtensionLibrary for Test {}
    /// ```
    #[allow(clippy::let_unit_value)]
    fn base_mut(&mut self) -> BaseMut<'_, Self> {
        let base_gd = self.base_field().to_gd();

        let gd = self.to_gd();
        // SAFETY:
        // - We have a `Gd<Self>` so, provided that `storage_unbounded` succeeds, the associated instance
        //   storage has been created.
        //
        // - Since we can get a `&'a Base<Self::Base>` from `&'a self`, that must mean we have a Rust object
        //   somewhere that has this base object. The only way to have such a base object is by being the
        //   Rust object referenced by that base object. I.e this storage's user-instance is that Rust
        //   object. That means this storage cannot be destroyed for the lifetime of that Rust object. And
        //   since we have a reference to the base object derived from that Rust object, then that Rust
        //   object must outlive `'a`. And so the storage cannot be destroyed during the lifetime `'a`.
        let storage = unsafe {
            gd.raw
                .storage_unbounded()
                .expect("we have a `Gd<Self>` so the raw should not be null")
        };

        let guard = storage.get_base_mut(self);

        BaseMut::new(base_gd, guard)
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

/// Capability traits, providing dedicated functionalities for Godot classes
pub mod cap {
    use super::*;
    use crate::obj::{Base, Gd};

    /// Trait for all classes that are default-constructible from the Godot engine.
    ///
    /// Enables the `MyClass.new()` syntax in GDScript, and allows the type to be used by the editor, which often default-constructs objects.
    ///
    /// This trait is automatically implemented for the following classes:
    /// - User defined classes if either:
    ///   - they override an `init()` method
    ///   - they have `#[class(init)]` attribute
    /// - Engine classes if:
    ///   - they are reference-counted and constructible (i.e. provide a `new()` method).
    ///
    /// This trait is not manually implemented, and you cannot call any methods. You can use it as a bound, but typically you'd use
    /// it indirectly through [`Gd::default()`][crate::obj::Gd::default()]. Note that `Gd::default()` has an additional requirement on
    /// being reference-counted, meaning not every `GodotDefault` class can automatically be used with `Gd::default()`.
    pub trait GodotDefault: GodotClass {
        /// Provides a default smart pointer instance.
        ///
        /// Semantics:
        /// - For user-defined classes, this calls `T::init()` or the generated init-constructor.
        /// - For engine classes, this calls `T::new()`.
        #[doc(hidden)]
        fn __godot_default() -> Gd<Self> {
            // This is a bit hackish, but the alternatives are:
            // 1. Separate trait `GodotUserDefault` for user classes, which then proliferates through all APIs and makes abstraction harder.
            // 2. Repeatedly implementing __godot_default() that forwards to something like Gd::default_user_instance(). Possible, but this
            //    will make the step toward builder APIs more difficult, as users would need to re-implement this as well.
            debug_assert_eq!(
                std::any::TypeId::of::<<Self as GodotClass>::Declarer>(),
                std::any::TypeId::of::<dom::UserDomain>(),
                "__godot_default() called on engine class; must be overridden for user classes"
            );

            Gd::default_instance()
        }

        /// Only provided for user classes.
        #[doc(hidden)]
        fn __godot_user_init(_base: Base<Self::Base>) -> Self {
            unreachable!(
                "__godot_user_init() called on engine class; must be overridden for user classes"
            )
        }
    }

    // TODO Evaluate whether we want this public or not
    #[doc(hidden)]
    pub trait GodotToString: GodotClass {
        #[doc(hidden)]
        fn __godot_to_string(&self) -> GString;
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
    use crate::{
        obj::{GodotClass, RawGd},
        storage::Storage,
    };

    /// Trait that specifies who declares a given `GodotClass`.
    pub trait Domain: Sealed {
        type DerefTarget<T: GodotClass>;

        #[doc(hidden)]
        fn scoped_mut<T, F, R>(obj: &mut RawGd<T>, closure: F) -> R
        where
            T: GodotClass<Declarer = Self>,
            F: FnOnce(&mut T) -> R;

        /// Check if the object is a user object *and* currently locked by a `bind()` or `bind_mut()` guard.
        ///
        /// # Safety
        /// Object must be alive.
        #[doc(hidden)]
        unsafe fn is_currently_bound<T>(obj: &RawGd<T>) -> bool
        where
            T: GodotClass<Declarer = Self>;
    }

    /// Expresses that a class is declared by the Godot engine.
    pub enum EngineDomain {}
    impl Sealed for EngineDomain {}
    impl Domain for EngineDomain {
        type DerefTarget<T: GodotClass> = T;

        fn scoped_mut<T, F, R>(obj: &mut RawGd<T>, closure: F) -> R
        where
            T: GodotClass<Declarer = EngineDomain>,
            F: FnOnce(&mut T) -> R,
        {
            closure(
                obj.as_target_mut()
                    .expect("scoped mut should not be called on a null object"),
            )
        }

        unsafe fn is_currently_bound<T>(_obj: &RawGd<T>) -> bool
        where
            T: GodotClass<Declarer = Self>,
        {
            false
        }
    }

    /// Expresses that a class is declared by the user.
    pub enum UserDomain {}
    impl Sealed for UserDomain {}
    impl Domain for UserDomain {
        type DerefTarget<T: GodotClass> = T::Base;

        fn scoped_mut<T, F, R>(obj: &mut RawGd<T>, closure: F) -> R
        where
            T: GodotClass<Declarer = Self>,
            F: FnOnce(&mut T) -> R,
        {
            let mut guard = obj.bind_mut();
            closure(&mut *guard)
        }

        unsafe fn is_currently_bound<T>(obj: &RawGd<T>) -> bool
        where
            T: GodotClass<Declarer = Self>,
        {
            obj.storage().unwrap_unchecked().is_bound()
        }
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

pub mod mem {
    use godot_ffi::PtrcallType;

    use super::private::Sealed;
    use crate::obj::{GodotClass, RawGd};
    use crate::out;

    /// Specifies the memory
    pub trait Memory: Sealed {
        /// Initialize reference counter
        #[doc(hidden)]
        fn maybe_init_ref<T: GodotClass>(obj: &RawGd<T>);

        /// If ref-counted, then increment count
        #[doc(hidden)]
        fn maybe_inc_ref<T: GodotClass>(obj: &RawGd<T>);

        /// If ref-counted, then decrement count. Returns `true` if the count hit 0 and the object can be
        /// safely freed.
        ///
        /// This behavior can be overriden by a script, making it possible for the function to return `false`
        /// even when the reference count hits 0. This is meant to be used to have a separate reference count
        /// from Godot's internal reference count, or otherwise stop the object from being freed when the
        /// reference count hits 0.
        ///
        /// # Safety
        ///
        /// If this method is used on a [`Gd`] that inherits from [`RefCounted`](crate::engine::RefCounted)
        /// then the reference count must either be incremented before it hits 0, or some [`Gd`] referencing
        /// this object must be forgotten.
        #[doc(hidden)]
        unsafe fn maybe_dec_ref<T: GodotClass>(obj: &RawGd<T>) -> bool;

        /// Check if ref-counted, return `None` if information is not available (dynamic and obj dead)
        #[doc(hidden)]
        fn is_ref_counted<T: GodotClass>(obj: &RawGd<T>) -> Option<bool>;

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
        fn maybe_init_ref<T: GodotClass>(obj: &RawGd<T>) {
            out!("  Stat::init  <{}>", std::any::type_name::<T>());
            if obj.is_null() {
                return;
            }
            obj.as_ref_counted(|refc| {
                let success = refc.init_ref();
                assert!(success, "init_ref() failed");
            });
        }

        fn maybe_inc_ref<T: GodotClass>(obj: &RawGd<T>) {
            out!("  Stat::inc   <{}>", std::any::type_name::<T>());
            if obj.is_null() {
                return;
            }
            obj.as_ref_counted(|refc| {
                let success = refc.reference();
                assert!(success, "reference() failed");
            });
        }

        unsafe fn maybe_dec_ref<T: GodotClass>(obj: &RawGd<T>) -> bool {
            out!("  Stat::dec   <{}>", std::any::type_name::<T>());
            if obj.is_null() {
                return false;
            }
            obj.as_ref_counted(|refc| {
                let is_last = refc.unreference();
                out!("  +-- was last={is_last}");
                is_last
            })
        }

        fn is_ref_counted<T: GodotClass>(_obj: &RawGd<T>) -> Option<bool> {
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
        fn maybe_init_ref<T: GodotClass>(obj: &RawGd<T>) {
            out!("  Dyn::init  <{}>", std::any::type_name::<T>());
            if obj
                .instance_id_unchecked()
                .map(|id| id.is_ref_counted())
                .unwrap_or(false)
            {
                // Will call `RefCounted::init_ref()` which checks for liveness.
                StaticRefCount::maybe_init_ref(obj)
            }
        }

        fn maybe_inc_ref<T: GodotClass>(obj: &RawGd<T>) {
            out!("  Dyn::inc   <{}>", std::any::type_name::<T>());
            if obj
                .instance_id_unchecked()
                .map(|id| id.is_ref_counted())
                .unwrap_or(false)
            {
                // Will call `RefCounted::reference()` which checks for liveness.
                StaticRefCount::maybe_inc_ref(obj)
            }
        }

        unsafe fn maybe_dec_ref<T: GodotClass>(obj: &RawGd<T>) -> bool {
            out!("  Dyn::dec   <{}>", std::any::type_name::<T>());
            if obj
                .instance_id_unchecked()
                .map(|id| id.is_ref_counted())
                .unwrap_or(false)
            {
                // Will call `RefCounted::unreference()` which checks for liveness.
                StaticRefCount::maybe_dec_ref(obj)
            } else {
                false
            }
        }

        fn is_ref_counted<T: GodotClass>(obj: &RawGd<T>) -> Option<bool> {
            // Return `None` if obj is dead
            obj.instance_id_unchecked().map(|id| id.is_ref_counted())
        }
    }

    impl PossiblyManual for DynamicRefCount {}

    /// No memory management, user responsible for not leaking.
    /// This is used for all `Object` derivates, which are not `RefCounted`. `Object` itself is also excluded.
    pub struct ManualMemory {}
    impl Sealed for ManualMemory {}
    impl Memory for ManualMemory {
        fn maybe_init_ref<T: GodotClass>(_obj: &RawGd<T>) {}
        fn maybe_inc_ref<T: GodotClass>(_obj: &RawGd<T>) {}
        unsafe fn maybe_dec_ref<T: GodotClass>(_obj: &RawGd<T>) -> bool {
            false
        }
        fn is_ref_counted<T: GodotClass>(_obj: &RawGd<T>) -> Option<bool> {
            Some(false)
        }
    }
    impl PossiblyManual for ManualMemory {}
}
