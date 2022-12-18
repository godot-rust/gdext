/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

#[cfg(all(test, not(gdext_clippy)))]
compile_error!("`cargo test` not supported for integration test -- use `cargo run`.");

use godot::bind::{godot_api, GodotClass};
use godot::init::{gdextension, ExtensionLibrary};
use godot::test::itest;
use std::panic::UnwindSafe;

mod base_test;
mod enum_test;
mod export_test;
mod gdscript_ffi_test;
mod node_test;
mod object_test;
mod singleton_test;
mod string_test;
mod utilities_test;
mod variant_test;
mod virtual_methods_test;

fn run_tests() -> bool {
    let mut ok = true;
    ok &= base_test::run();
    ok &= gdscript_ffi_test::run();
    ok &= node_test::run();
    ok &= enum_test::run();
    ok &= object_test::run();
    ok &= export_test::run();
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
    fn run(&mut self) -> bool {
        println!("Run Godot integration tests...");
        run_tests()
    }
}

#[gdextension(entry_point=itest_init)]
unsafe impl ExtensionLibrary for IntegrationTests {}

#[doc(hidden)]
#[macro_export]
macro_rules! godot_test_impl {
    ( $( $test_name:ident $body:block $($attrs:tt)* )* ) => {
        $(
            $($attrs)*
            #[doc(hidden)]
            #[inline]
            #[must_use]
            pub fn $test_name() -> bool {
                let str_name = stringify!($test_name);
                println!("   -- {}", str_name);

                let ok = ::std::panic::catch_unwind(
                    || $body
                ).is_ok();

                if !ok {
                    godot::log::godot_error!("   !! Test {} failed", str_name);
                }

                ok
            }
        )*
    }
}

/// Declares a test to be run with the Godot engine (i.e. not a pure Rust unit test).
///
/// Creates a wrapper function that catches panics, prints errors and returns true/false.
/// To be manually invoked in higher-level test routine.
///
/// This macro is designed to be used within the current crate only, hence the #[cfg] attribute.
#[doc(hidden)]
#[allow(unused_macros)]
macro_rules! godot_test {
    ($($test_name:ident $body:block)*) => {
        $(
            godot_test_impl!($test_name $body #[cfg(feature = "gd-test")]);
        )*
    }
}

/// Declares a test to be run with the Godot engine (i.e. not a pure Rust unit test).
///
/// Creates a wrapper function that catches panics, prints errors and returns true/false.
/// To be manually invoked in higher-level test routine.
///
/// This macro is designed to be used within the `test` crate, hence the method is always declared (not only in certain features).
#[doc(hidden)]
#[macro_export]
macro_rules! godot_itest {
    ($($test_name:ident $body:block)*) => {
        $(
            $crate::godot_test_impl!($test_name $body);
        )*
    };
    // Convenience
    ($(fn $test_name:ident () $body:block)*) => {
        $(
            $crate::godot_test_impl!($test_name $body);
        )*
    };
}

pub(crate) fn expect_panic(context: &str, code: impl FnOnce() + UnwindSafe) {
    let panic = std::panic::catch_unwind(code);
    assert!(
        panic.is_err(),
        "code should have panicked but did not: {}",
        context
    );
}
