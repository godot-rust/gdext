use gdext_class::out;
use gdext_macros::{godot_api, itest, GodotClass};

mod base_test;
mod gdscript_ffi_test;
mod object_test;
mod string_test;
mod variant_test;
mod virtual_methods_test;

fn run_tests() -> bool {
    let mut ok = true;
    ok &= base_test::run();
    ok &= gdscript_ffi_test::run();
    ok &= object_test::run();
    ok &= string_test::run();
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
#[godot(base=Node, init)]
struct IntegrationTests {}

#[godot_api]
impl IntegrationTests {
    #[godot]
    fn run(&mut self) -> bool {
        println!("Run Godot integration tests...");
        run_tests()
    }
}

gdext_builtin::gdext_init!(itest_init, |init: &mut gdext_builtin::InitOptions| {
    out!("itest_init()");
    init.register_init_function(gdext_builtin::InitLevel::Scene, || {
        out!("  register_class()");
        //gdext_class::register_class::<IntegrationTests>();
        gdext_class::auto_register_classes();
        //register_classes();
    });
});

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
                    gdext_builtin::gdext_print_error!("   !! Test {} failed", str_name);
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
