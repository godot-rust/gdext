/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use godot_ffi as sys;

use crate::builder::ClassBuilder;
use crate::builtin::GString;
use crate::init::InitLevel;
use crate::meta::inspect::EnumConstant;
use crate::meta::ClassName;
use crate::obj::{bounds, Base, BaseMut, BaseRef, Bounds, Gd};
#[cfg(since_api = "4.2")]
use crate::registry::signal::SignalObject;
use crate::storage::Storage;

/// Makes `T` eligible to be managed by Godot and stored in [`Gd<T>`][crate::obj::Gd] pointers.
///
/// The behavior of types implementing this trait is influenced by the associated types; check their documentation for information.
///
/// Normally, you don't need to implement this trait yourself; use [`#[derive(GodotClass)]`](../register/derive.GodotClass.html) instead.
// Above intra-doc link to the derive-macro only works as HTML, not as symbol link.
#[diagnostic::on_unimplemented(
    message = "only classes registered with Godot are allowed in this context",
    note = "you can use `#[derive(GodotClass)]` to register your own structs with Godot",
    note = "see also: https://godot-rust.github.io/book/register/classes.html"
)]
pub trait GodotClass: Bounds + 'static
where
    Self: Sized,
{
    /// The immediate superclass of `T`. This is always a Godot engine class.
    type Base: GodotClass; // not EngineClass because it can be ()

    /// The name of the class, under which it is registered in Godot.
    ///
    /// This may deviate from the Rust struct name: `HttpRequest::class_name().as_str() == "HTTPRequest"`.
    fn class_name() -> ClassName;

    /// Initialization level, during which this class should be initialized with Godot.
    ///
    /// The default is a good choice in most cases; override only if you have very specific initialization requirements.
    /// It must not be less than `Base::INIT_LEVEL`.
    const INIT_LEVEL: InitLevel = <Self::Base as GodotClass>::INIT_LEVEL;

    /// Returns whether `Self` inherits from `U`.
    ///
    /// This is reflexive, i.e `Self` inherits from itself.
    ///
    /// See also [`Inherits`] for a trait bound.
    fn inherits<U: GodotClass>() -> bool {
        if Self::class_name() == U::class_name() {
            true
        } else if Self::Base::class_name() == <NoBase>::class_name() {
            false
        } else {
            Self::Base::inherits::<U>()
        }
    }
}

/// Type representing the absence of a base class, at the root of the hierarchy.
///
/// `NoBase` is used as the base class for exactly one class: [`Object`][crate::classes::Object].
///
/// This is an enum without any variants, as we should never construct an instance of this class.
pub enum NoBase {}

impl GodotClass for NoBase {
    type Base = NoBase;

    fn class_name() -> ClassName {
        ClassName::none()
    }

    const INIT_LEVEL: InitLevel = InitLevel::Core; // arbitrary; never read.
}

unsafe impl Bounds for NoBase {
    type Memory = bounds::MemManual;
    type DynMemory = bounds::MemManual;
    type Declarer = bounds::DeclEngine;
    type Exportable = bounds::No;
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
/// # Safety
///
/// This trait must only be implemented for subclasses of `Base`.
///
/// Importantly, this means it is always safe to upcast a value of type `Gd<Self>` to `Gd<Base>`.
pub unsafe trait Inherits<Base: GodotClass>: GodotClass {}

// SAFETY: Every class is a subclass of itself.
unsafe impl<T: GodotClass> Inherits<T> for T {}

/// Trait that defines a `T` -> `dyn Trait` relation for use in [`DynGd`][crate::obj::DynGd].
///
/// You should typically not implement this manually, but use the [`#[godot_dyn]`](../register/attr.godot_dyn.html) macro.
#[diagnostic::on_unimplemented(
    message = "`{Trait}` needs to be a trait object linked with class `{Self}` in the library",
    note = "you can use `#[godot_dyn]` on `impl Trait for Class` to auto-generate `impl Implements<dyn Trait> for Class`"
)]
// Note: technically, `Trait` doesn't _have to_ implement `Self`. The Rust type system provides no way to verify that a) D is a trait object,
// and b) that the trait behind it is implemented for the class. Thus, users could any another reference type, such as `&str` pointing to a field.
// This should be safe, since lifetimes are checked throughout and the class instance remains in place (pinned) inside a DynGd.
pub trait AsDyn<Trait>: GodotClass
where
    Trait: ?Sized + 'static,
{
    fn dyn_upcast(&self) -> &Trait;
    fn dyn_upcast_mut(&mut self) -> &mut Trait;
}

