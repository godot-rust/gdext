/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use godot::bind::{godot_api, GodotClass};
use godot::init::{gdextension, ExtensionLibrary};
use godot::test::itest;
use std::panic::UnwindSafe;

mod array_test;
mod base_test;
mod builtin_test;
mod codegen_test;
mod dictionary_test;
mod enum_test;
mod export_test;
mod gdscript_ffi_test;
mod node_test;
mod object_test;
mod packed_array_test;
mod quaternion_test;
mod singleton_test;
mod string_test;
mod utilities_test;
mod variant_test;
mod virtual_methods_test;

fn run_tests() -> bool {
    let mut ok = true;
    ok &= array_test::run();
    ok &= base_test::run();
    ok &= builtin_test::run();
    ok &= codegen_test::run();
    ok &= dictionary_test::run();
    ok &= enum_test::run();
    ok &= export_test::run();
    ok &= gdscript_ffi_test::run();
    ok &= node_test::run();
    ok &= object_test::run();
    ok &= packed_array_test::run();
    ok &= quaternion_test::run();
    ok &= singleton_test::run();
    ok &= string_test::run();
    ok &= utilities_test::run();
    ok &= variant_test::run();
    ok &= virtual_methods_test::run();
    ok
}

// fn register_classes() {
//     object_test::register();
//     gdscript_ffi_test::register();
//     virtual_methods_test::register();
// }

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Implementation

#[derive(GodotClass, Debug)]
#[class(base=Node, init)]
struct IntegrationTests {}

#[godot_api]
impl IntegrationTests {
    #[func]
    fn test_all(&mut self) -> bool {
        println!("Run Godot integration tests...");
        run_tests()
    }
}

#[gdextension(entry_point=itest_init)]
unsafe impl ExtensionLibrary for IntegrationTests {}

pub(crate) fn expect_panic(context: &str, code: impl FnOnce() + UnwindSafe) {
    let panic = std::panic::catch_unwind(code);
    assert!(
        panic.is_err(),
        "code should have panicked but did not: {context}",
    );
}
