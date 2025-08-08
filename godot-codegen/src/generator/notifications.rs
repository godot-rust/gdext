/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use proc_macro2::{Ident, TokenStream};
use quote::quote;

use crate::context::Context;
use crate::models::domain::TyName;
use crate::models::json::JsonClassConstant;
use crate::util;

pub fn make_notify_methods(class_name: &TyName, ctx: &mut Context) -> TokenStream {
    // Note: there are two more methods, but only from Node downwards, not from Object:
    // - notify_thread_safe
    // - notify_deferred_thread_group
    // This could be modeled as either a single method, or two methods:
    //   fn notify(what: XyNotification);
    //   fn notify_with(what: XyNotification, mode: NotifyMode);
    // with NotifyMode being an enum of: Normal | Reversed | ThreadSafe | DeferredThreadGroup.
    // This would need either 2 enums (one starting at Object, one at Node) or have runtime checks.

    let enum_name = ctx.notification_enum_name(class_name);

    // If this class does not have its own notification type, do not redefine the methods.
    // The one from the parent class is fine.
    if !enum_name.declared_by_own_class {
        return TokenStream::new();
    }

    let enum_name = enum_name.name;

    quote! {
        /// ⚠️ Sends a Godot notification to all classes inherited by the object.
        ///
        /// Triggers calls to `on_notification()`, and depending on the notification, also to Godot's lifecycle callbacks such as `ready()`.
        ///
        /// Starts from the highest ancestor (the `Object` class) and goes down the hierarchy.
        /// See also [Godot docs for `Object::notification()`](https://docs.godotengine.org/en/latest/classes/class_object.html#id3).
        ///
        /// # Panics
        ///
        /// If you call this method on a user-defined object while holding a `GdRef` or `GdMut` guard on the instance, you will encounter
        /// a panic. The reason is that the receiving virtual method `on_notification()` acquires a `GdMut` lock dynamically, which must
        /// be exclusive.
        pub fn notify(&mut self, what: #enum_name) {
            self.notification(i32::from(what), false);
        }

        /// ⚠️ Like [`Self::notify()`], but starts at the most-derived class and goes up the hierarchy.
        ///
        /// See docs of that method, including the panics.
        pub fn notify_reversed(&mut self, what: #enum_name) {
            self.notification(i32::from(what), true);
        }
    }
}

pub fn make_notification_enum(
    class_name: &TyName,
    all_bases: &[TyName],
    cfg_attributes: &TokenStream,
    ctx: &mut Context,
) -> (Option<TokenStream>, Ident) {
    let Some(all_constants) = ctx.notification_constants(class_name) else {
        // Class has no notification constants: reuse (direct/indirect) base enum
        return (None, ctx.notification_enum_name(class_name).name);
    };

    // Collect all notification constants from current and base classes
    let mut all_constants = all_constants.clone();
    for base_name in all_bases {
        if let Some(constants) = ctx.notification_constants(base_name) {
            all_constants.extend(constants.iter().cloned());
        }
    }

    workaround_constant_collision(&mut all_constants);

    let enum_name = ctx.notification_enum_name(class_name).name;
    let doc_str = format!(
        "Notification type for class [`{c}`][crate::classes::{c}].",
        c = class_name.rust_ty
    );

    let mut notification_enumerators_shout = Vec::new();
    let mut notification_enumerators_ord = Vec::new();
    for (constant_ident, constant_value) in all_constants {
        notification_enumerators_shout.push(constant_ident);
        notification_enumerators_ord.push(constant_value);
    }

    let code = quote! {
        #[doc = #doc_str]
        ///
        /// Makes it easier to keep an overview all possible notification variants for a given class, including
        /// notifications defined in base classes.
        ///
        /// Contains the [`Unknown`][Self::Unknown] variant for forward compatibility.
        #[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
        #[repr(i32)]
        #[allow(non_camel_case_types)]
        #cfg_attributes
        pub enum #enum_name {
            #(
                #notification_enumerators_shout = #notification_enumerators_ord,
            )*

            /// Since Godot represents notifications as integers, it's always possible that a notification outside the known types
            /// is received. For example, the user can manually issue notifications through `Object::notify()`.
            ///
            /// This is also necessary if you develop an extension on a Godot version and want to be forward-compatible with newer
            /// versions. If Godot adds new notifications, they will be unknown to your extension, but you can still handle them.
            Unknown(i32),
        }

        impl From<i32> for #enum_name {
            /// Always succeeds, mapping unknown integers to the `Unknown` variant.
            fn from(enumerator: i32) -> Self {
                match enumerator {
                    #(
                        #notification_enumerators_ord => Self::#notification_enumerators_shout,
                    )*
                    other_int => Self::Unknown(other_int),
                }
            }
        }

        impl From<#enum_name> for i32 {
            fn from(notification: #enum_name) -> i32 {
                match notification {
                    #(
                        #enum_name::#notification_enumerators_shout => #notification_enumerators_ord,
                    )*
                    #enum_name::Unknown(int) => int,
                }
            }
        }
    };

    (Some(code), enum_name)
}

/// Tries to interpret the constant as a notification one, and transforms it to a Rust identifier on success.
pub fn try_to_notification(constant: &JsonClassConstant) -> Option<Ident> {
    constant.name.strip_prefix("NOTIFICATION_").map(util::ident) // used to be conv::shout_to_pascal(s)
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Implementation

/// Workaround for Godot bug https://github.com/godotengine/godot/issues/75839, fixed in 4.2.
///
/// Godot has a collision for two notification constants (DRAW, NODE_CACHE_REQUESTED) in the same inheritance branch (as of 4.0.2).
/// This cannot be represented in a Rust enum, so we merge the two constants into a single enumerator.
fn workaround_constant_collision(all_constants: &mut Vec<(Ident, i32)>) {
    // This constant has never been used by the engine.
    #[cfg(before_api = "4.2")]
    all_constants.retain(|(constant_name, _)| constant_name != "NODE_RECACHE_REQUESTED");

    let _ = &all_constants; // unused warning
}
