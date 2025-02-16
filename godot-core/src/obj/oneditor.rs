/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::meta::{ClassName, FromGodot, GodotConvert, PropertyHintInfo};
use crate::obj::{bounds, Bounds, Gd, GodotClass};
use crate::registry::property::{Export, Var};

// Possible areas for improvement that can be explored:
// - Check if `impl<T: Export + GodotType> Export for OnEditor<T>` makes sense as well to support exporting primitives (such as ids and whatnot – "invalid" value such as `-1` or Vector(NaN, NaN, NaN) should represent null/None in such cases).
// - Should we provide something similar to [`from_base_fn()`](crate::OnReady::from_base_fn)? Right now it is just a simple wrapper for Option that helps to organize code and provides ergonomic improvements – on another hand more elaborate late initialization logic should be handled either by Option or OnReady.
// - Skipping `OnEditor::new(…)` in `#[init=val(…)]` - it is nothing but noise since OnEditor has only two logical states – "HasValue" (Some) and "Invalid" (None). Might be confusing, since nothing else follows such pattern.

/// Represents exported `Gd<T>` property which must not be null and must be set via the editor – or associated code – before use.
///
/// Panics during access if value hasn't been set. Checks if value has been set before the `ready` is being run and panics if `OnEditor` fields are not properly initialized.
/// Should always be used as a property, preferably in tandem with an `#[export]`.
///
/// The underlying type is de facto a wrapper for an `Option<Gd<T>` which dereferences to underlying value.
/// It should be used as it would be a value itself and lack thereof treated as a logical error.
///
/// `#[init]` can be used to provide default values – for example default Resources supposed to be filled by user.
/// One can create new instance and set its required properties after the init, albeit [`Option<Gd<T>>`](option) and [`OnReady<Gd<T>>`](crate::obj::onready::OnReady) should be preferred instead for late initialization.
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
///     base: Base<Node>,
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
pub struct OnEditor<T> {
    inner: Option<T>,
}

impl<T: GodotConvert> OnEditor<T> {
    pub fn new(val: T) -> Self {
        OnEditor { inner: Some(val) }
    }

    #[doc(hidden)]
    pub fn is_invalid(&self) -> bool {
        self.inner.is_none()
    }
}

#[doc(hidden)]
impl<T: GodotConvert> Default for OnEditor<T> {
    fn default() -> Self {
        OnEditor { inner: None }
    }
}

impl<T> std::ops::Deref for OnEditor<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        match &self.inner {
            None => panic!(),
            Some(v) => v,
        }
    }
}

impl<T> std::ops::DerefMut for OnEditor<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        match &mut self.inner {
            None => panic!(),
            Some(v) => v,
        }
    }
}

impl<T: GodotConvert> GodotConvert for OnEditor<T> {
    type Via = T::Via;
}

impl<T: Var> Var for OnEditor<T>
where
    T: FromGodot,
{
    fn get_property(&self) -> Self::Via {
        let deref: &T = self;
        deref.get_property()
    }

    fn set_property(&mut self, value: Self::Via) {
        if self.inner.is_none() {
            self.inner = Some(FromGodot::from_godot(value))
        } else {
            let deref: &mut T = self;
            deref.set_property(value);
        }
    }
}

impl<T> Export for OnEditor<Gd<T>>
where
    T: GodotClass + Bounds<Exportable = bounds::Yes>,
{
    fn export_hint() -> PropertyHintInfo {
        Gd::<T>::export_hint()
    }

    #[doc(hidden)]
    fn as_node_class() -> Option<ClassName> {
        Gd::<T>::as_node_class()
    }
}
