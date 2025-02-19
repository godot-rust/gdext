/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::meta::{FromGodot, GodotConvert};
use crate::registry::property::Var;

// Possible areas for improvement that can be explored:
// - Should we provide something similar to [`from_base_fn()`](crate::OnReady::from_base_fn)? In general more elaborate late initialization logic should be handled either by Option or OnReady.
// - Adding `OnEditor` section to `init(…)`. Might be noisy and unnecessary, since OnEditor, for now, avoids elaborate late initialization logic.

/// Represents exported property which must not be null and must be set via the editor – or associated code – before use.
/// Allows to use `Gd<T>` – which by itself never holds null objects – as an `#[export]` which should not be null during the runtime.
///
/// Panics during access if value hasn't been set. Checks if value has been set before the `ready` is being run and panics if any of `OnEditor` fields is not properly initialized.
/// `OnEditor<T>` should always be used as a property, preferably in tandem with an `#[export]` or `#[var]`.
/// It should be used as it would be a value itself and lack thereof treated as a logical error.
///
/// `#[init]` can be used to provide default values.
/// One can create new instance and set its required properties after the init, albeit [`Option<Gd<T>>`](std::option) and [`OnReady<Gd<T>>`](crate::obj::onready::OnReady) should be preferred instead for late initialization.
///
/// # Using `OnEditor<T>` with `Gd<T>` and `DynGd<T, D>`
///
/// Primarily use of `OnEditor<Gd<T>>` is exposing properties to the Godot editor.
///
/// One can provide default values for `OnEditor` using `#[init]`.
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
///     editor_field: OnEditor<Gd<Resource>>,
///     #[export]
///     #[init(val = OnEditor::new(Node::new_alloc()))]
///     required_with_default: OnEditor<Gd<Node>>,
///     // Does **NOT** require base field to work.
///     base: Base<Node>,
/// }
///
/// #[godot_api]
/// impl INode for MyClass {
///     fn ready(&mut self) {
///         // Field `required_with_default` can be either default value - specified in `#[init]` or value set via the Godot Editor.
///        assert_eq!(self.required_with_default.get_class(), GString::from("Node"));
///         // Will always be valid and must be set via editor – an additional check is being run before ready to make sure that given value can't be null.
///         let some_variant = self.editor_field.get_meta("SomeName");
///     }
/// }
/// ```
///
/// ## Example - user-generated init
///
/// ```
///  use godot::prelude::*;
///
/// #[derive(GodotClass)]
/// #[class(base = Node)]
/// struct MyClass {
///     #[export]
///     required_node: OnEditor<Gd<Node>>,
///     base: Base<Node>
/// }
///
/// #[godot_api]
/// impl INode for MyClass {
///     fn init(base: Base<Node>) -> Self {
///        Self {
///            base,
///            required_node: OnEditor::new(Node::new_alloc()),
///        }
///     }
/// }
///```
///
/// ## Example - Late init
///
/// ```
///  use godot::prelude::*;
///
/// #[derive(GodotClass)]
/// #[class(init, base = Node)]
/// struct SomeClassThatCanBeInstantiatedInCode {
///     #[export]
///     required_node: OnEditor<Gd<Node>>,
/// }
///
/// fn foo(mut this: Gd<Node>) {
///     let mut my_node_to_add = SomeClassThatCanBeInstantiatedInCode::new_alloc();
///     // Would cause the panic:
///     // this.add_child(&my_node_to_add);
///     // Note: Remember that nodes are manually managed. They will leak memory if not added to tree and/or pruned.
///     my_node_to_add.bind_mut().required_node = OnEditor::new(Node::new_alloc());
///     // Will not cause the panic.
///     this.add_child(&my_node_to_add);
/// }
/// ```
///
/// # Using `OnEditor<T>` with other GodotTypes
///
/// `OnEditor<T>` can be used with other built-ins to provide an extra validation logic and making sure that given properties has been set.
/// Example usage might be checking if entities has been granted proper generated ids.
///
/// In such cases user must specify value which will be deemed invalid. Accessing uninitialized value will cause the panic.
/// To initialize given value simply replace it with `Onready::new(…)`.
///
/// ## Example - using `OnEditor` with primitives
///
/// ```
///  use godot::prelude::*;
///
/// #[derive(GodotClass)]
/// #[class(init, base = Node)]
/// struct SomeClassThatCanBeInstantiatedInCode {
///     #[export]
///     #[init(val = OnEditor::uninit(42))]
///     some_primitive: OnEditor<i64>,
/// }
///
/// fn foo(mut this: Gd<Node>) {
///     let mut my_node_to_add = SomeClassThatCanBeInstantiatedInCode::new_alloc();
///     // Would cause the panic:
///     // this.add_child(&my_node_to_add);
///     my_node_to_add.bind_mut().some_primitive = OnEditor::new(45);
///     // Will not cause the panic.
///     this.add_child(&my_node_to_add);
/// }
/// ```
///
#[doc(
    alias = "impl<T> export for Gd<T>",
    alias = "gd_export",
    alias = "dyn_gd_export",
    alias = "impl<T, D> export for DynGd<T, D>"
)]
pub enum OnEditor<T> {
    // Represents uninitialized, null value.
    Null,
    // Represents initialized, invalid value.
    Uninitialized(T),
    Initialized(T),
}

impl<T: GodotConvert + Var + FromGodot + PartialEq> OnEditor<T> {
    pub fn new(val: T) -> Self {
        OnEditor::Initialized(val)
    }

    pub fn uninit(val: T) -> Self {
        OnEditor::Uninitialized(val)
    }

    #[doc(hidden)]
    pub fn is_invalid(&self) -> bool {
        match self {
            OnEditor::Null | OnEditor::Uninitialized(_) => true,
            OnEditor::Initialized(_) => false,
        }
    }

    /// `Var::get_property` implementation that works both for nullable and non-nullable types.
    #[doc(hidden)]
    pub(crate) fn get_property(&self) -> Option<T::Via> {
        match self {
            OnEditor::Null => None,
            OnEditor::Uninitialized(val) | OnEditor::Initialized(val) => Some(val.get_property()),
        }
    }

    /// `Var::set_property` implementation that works both for nullable and non-nullable types.
    #[doc(hidden)]
    pub(crate) fn set_property(&mut self, value: Option<T::Via>)
    where
        T::Via: PartialEq,
    {
        match (value, &mut *self) {
            (None, _) => *self = OnEditor::Null,
            (Some(value), OnEditor::Initialized(current_value)) => {
                current_value.set_property(value)
            }
            (Some(value), OnEditor::Null) => {
                *self = OnEditor::Initialized(FromGodot::from_godot(value))
            }
            (Some(value), OnEditor::Uninitialized(current_value)) => {
                let value = FromGodot::from_godot(value);
                if value != *current_value {
                    *self = OnEditor::Initialized(value)
                }
            }
        }
    }
}

impl<T> std::ops::Deref for OnEditor<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        match &self {
            OnEditor::Null | OnEditor::Uninitialized(_) => {
                panic!("godot-rust: OnEditor field hasn't been initialized.")
            }
            OnEditor::Initialized(v) => v,
        }
    }
}

impl<T> std::ops::DerefMut for OnEditor<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        match self {
            OnEditor::Null | OnEditor::Uninitialized(_) => {
                panic!("godot-rust: OnEditor field hasn't been initialized.")
            }
            OnEditor::Initialized(v) => v,
        }
    }
}