/// Implemented for all user-defined classes, providing extensions on the raw object to interact with `Gd`.
#[doc(hidden)]
pub trait UserClass: Bounds<Declarer = bounds::DeclUser> {
    #[doc(hidden)]
    fn __config() -> crate::private::ClassConfig;

    #[doc(hidden)]
    fn __before_ready(&mut self);

    #[doc(hidden)]
    fn __default_virtual_call(
        _method_name: &str,
        #[cfg(since_api = "4.4")] _hash: u32,
    ) -> sys::GDExtensionClassCallVirtual {
        None
    }
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

    /// The name of the enumerator, as it appears in Rust.
    ///
    /// Note that **this may not match the Rust constant name.** In case of multiple constants with the same ordinal value, this method returns
    /// the first one in the order of definition. For example, [`LayoutDirection::LOCALE.as_str()`][crate::classes::window::LayoutDirection::LOCALE]
    /// (ord 1) returns `"APPLICATION_LOCALE"`, because that happens to be the first constant with ordinal `1`.
    /// See [`all_constants()`][Self::all_constants] for a more robust and general approach to introspection of enum constants.
    ///
    /// If the value does not match one of the known enumerators, the empty string is returned.
    fn as_str(&self) -> &'static str;

    /// The equivalent name of the enumerator, as specified in Godot.
    ///
    /// If the value does not match one of the known enumerators, the empty string is returned.
    ///
    /// # Deprecation
    /// Design change is due to the fact that Godot enums may have multiple constants with the same ordinal value, and `godot_name()` cannot
    /// always return a unique name for it. So there are cases where this method returns unexpected results.
    ///
    /// To keep the old -- possibly incorrect -- behavior, you can write the following function. However, it runs in linear rather than constant
    /// time (which is often OK, given that there are very few constants per enum).
    /// ```
    /// use godot::obj::EngineEnum;
    ///
    /// fn godot_name<T: EngineEnum + Eq + PartialEq + 'static>(value: T) -> &'static str {
    ///     T::all_constants()
    ///         .iter()
    ///         .find(|c| c.value() == value)
    ///         .map(|c| c.godot_name())
    ///         .unwrap_or("") // Previous behavior.
    /// }
    /// ```
    #[deprecated = "Moved to introspection API, see `EngineEnum::all_constants()` and `EnumConstant::godot_name()`"]
    fn godot_name(&self) -> &'static str;

    /// Returns a slice of distinct enum values.
    ///
    /// This excludes `MAX` constants at the end (existing only to express the number of enumerators) and deduplicates aliases,
    /// providing only meaningful enum values. See [`all_constants()`][Self::all_constants] for a complete list of all constants.
    ///
    /// Enables iteration over distinct enum variants:
    /// ```no_run
    /// use godot::classes::window;
    /// use godot::obj::EngineEnum;
    ///
    /// for mode in window::Mode::values() {
    ///     println!("* {}: {}", mode.as_str(), mode.ord());
    /// }
    /// ```
    fn values() -> &'static [Self];

