/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::fmt::Write;

use proc_macro2::{Ident, TokenStream};
use quote::quote;

use crate::context::Context;
use crate::generator::functions_common::FnCode;
use crate::generator::{docs, functions_common};
use crate::models::domain::{
    ApiView, Class, ClassLike, ClassMethod, FnQualifier, Function, TyName, VirtualMethodPresence,
};
use crate::special_cases;
use crate::util::ident;

pub fn make_virtual_methods_trait(
    class: &Class,
    all_base_names: &[TyName],
    notification_enum_name: &Ident,
    cfg_attributes: &TokenStream,
    view: &ApiView,
    ctx: &Context,
) -> TokenStream {
    let class_name = &class.name().rust_ty;
    let trait_name_str = class.name().virtual_trait_name();
    let trait_name = ident(&trait_name_str);

    let (virtual_method_fns, extra_docs) = make_all_virtual_methods(class, all_base_names, view);
    let special_virtual_methods = make_special_virtual_methods(notification_enum_name);

    let base_traits = collect_base_traits(all_base_names, ctx);
    let trait_doc = docs::make_virtual_trait_doc(&trait_name_str, &base_traits, class.name());

    quote! {
        #[doc = #trait_doc]
        #[doc = #extra_docs]
        #[allow(unused_variables)]
        #[allow(clippy::unimplemented)]
        #cfg_attributes
        pub trait #trait_name: crate::obj::GodotClass<Base = #class_name> + crate::private::You_forgot_the_attribute__godot_api {
            #special_virtual_methods
            #virtual_method_fns
        }
    }
}

/// Collects base traits (without current) in order towards root.
///
/// Results contain the name of the trait `I*` and whether code for it is generated (e.g. `false` for final classes).
fn collect_base_traits(all_base_classes: &[TyName], ctx: &Context) -> Vec<(String, bool)> {
    let mut base_traits = vec![];

    for class_name in all_base_classes {
        let trait_name = class_name.virtual_trait_name();
        let has_interface_trait = !ctx.is_final(class_name);

        base_traits.push((trait_name, has_interface_trait))
    }

    base_traits
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Implementation

fn make_special_virtual_methods(notification_enum_name: &Ident) -> TokenStream {
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
        /// [`Object::notify`][crate::classes::Object::notify] method.
        ///
        /// See also in Godot docs:
        /// * [`Object::_notification`](https://docs.godotengine.org/en/stable/classes/class_object.html#class-object-method-notification).
        /// * [Notifications tutorial](https://docs.godotengine.org/en/stable/tutorials/best_practices/godot_notifications.html).
        fn on_notification(&mut self, what: #notification_enum_name) {
            unimplemented!()
        }

        /// Called whenever [`get()`](crate::classes::Object::get) is called or Godot gets the value of a property.
        ///
        /// Should return the given `property`'s value as `Some(value)`, or `None` if the property should be handled normally.
        ///
        /// See also in Godot docs:
        /// * [`Object::_get`](https://docs.godotengine.org/en/stable/classes/class_object.html#class-object-private-method-get).
        fn get_property(&self, property: StringName) -> Option<Variant> {
            unimplemented!()
        }

        /// Called whenever Godot [`set()`](crate::classes::Object::set) is called or Godot sets the value of a property.
        ///
        /// Should set `property` to the given `value` and return `true`, or return `false` to indicate the `property`
        /// should be handled normally.
        ///
        /// See also in Godot docs:
        /// * [`Object::_set`](https://docs.godotengine.org/en/stable/classes/class_object.html#class-object-private-method-set).
        fn set_property(&mut self, property: StringName, value: Variant) -> bool {
            unimplemented!()
        }

        /// Called whenever Godot [`get_property_list()`](crate::classes::Object::get_property_list) is called, the returned vector here is
        /// appended to the existing list of properties.
        ///
        /// This should mainly be used for advanced purposes, such as dynamically updating the property list in the editor.
        ///
        /// See also in Godot docs:
        /// * [`Object::_get_property_list`](https://docs.godotengine.org/en/latest/classes/class_object.html#class-object-private-method-get-property-list)
        #[cfg(since_api = "4.3")]
        fn get_property_list(&mut self) -> Vec<crate::meta::PropertyInfo> {
            unimplemented!()
        }

        /// Called whenever Godot retrieves value of property. Allows to customize existing properties.
        /// Every property info goes through this method, except properties **added** with `get_property_list()`.
        ///
        /// Exposed `property` here is a shared mutable reference obtained (and returned to) from Godot.
        ///
        /// See also in the Godot docs:
        /// * [`Object::_validate_property`](https://docs.godotengine.org/en/stable/classes/class_object.html#class-object-private-method-validate-property)
        fn validate_property(&self, property: &mut crate::meta::PropertyInfo) {
            unimplemented!()
        }

        /// Called by Godot to tell if a property has a custom revert or not.
        ///
        /// Return `None` for no custom revert, and return `Some(value)` to specify the custom revert.
        ///
        /// This is a combination of Godot's [`Object::_property_get_revert`] and [`Object::_property_can_revert`]. This means that this
        /// function will usually be called twice by Godot to find the revert.
        ///
        /// Note that this should be a _pure_ function. That is, it should always return the same value for a property as long as `self`
        /// remains unchanged. Otherwise, this may lead to unexpected (safe) behavior.
        ///
        /// [`Object::_property_get_revert`]: https://docs.godotengine.org/en/latest/classes/class_object.html#class-object-private-method-property-get-revert
        /// [`Object::_property_can_revert`]: https://docs.godotengine.org/en/latest/classes/class_object.html#class-object-private-method-property-can-revert
        #[doc(alias = "property_can_revert")]
        fn property_get_revert(&self, property: StringName) -> Option<Variant> {
            unimplemented!()
        }
    }
}

