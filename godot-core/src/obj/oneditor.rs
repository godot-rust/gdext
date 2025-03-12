/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::meta::{FromGodot, GodotConvert, GodotType, PropertyHintInfo};
use crate::registry::property::{BuiltinExport, Export, Var};

/// Exported property that must be initialized in the editor (or associated code) before use.
///
/// Allows to use `Gd<T>`, which by itself never holds null objects, as an `#[export]` that should not be null during runtime.
/// As such, it can be used as a more ergonomic version of `Option<Gd<T>>` which _assumes_ initialization.
///
/// Panics during access if uninitialized.
/// When used inside a node class, `OnEditor` checks if a value has been set before `ready()` is run, and panics otherwise.
/// This validation is performed for all `OnEditor` fields declared in a given `GodotClass`, regardless of whether they are `#[var]`, `#[export]`, or neither.
/// Once initialized, it can be used almost as if it was a `T` value itself, due to `Deref`/`DerefMut` impls.
///
/// `OnEditor<T>` should always be used as a property, preferably in tandem with an `#[export]` or `#[var]`.
/// Initializing `OnEditor` values via code before the first use is supported but should be limited to use cases involving builder or factory patterns.
///
/// [`Option<Gd<T>>`](std::option) and [`OnReady<Gd<T>>`](crate::obj::onready::OnReady) should be used for any other late initialization logic.
///
/// # Using `OnEditor<T>` with `Gd<T>` and `DynGd<T, D>`
///
/// ## Example - auto-generated init
///
/// ```
///  use godot::prelude::*;
///
/// #[derive(GodotClass)]
/// #[class(init, base = Node)]
/// struct MyClass {
///     #[export]
///     editor_property: OnEditor<Gd<Resource>>,
/// }
///
/// #[godot_api]
/// impl INode for MyClass {
///     fn ready(&mut self) {
///         // Will always be valid and **must** be set via editor.
///         // Additional check is being run before ready()
///         // to ensure that given value can't be null.
///         let some_variant = self.editor_property.get_meta("SomeName");
///     }
/// }
///
/// ```
///
/// ## Example - user-generated init
///
/// Uninitialized `OnEditor<Gd<T>>` and `OnEditor<DynGd<T, D>>` can be created with `OnEditor<...>::default()`.
///
/// ```
///  use godot::prelude::*;
///
/// #[derive(GodotClass)]
/// #[class(base = Node)]
/// struct MyClass {
///     #[export]
///     required_node: OnEditor<Gd<Node>>,
///
///     base: Base<Node>
/// }
///
/// #[godot_api]
/// impl INode for MyClass {
///     fn init(base: Base<Node>) -> Self {
///        Self {
///            base,
///            required_node: OnEditor::default(),
///        }
///     }
/// }
///```
///
/// ## Example - factory pattern
///
/// ```
/// use godot::prelude::*;
///
/// #[derive(GodotClass)]
/// #[class(init, base = Node)]
/// struct SomeClass {
///     #[export]
///     required_node: OnEditor<Gd<Node>>,
/// }
///
/// fn create_and_add(
///     mut this: Gd<Node>,
///     some_class_scene: Gd<PackedScene>,
///     some_node: Gd<Node>,
/// ) -> Gd<SomeClass> {
///     let mut my_node = some_class_scene.instantiate_as::<SomeClass>();
///
///     // Would cause the panic:
///     // this.add_child(&my_node);
///
///     // Note: Remember that nodes are manually managed.
///     // They will leak memory if not added to tree and/or pruned.
///     my_node.bind_mut().required_node.init(some_node);
///
///     // Will not cause the panic.
///     this.add_child(&my_node);
///
///     my_node
/// }
/// ```
///
/// # Using `OnEditor<T>` with other GodotTypes
///
/// `OnEditor<T>` can be used with other built-ins to provide extra validation logic and making sure that given properties has been set.
/// Example usage might be checking if entities has been granted properly generated id.
///
/// In such cases the value which will be deemed invalid **must** be specified with `#[init(uninit = val)]`.
/// Given `val` will be used to represent uninitialized `OnEditor<T>` in the Godot editor.
/// Accessing uninitialized value will cause the panic.
///
/// ## Example - using `OnEditor` with primitives
///
/// ```
///  use godot::prelude::*;
///
/// #[derive(GodotClass)]
/// #[class(init, base = Node)]
/// struct SomeClassThatCanBeInstantiatedInCode {
///     // Uninitialized value will be represented by `42` in the editor.
///     // Will cause panic if not set via the editor or code before use.
///     #[export]
///     #[init(invalid = 42)]
///     some_primitive: OnEditor<i64>,
/// }
///
/// fn create_and_add(mut this: Gd<Node>, val: i64) -> Gd<SomeClassThatCanBeInstantiatedInCode> {
///     let mut my_node = SomeClassThatCanBeInstantiatedInCode::new_alloc();
///
///     // Would cause the panic:
///     // this.add_child(&my_node);
///
///     my_node.bind_mut().some_primitive.init(val);
///
///     // Will not cause the panic.
///     this.add_child(&my_node);
///
///     my_node
/// }
/// ```
///
/// # Using `OnEditor<T>` with `#[class(tool)]`
///
/// When used with `#[class(tool)]`, the before-ready checks are omitted.
/// Otherwise, `OnEditor<T>` behaves the same — accessing an uninitialized value
/// will cause a panic.
pub struct OnEditor<T> {
    inner: OnEditorState<T>,
}

