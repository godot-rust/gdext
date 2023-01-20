/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::builder::ClassBuilder;
use crate::builtin::GodotString;
use crate::obj::Base;
use crate::obj::GodotClass;

/// Extension API for Godot classes, used with `#[godot_api]`.
///
/// Helps with adding custom functionality:
/// * `init` constructors
/// * `to_string` method
/// * Custom register methods (builder style)
/// * All the lifecycle methods like `ready`, `process` etc.
///
/// This trait is special in that it needs to be used in combination with the `#[godot_api]`
/// proc-macro attribute to ensure proper registration of its methods. All methods have
/// default implementations, so you can select precisely which functionality you want to have.
/// Those default implementations are never called however, the proc-macro detects what you implement.
///
/// Do not call any of these methods directly -- they are an interface to Godot. Functionality
/// described here is available through other means (e.g. `init` via `Gd::new_default`).
#[allow(unused_variables)]
#[allow(clippy::unimplemented)] // TODO consider using panic! with specific message, possibly generated code
pub trait GodotExt: crate::private::You_forgot_the_attribute__godot_api
where
    Self: GodotClass,
{
    // Note: keep in sync with VIRTUAL_METHOD_NAMES in godot_api.rs

    // Some methods that were called:
    // _enter_tree
    // _input
    // _shortcut_input
    // _unhandled_input
    // _unhandled_key_input
    // _process
    // _physics_process
    // _ready

    fn register_class(builder: &mut ClassBuilder<Self>) {}

    fn init(base: Base<Self::Base>) -> Self {
        unimplemented!()
    }

    fn ready(&mut self) {
        unimplemented!()
    }
    fn process(&mut self, delta: f64) {
        unimplemented!()
    }
    fn physics_process(&mut self, delta: f64) {
        unimplemented!()
    }
    fn to_string(&self) -> GodotString {
        unimplemented!()
    }
}