    /// Returns metadata for all enum constants.
    ///
    /// This includes all constants as they appear in the enum definition, including duplicates and `MAX` constants.
    /// For a list of useful, distinct values, use [`values()`][Self::values].
    ///
    /// Enables introspection of available constants:
    /// ```no_run
    /// use godot::classes::window;
    /// use godot::obj::EngineEnum;
    ///
    /// for constant in window::Mode::all_constants() {
    ///     println!("* window::Mode.{} (original {}) has ordinal value {}.",
    ///         constant.rust_name(),
    ///         constant.godot_name(),
    ///         constant.value().ord()
    ///     );
    /// }
    /// ```
    fn all_constants() -> &'static [EnumConstant<Self>];
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

    // TODO consolidate API: named methods vs. | & ! etc.
    fn is_set(self, flag: Self) -> bool {
        self.ord() & flag.ord() != 0
    }

    /// Returns metadata for all bitfield constants.
    ///
    /// This includes all constants as they appear in the bitfield definition.
    ///
    /// Enables introspection of available constants:
    /// ```no_run
    /// use godot::global::KeyModifierMask;
    /// use godot::obj::EngineBitfield;
    ///
    /// for constant in KeyModifierMask::all_constants() {
    ///     println!("* KeyModifierMask.{} (original {}) has ordinal value {}.",
    ///         constant.rust_name(),
    ///         constant.godot_name(),
    ///         constant.value().ord()
    ///     );
    /// }
    /// ```
    fn all_constants() -> &'static [EnumConstant<Self>];
}

/// Trait for enums that can be used as indices in arrays.
///
/// The conditions for a Godot enum to be "index-like" are:
/// - Contains an enumerator ending in `_MAX`, which has the highest ordinal (denotes the size).
/// - All other enumerators are consecutive integers inside `0..max` (no negative ordinals, no gaps).
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

/// Trait that is automatically implemented for user classes containing a `Base<T>` field.
///
/// Gives direct access to the containing `Gd<Self>` from `self`.
///
/// # Usage as a bound
///
/// In order to call `base()` or `base_mut()` within a function or on a type you define, you need a `WithBaseField<Base = T>` bound,
/// where `T` is the base class of your type.
///
/// ```no_run
/// # use godot::prelude::*;
/// # use godot::obj::WithBaseField;
/// fn some_fn<T>(value: &T)
/// where
///     T: WithBaseField<Base = Node3D>,
/// {
///     let base = value.base();
///     let pos = base.get_position();
/// }
/// ```
///
// Possible alternative for builder APIs, although even less ergonomic: Base<T> could be Base<T, Self> and return Gd<Self>.
#[diagnostic::on_unimplemented(
    message = "Class `{Self}` requires a `Base<T>` field",
    label = "missing field `_base: Base<...>` in struct declaration",
    note = "a base field is required to access the base from within `self`, as well as for #[signal], #[rpc] and #[func(virtual)]",
    note = "see also: https://godot-rust.github.io/book/register/classes.html#the-base-field"
)]
pub trait WithBaseField: GodotClass + Bounds<Declarer = bounds::DeclUser> {
    /// Returns the `Gd` pointer containing this object.
    ///
    /// This is intended to be stored or passed to engine methods. You cannot call `bind()` or `bind_mut()` on it, while the method
    /// calling `to_gd()` is still running; that would lead to a double borrow panic.
    ///
    /// # Panics
    /// If called during initialization (the `init()` function or `Gd::from_init_fn()`). Use [`Base::to_init_gd()`] instead.
    fn to_gd(&self) -> Gd<Self>;

    /// Returns a reference to the `Base` stored by this object.
    #[doc(hidden)]
    fn base_field(&self) -> &Base<Self::Base>;

