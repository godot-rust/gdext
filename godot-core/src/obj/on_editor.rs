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
/// Use this type whenever your Rust code cannot provide a value for a field, but expects one to be specified in the Godot editor.
///
/// If you need automatic initialization during `ready()`, e.g. for loading nodes or resources, use [`OnReady<Gd<T>>`](crate::obj::OnReady)
/// instead. As a general "maybe initialized" type, `Option<Gd<T>>` is always available, even if more verbose.
///
///
/// # What constitutes "initialized"?
/// Whether a value is considered initialized or not depends on `T`.
///
/// - For objects, a value is initialized if it is not null. Exported object propreties in Godot are nullable, but `Gd<T>` and `DynGd<T, D>` do
///   not support nullability and can thus not directly be exported with `#[export]`. `OnEditor` can bridge this gap, by expecting users
///   to set a non-null value, and panicking if they don't.
/// - For built-in types, a value is initialized if it is different from a user-selected sentinel value (e.g. `-1`).
///
/// More on this below (see also table-of-contents sidebar).
///
/// # Initialization semantics
/// Panics during access (`Deref/DerefMut` impls) if uninitialized.
///
/// When used inside a node class, `OnEditor` checks if a value has been set before `ready()` is run, and panics otherwise.
/// This validation is performed for all `OnEditor` fields declared in a given `GodotClass`, regardless of whether they are `#[var]`, `#[export]`, or neither.
/// Once initialized, `OnEditor` can be used almost as if it were a `T` value itself, due to `Deref`/`DerefMut` impls.
///
/// `OnEditor<T>` should always be used as a struct field, preferably in tandem with an `#[export]` or `#[var]`.
/// Initializing `OnEditor` values via code before the first use is supported, but should be limited to use cases involving builder or factory patterns.
///
///
/// # Using `OnEditor` with classes
/// You can wrap class smart pointers `Gd<T>` and `DynGd<T, D>` inside `OnEditor`, to make them exportable.
/// `Gd<T>` itself does not implement the `Export` trait.
///
/// ## Example: automatic init
/// This example uses the `Default` impl, which expects a non-null value to be provided.
///
/// ```
/// # use godot::prelude::*;
/// #[derive(GodotClass)]
/// #[class(init, base = Node)]
/// struct ResourceHolder {
///     #[export]
///     editor_property: OnEditor<Gd<Resource>>,
/// }
///
/// #[godot_api]
/// impl INode for ResourceHolder {
///     fn ready(&mut self) {
///         // Will always be valid and **must** be set via editor.
///         // Additional check is being run before ready(),
///         // to ensure that given value can't be null.
///         let some_variant = self.editor_property.get_meta("SomeName");
///     }
/// }
/// ```
///
/// ## Example: user-defined init
/// Uninitialized `OnEditor<Gd<T>>` and `OnEditor<DynGd<T, D>>` can be created with `OnEditor::default()`.
///
/// ```
/// # use godot::prelude::*;
/// #[derive(GodotClass)]
/// #[class(base = Node)]
/// struct NodeHolder {
///     #[export]
///     required_node: OnEditor<Gd<Node>>,
///
///     base: Base<Node>
/// }
///
/// #[godot_api]
/// impl INode for NodeHolder {
///     fn init(base: Base<Node>) -> Self {
///        Self {
///            base,
///            required_node: OnEditor::default(),
///        }
///     }
/// }
///```
///
/// ## Example: factory pattern
///
/// ```
/// # use godot::prelude::*;
/// #[derive(GodotClass)]
/// #[class(init, base = Node)]
/// struct NodeHolder {
///     #[export]
///     required_node: OnEditor<Gd<Node>>,
/// }
///
/// fn create_and_add(
///     mut this: Gd<Node>,
///     some_class_scene: Gd<PackedScene>,
///     some_node: Gd<Node>,
/// ) -> Gd<NodeHolder> {
///     let mut my_node = some_class_scene.instantiate_as::<NodeHolder>();
///
///     // Would cause a panic:
///     // this.add_child(&my_node);
///
///     // It's possible to initialize the value programmatically, although typically
///     // it is set in the editor and stored in a .tscn file.
///     // Note: nodes are manually managed and leak memory unless tree-attached or freed.
///     my_node.bind_mut().required_node.init(some_node);
///
///     // Will not panic, since the node is initialized now.
///     this.add_child(&my_node);
///
///     my_node
/// }
/// ```
///
/// # Using `OnEditor` with built-in types
/// `OnEditor<T>` can be used with any `#[export]`-enabled builtins, to provide domain-specific validation logic.
/// An example might be to check whether a game entity has been granted a non-zero ID.
///
/// To detect whether a value has been set in the editor, `OnEditor<T>` uses a _sentinel value_. This is a special marker value for
/// "uninitialized" and is selected by the user. For example, a sentinel value of `-1` or `0` might be used to represent an uninitialized `i32`.
///
/// There is deliberately no `Default` implementation for `OnEditor` with builtins, as the sentinel is highly domain-specific.
///
/// ## Example
///
/// ```
/// use godot::prelude::*;
///
/// #[derive(GodotClass)]
/// #[class(init, base = Node)]
/// struct IntHolder {
///     // Uninitialized value will be represented by `42` in the editor.
///     // Will cause panic if not set via the editor or code before use.
///     #[export]
///     #[init(sentinel = 42)]
///     some_primitive: OnEditor<i64>,
/// }
///
/// fn create_and_add(mut this: Gd<Node>, val: i64) -> Gd<IntHolder> {
///     let mut my_node = IntHolder::new_alloc();
///
///     // Would cause a panic:
///     // this.add_child(&my_node);
///
///     // It's possible to initialize the value programmatically, although typically
///     // it is set in the editor and stored in a .tscn file.
///     my_node.bind_mut().some_primitive.init(val);
///
///     // Will not panic, since the node is initialized now.
///     this.add_child(&my_node);
///
///     my_node
/// }
/// ```
///
/// # Custom getters and setters for `OnEditor`
/// Custom setters and/or getters for `OnEditor` are declared by accepting/returning the inner type.
/// In their implementation, delegate to the `Var` trait methods, rather than dereferencing directly.
///
/// ```
/// # use godot::prelude::*;
/// #[derive(GodotClass)]
/// #[class(init)]
/// struct MyStruct {
///     #[var(get, set)]
///     my_node: OnEditor<Gd<Node>>,
///
///     #[var(get, set)]
///     #[init(sentinel = -1)]
///     my_value: OnEditor<i32>,
/// }
///
/// #[godot_api]
/// impl MyStruct {
///     #[func]
///     pub fn get_my_node(&self) -> Gd<Node> {
///         let ret = Var::var_pub_get(&self.my_node);
///         // Do something with the value...
///         ret
///     }
///
///     #[func]
///     pub fn set_my_node(&mut self, value: Gd<Node>) {
///         // Validate, pre-process, etc...
///         Var::var_pub_set(&mut self.my_node, value);
///     }
///
///     #[func]
///     pub fn get_my_value(&self) -> i32 {
///         let ret = Var::var_pub_get(&self.my_value);
///         // Do something with the value...
///         ret
///     }
///
///     #[func]
///     pub fn set_my_value(&mut self, value: i32) {
///         if value == 13 {
///             godot_warn!("13 is unlucky number.");
///             return;
///         }
///
///         Var::var_pub_set(&mut self.my_value, value);
///     }
/// }
/// ```
///
/// See also: [Register properties -- `#[var]`](../register/derive.GodotClass.html#register-properties--var)
///
/// # Using `OnEditor` with `#[class(tool)]`
/// When used with `#[class(tool)]`, the before-ready checks are omitted.
/// Otherwise, `OnEditor<T>` behaves the same — accessing an uninitialized value will cause a panic.
#[derive(Debug)]
pub struct OnEditor<T> {
    state: OnEditorState<T>,
}