fn make_virtual_method(
    method: &ClassMethod,
    presence: VirtualMethodPresence,
) -> Option<TokenStream> {
    if !method.is_virtual() {
        return None;
    }

    // Possibly change behavior of required/optional-ness of the virtual method in derived classes.
    // It's also possible that it's removed, which would not declare it at all in the `I*` trait.
    let is_virtual_required = match presence {
        VirtualMethodPresence::Inherit => method.is_virtual_required(),
        VirtualMethodPresence::Override { is_required } => is_required,
        VirtualMethodPresence::Remove => return None,
    };

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
            is_virtual_required,
            is_varcall_fallible: true,
        },
        None,
        &TokenStream::new(),
    );

    // Virtual methods have no builders.
    Some(definition.into_functions_only())
}

fn make_all_virtual_methods(
    class: &Class,
    all_base_names: &[TyName],
    view: &ApiView,
) -> (TokenStream, String) {
    let mut all_tokens = TokenStream::new();

    for method in class.methods.iter() {
        // Assumes that inner function filters on is_virtual.
        if let Some(tokens) = make_virtual_method(method, VirtualMethodPresence::Inherit) {
            all_tokens.extend(tokens);
        }
    }

    let mut changes_from_base = String::new();

    for base_name in all_base_names {
        let base_class = view.get_engine_class(base_name);
        for method in base_class.methods.iter() {
            // Certain derived classes in Godot implement a virtual method declared in a base class, thus no longer
            // making it required. This isn't advertised in the extension_api, but instead manually tracked via special cases.
            let derived_presence = special_cases::get_derived_virtual_method_presence(
                class.name(),
                method.godot_name(),
            );

            // Collect all changes in a Markdown table.
            let new = match derived_presence {
                VirtualMethodPresence::Inherit => None,
                VirtualMethodPresence::Override { is_required } => {
                    Some(format_required(is_required))
                }
                VirtualMethodPresence::Remove => Some("removed"),
            };
            if let Some(new) = new {
                let orig = format_required(method.is_virtual_required());
                let base_name = &base_name.rust_ty;
                let method_name = format_method_name(method);

                write!(
                    changes_from_base,
                    "\n| [`I{base_name}::{method_name}`](crate::classes::I{base_name}::{method_name}) | {orig} | {new} |"
                )
                .unwrap();
            }

            if let Some(tokens) = make_virtual_method(method, derived_presence) {
                all_tokens.extend(tokens);
            }
        }
    }

    let extra_docs = if changes_from_base.is_empty() {
        String::new()
    } else {
        let class_name = &class.name().rust_ty;
        format!(
            "\n\n# Changes from base interface traits\n\
            The following virtual methods originally declared in direct/indirect base classes have their presence (required/optional) changed \
            in `I{class_name}`. This can happen if Godot already overrides virtual methods, discouraging the user from further overriding them.\n\
            \n\n| Base method | Original | New |\n| --- | --- | --- |{changes_from_base}"
        )
    };

    (all_tokens, extra_docs)
}

fn format_required(is_required: bool) -> &'static str {
    if is_required {
        "required"
    } else {
        "optional"
    }
}

fn format_method_name(method: &ClassMethod) -> &str {
    // TODO when we have `_unsafe` or similar postfix with raw pointers, update this here.
    method.name()
}