    /// Returns a shared reference suitable for calling engine methods on this object.
    ///
    /// Holding a shared guard prevents other code paths from obtaining a _mutable_ reference to `self`, as such it is recommended to drop the
    /// guard as soon as you no longer need it.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use godot::prelude::*;
    ///
    /// #[derive(GodotClass)]
    /// #[class(init, base = Node)]
    /// struct MyClass {
    ///     base: Base<Node>,
    /// }
    ///
    /// #[godot_api]
    /// impl INode for MyClass {
    ///     fn process(&mut self, _delta: f32) {
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
    /// However, we cannot call methods that require `&mut Base`, such as
    /// [`Node::add_child()`](crate::classes::Node::add_child).
    ///
    /// ```compile_fail
    /// use godot::prelude::*;
    ///
    /// #[derive(GodotClass)]
    /// #[class(init, base = Node)]
    /// struct MyClass {
    ///     ///     base: Base<Node>,
    /// }
    ///
    /// #[godot_api]
    /// impl INode for MyClass {
    ///     fn process(&mut self, _delta: f32) {
    ///         let node = Node::new_alloc();
    ///         // fails because `add_child` requires a mutable reference.
    ///         self.base().add_child(&node);
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
        let gd = self.base_field().__constructed_gd();

        BaseRef::new(gd, self)
    }

    /// Returns a mutable reference suitable for calling engine methods on this object.
    ///
    /// This method will allow you to call back into the same object from Godot, unlike what would happen
    /// if you used [`to_gd()`](WithBaseField::to_gd). You have to keep the `BaseRef` guard bound for the entire duration the engine might
    /// re-enter a function of your class. The guard temporarily absorbs the `&mut self` reference, which allows for an additional mutable
    /// reference to be acquired.
    ///
    /// Holding a mutable guard prevents other code paths from obtaining _any_ reference to `self`, as such it is recommended to drop the
    /// guard as soon as you no longer need it.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use godot::prelude::*;
    ///
    /// #[derive(GodotClass)]
    /// #[class(init, base = Node)]
    /// struct MyClass {
    ///     base: Base<Node>,
    /// }
    ///
    /// #[godot_api]
    /// impl INode for MyClass {
    ///     fn process(&mut self, _delta: f32) {
    ///         let node = Node::new_alloc();
    ///         self.base_mut().add_child(&node);
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
    ///     base: Base<Node>,
    /// }
    ///
    /// #[godot_api]
    /// impl INode for MyClass {
    ///     fn process(&mut self, _delta: f32) {
    ///         self.base_mut().call("other_method", &[]);
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
        let base_gd = self.base_field().__constructed_gd();

        let gd = self.to_gd();
        // SAFETY:
        // - We have a `Gd<Self>` so, provided that `storage_unbounded` succeeds, the associated instance
        //   storage has been created.
        //
        // - Since we can get a `&'a Base<Self::Base>` from `&'a self`, that must mean we have a Rust object
        //   somewhere that has this base object. The only way to have such a base object is by being the
        //   Rust object referenced by that base object. I.e. this storage's user-instance is that Rust
        //   object. That means this storage cannot be destroyed for the lifetime of that Rust object. And
        //   since we have a reference to the base object derived from that Rust object, then that Rust
        //   object must outlive `'a`. And so the storage cannot be destroyed during the lifetime `'a`.
        let storage = unsafe {
            gd.raw
                .storage_unbounded()
                .expect("we have a `Gd<Self>` so the raw should not be null")
        };

        let guard = storage.get_inaccessible(self);

        BaseMut::new(base_gd, guard)
    }
}

// Dummy traits to still allow bounds and imports.
#[cfg(before_api = "4.2")]
pub trait WithSignals: GodotClass {}
#[cfg(before_api = "4.2")]
pub trait WithUserSignals: WithSignals + WithBaseField {}

