/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::meta::{ClassName, FromGodot, GodotConvert, GodotType, PropertyHintInfo};
use crate::obj::{bounds, Bounds, Gd, GodotClass};
use crate::registry::property::{Export, Var};

// Possible areas for improvement that can be explored:
// - Check if it makes sense to add `OnEditor<T>` support for primitives as well (such as ids and whatnot – "invalid" value such as `-1` or Vector(NaN, NaN, NaN) should represent null/None in such cases).
// - Should we provide something similar to [`from_base_fn()`](crate::OnReady::from_base_fn)? Right now it is just a simple wrapper for Option that helps to organize code and provides ergonomic improvements – on another hand more elaborate late initialization logic should be handled either by Option or OnReady.
// - Skipping `OnEditor::new(…)` in `#[init=val(…)]` - it is nothing but noise since OnEditor has only two logical states – "HasValue" (Some) and "Invalid" (None). Might be confusing, since nothing else follows such pattern.

/// Represents exported property which must not be null and must be set via the editor – or associated code – before use.
/// Allows to use `Gd<T>` – which by itself never holds null objects – as an `#[export]` which should not be null during the runtime.
///
/// Panics during access if value hasn't been set. Checks if value has been set before the `ready` is being run and panics if `OnEditor` fields are not properly initialized.
/// `OnEditor<T>` should always be used as a property, preferably in tandem with an `#[export]`.
/// It should be used as it would be a value itself and lack thereof treated as a logical error.
///
/// `#[init]` can be used to provide default values.
/// One can create new instance and set its required properties after the init, albeit [`Option<Gd<T>>`](std::option) and [`OnReady<Gd<T>>`](crate::obj::onready::OnReady) should be preferred instead for late initialization.
///
/// # Example - auto-generated init
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
/// # Example - user-generated init
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
/// # Example - Late init
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
///     my_node_to_add.bind_mut().required_node = OnEditor::new(Node::new_alloc());
///     // Will not cause the panic.
///     this.add_child(&my_node_to_add);
/// }
/// ```
#[doc(alias = "impl<T> export for Gd<T>", alias = "gd_export")]
pub enum OnEditor<T> {
    // Represents uninitialized, null value.
    Null,
    // Represents initialized, invalid value.
    Uninitialized(T),
    Initialized(T),
}

impl<T: GodotConvert + Var + FromGodot> OnEditor<T> {
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

    #[doc(hidden)]
    pub(crate) fn get_property(&self) -> Option<T::Via> {
        match self {
            OnEditor::Null => None,
            OnEditor::Uninitialized(val) | OnEditor::Initialized(val) => Some(val.get_property()),
        }
    }

    #[doc(hidden)]
    pub(crate) fn set_property(&mut self, value: Option<T::Via>) {
        match value {
            None => *self = OnEditor::Null,
            Some(value) => {
                if let OnEditor::Initialized(current_value) = self {
                    current_value.set_property(value)
                } else {
                    *self = OnEditor::Initialized(FromGodot::from_godot(value))
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

// Blanket implementations for nullable types.
// Don't provide blanket implementations for primitives.

#[doc(hidden)]
#[allow(clippy::derivable_impls)]
impl<T: GodotClass> Default for OnEditor<Gd<T>> {
    fn default() -> Self {
        OnEditor::Null
    }
}

impl<T: GodotClass> GodotConvert for OnEditor<Gd<T>>
where
    Gd<T>: GodotConvert,
    Option<<Gd<T> as GodotConvert>::Via>: GodotType,
{
    type Via = Option<<Gd<T> as GodotConvert>::Via>;
}

impl<T> Var for OnEditor<Gd<T>>
where
    T: GodotClass,
    OnEditor<Gd<T>>: GodotConvert<Via = Option<<Gd<T> as GodotConvert>::Via>>,
{
    fn get_property(&self) -> Self::Via {
        self.get_property()
    }

    fn set_property(&mut self, value: Self::Via) {
        self.set_property(value)
    }
}

impl<T> Export for OnEditor<Gd<T>>
where
    T: GodotClass + Bounds<Exportable = bounds::Yes>,
    OnEditor<Gd<T>>: Var,
{
    fn export_hint() -> PropertyHintInfo {
        Gd::<T>::export_hint()
    }

    #[doc(hidden)]
    fn as_node_class() -> Option<ClassName> {
        Gd::<T>::as_node_class()
    }
}
