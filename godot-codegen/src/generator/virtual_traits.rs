/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::generator::functions_common::FnCode;
use crate::generator::{docs, functions_common};
use crate::models::domain::{
    ApiView, Class, ClassLike, ClassMethod, FnQualifier, Function, TyName,
};
use crate::util::ident;
use proc_macro2::{Ident, TokenStream};
use quote::quote;

pub fn make_virtual_methods_trait(
    class: &Class,
    all_base_names: &[TyName],
    trait_name: &str,
    notification_enum_name: &Ident,
    view: &ApiView,
) -> TokenStream {
    let trait_name = ident(trait_name);

    let virtual_method_fns = make_all_virtual_methods(class, all_base_names, view);
    let special_virtual_methods = special_virtual_methods(notification_enum_name);

    let trait_doc = docs::make_virtual_trait_doc(class.name());

    quote! {
        #[doc = #trait_doc]
        #[allow(unused_variables)]
        #[allow(clippy::unimplemented)]
        pub trait #trait_name: crate::obj::GodotClass + crate::private::You_forgot_the_attribute__godot_api {
            #special_virtual_methods
            #( #virtual_method_fns )*
        }
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Implementation

fn special_virtual_methods(notification_enum_name: &Ident) -> TokenStream {
    quote! {
        #[doc(hidden)]
        fn register_class(builder: &mut crate::builder::ClassBuilder<Self>) {
            unimplemented!()
        }

        /// Godot constructor, accepting an injected `base` object.
        ///
        /// `base` refers to the base instance of the class, which can either be stored in a `Base<T>` field or discarded.
        /// This method returns a fully-constructed instance, which will then be moved into a [`Gd<T>`][crate::obj::Gd] pointer.
        ///
        /// If the class has a `#[class(init)]` attribute, this method will be auto-generated and must not be overridden.
        fn init(base: crate::obj::Base<Self::Base>) -> Self {
            unimplemented!()
        }

        /// String representation of the Godot instance.
        ///
        /// Override this method to define how the instance is represented as a string.
        /// Used by `impl Display for Gd<T>`, as well as `str()` and `print()` in GDScript.
        fn to_string(&self) -> crate::builtin::GString {
            unimplemented!()
        }

        /// Called when the object receives a Godot notification.
        ///
        /// The type of notification can be identified through `what`. The enum is designed to hold all possible `NOTIFICATION_*`
        /// constants that the current class can handle. However, this is not validated in Godot, so an enum variant `Unknown` exists
        /// to represent integers out of known constants (mistakes or future additions).
        ///
        /// This method is named `_notification` in Godot, but `on_notification` in Rust. To _send_ notifications, use the
        /// [`Object::notify`][crate::engine::Object::notify] method.
        ///
        /// See also in Godot docs:
        /// * [`Object::_notification`](https://docs.godotengine.org/en/stable/classes/class_object.html#class-object-method-notification).
        /// * [Notifications tutorial](https://docs.godotengine.org/en/stable/tutorials/best_practices/godot_notifications.html).
        fn on_notification(&mut self, what: #notification_enum_name) {
            unimplemented!()
        }

        /// Called whenever [`get()`](crate::engine::Object::get) is called or Godot gets the value of a property.
        ///
        /// Should return the given `property`'s value as `Some(value)`, or `None` if the property should be handled normally.
        ///
        /// See also in Godot docs:
        /// * [`Object::_get`](https://docs.godotengine.org/en/stable/classes/class_object.html#class-object-private-method-get).
        fn get_property(&self, property: StringName) -> Option<Variant> {
            unimplemented!()
        }

        /// Called whenever Godot [`set()`](crate::engine::Object::set) is called or Godot sets the value of a property.
        ///
        /// Should set `property` to the given `value` and return `true`, or return `false` to indicate the `property`
        /// should be handled normally.
        ///
        /// See also in Godot docs:
        /// * [`Object::_set`](https://docs.godotengine.org/en/stable/classes/class_object.html#class-object-private-method-set).
        fn set_property(&mut self, property: StringName, value: Variant) -> bool {
            unimplemented!()
        }

        /// Called whenever the Godot engine needs to determine the properties an object has.
        ///
        /// This method can be used to dynamically update the properties displayed by the editor depending on various conditions. This should
        /// usually be combined with `#[class(tool)]` so that the code actually runs in the editor. Additionally if the property list changes
        /// you need to call [`notify_property_list_changed`](crate::engine::Object::notify_property_list_changed) to actually notify the engine
        /// that the property list has changed, otherwise nothing will appear to have happened.
        ///
        /// See also in Godot docs:
        /// * [`Object::_get_property_list`](https://docs.godotengine.org/en/stable/classes/class_object.html#class-object-private-method-get-property-list).
        fn get_property_list(&self) -> Vec<PropertyInfo> {
            unimplemented!()
        }

        /// Called by Godot to determine if a property has a custom default value.
        ///
        /// Should return `true` when the property has a custom default value, otherwise should return `false`. Must be used in conjunction with
        /// [`property_get_revert()`] to specify the custom default value.
        ///
        /// See also in Godot docs:
        /// * [`Object::_property_can_revert`](https://docs.godotengine.org/en/stable/classes/class_object.html#class-object-private-method-property-can-revert).
        fn property_can_revert(&self, property: StringName) -> bool {
            unimplemented!()
        }

        /// Called by Godot to determine the custom default value of a property.
        ///
        /// Should return the given property's custom default value as `Some(value)`, or `None` if the given property doesn't have a custom
        /// default value.
        ///
        /// See also in Godot docs:
        /// * [`Object::_property_get_revert`](https://docs.godotengine.org/en/stable/classes/class_object.html#class-object-private-method-property-get-revert).
        fn property_get_revert(&self, property: StringName) -> Option<Variant> {
            unimplemented!()
        }

    }
}

fn make_virtual_method(method: &ClassMethod) -> Option<TokenStream> {
    if !method.is_virtual() {
        return None;
    }

    // Virtual methods are never static.
    let qualifier = method.qualifier();
    assert!(matches!(qualifier, FnQualifier::Mut | FnQualifier::Const));

    let definition = functions_common::make_function_definition(
        method,
        &FnCode {
            receiver: functions_common::make_receiver(qualifier, TokenStream::new()),
            // make_return() requests following args, but they are not used for virtual methods. We can provide empty streams.
            varcall_invocation: TokenStream::new(),
            ptrcall_invocation: TokenStream::new(),
        },
        None,
    );

    // Virtual methods have no builders.
    Some(definition.into_functions_only())
}

fn make_all_virtual_methods(
    class: &Class,
    all_base_names: &[TyName],
    view: &ApiView,
) -> Vec<TokenStream> {
    let mut all_tokens = vec![];

    for method in class.methods.iter() {
        // Assumes that inner function filters on is_virtual.
        if let Some(tokens) = make_virtual_method(method) {
            all_tokens.push(tokens);
        }
    }

    for base_name in all_base_names {
        let base_class = view.get_engine_class(base_name);
        for method in base_class.methods.iter() {
            if let Some(tokens) = make_virtual_method(method) {
                all_tokens.push(tokens);
            }
        }
    }

    all_tokens
}