/// Implemented for all classes with registered signals, both engine- and user-declared.
///
/// This trait enables the [`Gd::signals()`] method.
///
/// User-defined classes with `#[signal]` additionally implement [`WithUserSignals`].
#[cfg(since_api = "4.2")]
// Inherits bound makes some up/downcasting in signals impl easier.
pub trait WithSignals: GodotClass + Inherits<crate::classes::Object> {
    /// The associated struct listing all signals of this class.
    ///
    /// Parameters:
    /// - `'c` denotes the lifetime during which the class instance is borrowed and its signals can be modified.
    /// - `C` is the concrete class on which the signals are provided. This can be different than `Self` in case of derived classes
    ///   (e.g. a user-defined node) connecting/emitting signals of a base class (e.g. `Node`).
    type SignalCollection<'c, C>
    where
        C: WithSignals;

    /// Whether the representation needs to be able to hold just `Gd` (for engine classes) or `UserSignalObject` (for user classes).
    // Note: this cannot be in Declarer (Engine/UserDecl) as associated type `type SignalObjectType<'c, T: WithSignals>`,
    // because the user impl has the additional requirement T: WithUserSignals.
    #[doc(hidden)]
    type __SignalObj<'c>: SignalObject<'c>;
    // type __SignalObj<'c, C>: SignalObject<'c>
    // where
    //     C: WithSignals + 'c;

    /// Create from existing `Gd`, to enable `Gd::signals()`.
    ///
    /// Only used for constructing from a concrete class, so `C = Self` in the return type.
    ///
    /// Takes by reference and not value, to retain lifetime chain.
    #[doc(hidden)]
    fn __signals_from_external(external: &Gd<Self>) -> Self::SignalCollection<'_, Self>;
}

/// Implemented for user-defined classes with at least one `#[signal]` declaration.
///
/// Allows to access signals from within the class, as `self.signals()`. This requires a `Base<T>` field.
#[cfg(since_api = "4.2")]
pub trait WithUserSignals: WithSignals + WithBaseField {
    /// Access user-defined signals of the current object `self`.
    ///
    /// For classes that have at least one `#[signal]` defined, returns a collection of signal names. Each returned signal has a specialized
    /// API for connecting and emitting signals in a type-safe way. If you need to access signals from outside (given a `Gd` pointer), use
    /// [`Gd::signals()`] instead.
    ///
    /// If you haven't already, read the [book chapter about signals](https://godot-rust.github.io/book/register/signals.html) for a
    /// walkthrough.
    ///
    /// # Provided API
    ///
    /// The returned collection provides a method for each signal, with the same name as the corresponding `#[signal]`.  \
    /// For example, if you have...
    /// ```ignore
    /// #[signal]
    /// fn damage_taken(&mut self, amount: i32);
    /// ```
    /// ...then you can access the signal as `self.signals().damage_taken()`, which returns an object with the following API:
    /// ```ignore
    /// // Connects global or associated function, or a closure.
    /// fn connect(f: impl FnMut(i32));
    ///
    /// // Connects a &mut self method or closure on the emitter object.
    /// fn connect_self(f: impl FnMut(&mut Self, i32));
    ///
    /// // Connects a &mut self method or closure on another object.
    /// fn connect_other<C>(f: impl FnMut(&mut C, i32));
    ///
    /// // Emits the signal with the given arguments.
    /// fn emit(amount: i32);
    /// ```
    ///
    /// See [`TypedSignal`](crate::registry::signal::TypedSignal) for more information.
    fn signals(&mut self) -> Self::SignalCollection<'_, Self>;
}

/// Extension trait for all reference-counted classes.
pub trait NewGd: GodotClass {
    /// Return a new, ref-counted `Gd` containing a default-constructed instance.
    ///
    /// `MyClass::new_gd()` is equivalent to `Gd::<MyClass>::default()`.
    ///
    /// # Panics
    /// If `Self` is user-defined and its default constructor `init()` panics, that panic is propagated.
    fn new_gd() -> Gd<Self>;
}

impl<T> NewGd for T
where
    T: cap::GodotDefault + Bounds<Memory = bounds::MemRefCounted>,
{
    fn new_gd() -> Gd<Self> {
        Gd::default()
    }
}