pub(crate) enum OnEditorState<T> {
    /// Uninitialized null value.
    Null,
    /// Uninitialized state, but with a value marked as invalid (required to represent non-nullable type in the editor).
    Uninitialized(T),
    /// Initialized with a value.
    Initialized(T),
}

/// `OnEditor<T>` is usable only for properties – which is enforced via `Var` and `FromGodot` bounds.
///
/// Furthermore, `PartialEq` is needed to compare against uninitialized sentinel values.
impl<T: Var + FromGodot + PartialEq> OnEditor<T> {
    /// Initializes invalid `OnEditor<T>` with given value.
    ///
    /// # Panics
    /// If `init()` was called before.
    pub fn init(&mut self, val: T) {
        match self.inner {
            OnEditorState::Null | OnEditorState::Uninitialized(_) => {
                *self = OnEditor {
                    inner: OnEditorState::Initialized(val),
                };
            }
            OnEditorState::Initialized(_) => {
                panic!("Given OnEditor value has been already initialized; did you call init() more than once?")
            }
        }
    }

    /// Creates new `OnEditor<T>` with a value that is considered invalid.
    ///
    /// If this value is not changed in the editor, accessing it from Rust will cause a panic.
    pub fn new_invalid(val: T) -> Self
    where
        T::Via: BuiltinExport,
    {
        OnEditor {
            inner: OnEditorState::Uninitialized(val),
        }
    }

    /// Creates new uninitialized `OnEditor<T>` value for nullable GodotTypes.
    ///
    /// Not a part of public API – available only via `Default` implementation on `OnEditor<Gd<T>>` and `OnEditor<DynGd<D, T>>`.
    pub(crate) fn gd_invalid() -> Self {
        OnEditor {
            inner: OnEditorState::Null,
        }
    }

    #[doc(hidden)]
    pub fn is_invalid(&self) -> bool {
        match self.inner {
            OnEditorState::Null | OnEditorState::Uninitialized(_) => true,
            OnEditorState::Initialized(_) => false,
        }
    }

    /// `Var::get_property` implementation that works both for nullable and non-nullable types.
    pub(crate) fn get_property_inner(&self) -> Option<T::Via> {
        match &self.inner {
            OnEditorState::Null => None,
            OnEditorState::Uninitialized(val) | OnEditorState::Initialized(val) => {
                Some(val.get_property())
            }
        }
    }

    /// [`Var::set_property`] implementation that works both for nullable and non-nullable types.
    ///
    /// All the state transitions are valid, since it is being run only in the editor.
    /// See also [`Option::set_property()`].
    pub(crate) fn set_property_inner(&mut self, value: Option<T::Via>) {
        match (value, &mut self.inner) {
            (None, _) => self.inner = OnEditorState::Null,
            (Some(value), OnEditorState::Initialized(current_value)) => {
                current_value.set_property(value);
            }
            (Some(value), OnEditorState::Null) => {
                self.inner = OnEditorState::Initialized(FromGodot::from_godot(value))
            }
            (Some(value), OnEditorState::Uninitialized(current_value)) => {
                let value = FromGodot::from_godot(value);
                if value != *current_value {
                    self.inner = OnEditorState::Initialized(value)
                }
            }
        }
    }
}

impl<T> std::ops::Deref for OnEditor<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        match &self.inner {
            OnEditorState::Null | OnEditorState::Uninitialized(_) => {
                panic!("OnEditor field hasn't been initialized.")
            }
            OnEditorState::Initialized(v) => v,
        }
    }
}

impl<T> std::ops::DerefMut for OnEditor<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        match &mut self.inner {
            OnEditorState::Null | OnEditorState::Uninitialized(_) => {
                panic!("OnEditor field hasn't been initialized.")
            }
            OnEditorState::Initialized(v) => v,
        }
    }
}

impl<T> GodotConvert for OnEditor<T>
where
    T: GodotConvert,
    T::Via: GodotType + BuiltinExport,
{
    type Via = T::Via;
}

impl<T> Var for OnEditor<T>
where
    OnEditor<T>: GodotConvert<Via = T::Via>,
    T: Var + FromGodot + PartialEq,
    T::Via: BuiltinExport,
{
    fn get_property(&self) -> Self::Via {
        // Will never fail – `PrimitiveGodotType` can not be represented by the `OnEditorState::Null`.
        OnEditor::<T>::get_property_inner(self).expect("DirectExport is not nullable.")
    }

    fn set_property(&mut self, value: T::Via) {
        OnEditor::<T>::set_property_inner(self, Some(value));
    }
}

impl<T> Export for OnEditor<T>
where
    OnEditor<T>: Var,
    T: GodotConvert + Export,
    T::Via: BuiltinExport,
{
    fn export_hint() -> PropertyHintInfo {
        T::export_hint()
    }
}

impl<T> BuiltinExport for OnEditor<T> {}