#[derive(Debug)]
pub(crate) enum OnEditorState<T> {
    /// Uninitialized null value.
    UninitNull,
    /// Uninitialized state, but with a value marked as invalid (required to represent non-nullable type in the editor).
    UninitSentinel(T),
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
        match self.state {
            OnEditorState::UninitNull | OnEditorState::UninitSentinel(_) => {
                *self = OnEditor {
                    state: OnEditorState::Initialized(val),
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
    pub fn from_sentinel(val: T) -> Self
    where
        T::Via: BuiltinExport,
    {
        OnEditor {
            state: OnEditorState::UninitSentinel(val),
        }
    }

    /// Creates new uninitialized `OnEditor<T>` value for nullable GodotTypes.
    ///
    /// Not a part of public API – available only via `Default` implementation on `OnEditor<Gd<T>>` and `OnEditor<DynGd<D, T>>`.
    pub(crate) fn gd_invalid() -> Self {
        OnEditor {
            state: OnEditorState::UninitNull,
        }
    }

    #[doc(hidden)]
    pub fn is_invalid(&self) -> bool {
        match self.state {
            OnEditorState::UninitNull | OnEditorState::UninitSentinel(_) => true,
            OnEditorState::Initialized(_) => false,
        }
    }

    /// `Var::get_property` implementation that works both for nullable and non-nullable types.
    pub(crate) fn get_property_inner(&self) -> Option<T::Via> {
        match &self.state {
            OnEditorState::UninitNull => None,
            OnEditorState::UninitSentinel(val) | OnEditorState::Initialized(val) => {
                Some(T::var_get(val))
            }
        }
    }

    /// [`Var::var_set`] implementation that works both for nullable and non-nullable types.
    ///
    /// All the state transitions are valid, since it is being run only in the editor.
    /// See also [`Option::var_set()`].
    pub(crate) fn set_property_inner(&mut self, value: Option<T::Via>) {
        match (value, &mut self.state) {
            (None, _) => self.state = OnEditorState::UninitNull,
            (Some(value), OnEditorState::Initialized(current_value)) => {
                T::var_set(current_value, value);
            }
            (Some(value), OnEditorState::UninitNull) => {
                self.state = OnEditorState::Initialized(FromGodot::from_godot(value))
            }
            (Some(value), OnEditorState::UninitSentinel(current_value)) => {
                let value = FromGodot::from_godot(value);
                if value != *current_value {
                    self.state = OnEditorState::Initialized(value)
                }
            }
        }
    }
}

impl<T> std::ops::Deref for OnEditor<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        match &self.state {
            OnEditorState::UninitNull | OnEditorState::UninitSentinel(_) => {
                panic!("OnEditor field hasn't been initialized.")
            }
            OnEditorState::Initialized(v) => v,
        }
    }
}

impl<T> std::ops::DerefMut for OnEditor<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        match &mut self.state {
            OnEditorState::UninitNull | OnEditorState::UninitSentinel(_) => {
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
    type PubType = T::Via;

    fn var_get(field: &Self) -> Self::Via {
        field
            .get_property_inner()
            .expect("OnEditor field of non-nullable type must be initialized before access")
    }

    fn var_set(field: &mut Self, value: Self::Via) {
        field.set_property_inner(Some(value));
    }

    fn var_pub_get(field: &Self) -> Self::PubType {
        Self::var_get(field)
    }

    fn var_pub_set(field: &mut Self, value: Self::PubType) {
        Self::var_set(field, value)
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