/// Extension trait for all manually managed classes.
pub trait NewAlloc: GodotClass {
    /// Return a new, manually-managed `Gd` containing a default-constructed instance.
    ///
    /// The result must be manually managed, e.g. by attaching it to the scene tree or calling `free()` after usage.
    /// Failure to do so will result in memory leaks.
    ///
    /// # Panics
    /// If `Self` is user-defined and its default constructor `init()` panics, that panic is propagated to the caller.
    #[must_use]
    fn new_alloc() -> Gd<Self>;
}

impl<T> NewAlloc for T
where
    T: cap::GodotDefault + Bounds<Memory = bounds::MemManual>,
{
    fn new_alloc() -> Gd<Self> {
        use crate::obj::bounds::Declarer as _;

        <Self as Bounds>::Declarer::create_gd()
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

/// Capability traits, providing dedicated functionalities for Godot classes
pub mod cap {
    use std::any::Any;

    use super::*;
    use crate::builtin::{StringName, Variant};
    use crate::meta::PropertyInfo;
    use crate::obj::{Base, Bounds, Gd};

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
    #[diagnostic::on_unimplemented(
        message = "Class `{Self}` requires either an `init` constructor, or explicit opt-out",
        label = "needs `init`",
        note = "to provide a default constructor, use `#[class(init)]` or implement an `init` method",
        note = "to opt out, use `#[class(no_init)]`",
        note = "see also: https://godot-rust.github.io/book/register/constructors.html"
    )]
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
                std::any::TypeId::of::<<Self as Bounds>::Declarer>(),
                std::any::TypeId::of::<bounds::DeclUser>(),
                "__godot_default() called on engine class; must be overridden for engine classes"
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

    #[doc(hidden)]
    pub trait GodotGet: GodotClass {
        #[doc(hidden)]
        fn __godot_get_property(&self, property: StringName) -> Option<Variant>;
    }

    #[doc(hidden)]
    pub trait GodotSet: GodotClass {
        #[doc(hidden)]
        fn __godot_set_property(&mut self, property: StringName, value: Variant) -> bool;
    }

    #[doc(hidden)]
    pub trait GodotGetPropertyList: GodotClass {
        #[doc(hidden)]
        fn __godot_get_property_list(&mut self) -> Vec<crate::meta::PropertyInfo>;
    }

    #[doc(hidden)]
    pub trait GodotPropertyGetRevert: GodotClass {
        #[doc(hidden)]
        fn __godot_property_get_revert(&self, property: StringName) -> Option<Variant>;
    }

    #[doc(hidden)]
    #[cfg(since_api = "4.2")]
    pub trait GodotValidateProperty: GodotClass {
        #[doc(hidden)]
        fn __godot_validate_property(&self, property: &mut PropertyInfo);
    }

    /// Auto-implemented for `#[godot_api] impl MyClass` blocks
    pub trait ImplementsGodotApi: GodotClass {
        #[doc(hidden)]
        fn __register_methods();
        #[doc(hidden)]
        fn __register_constants();
        #[doc(hidden)]
        fn __register_rpcs(_: &mut dyn Any) {}
    }

    pub trait ImplementsGodotExports: GodotClass {
        #[doc(hidden)]
        fn __register_exports();
    }

    /// Auto-implemented for `#[godot_api] impl XyVirtual for MyClass` blocks
    pub trait ImplementsGodotVirtual: GodotClass {
        // Cannot use #[cfg(since_api = "4.4")] on the `hash` parameter, because the doc-postprocessing generates #[doc(cfg)],
        // which isn't valid in parameter position.

        #[cfg(before_api = "4.4")]
        #[doc(hidden)]
        fn __virtual_call(name: &str) -> sys::GDExtensionClassCallVirtual;

        #[cfg(since_api = "4.4")]
        #[doc(hidden)]
        fn __virtual_call(name: &str, hash: u32) -> sys::GDExtensionClassCallVirtual;
    }
}
