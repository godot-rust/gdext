use gdext_class::marker::UserClass;
use gdext_class::{
    api, gdext_virtual_method_body, gdext_wrap_method, out, GodotClass, GodotExtensionClass,
    GodotExtensionClassMethods, GodotMethods,
};
mod variant_test;

fn run_tests() -> bool {
    let mut ok = true;
    ok &= variant_test::run();

    ok
}
// ----------------------------------------------------------------------------------------------------------------------------------------------
// Implementation

#[derive(Debug)]
struct IntegrationTests {}

impl IntegrationTests {
    fn run(&mut self) -> bool {
        println!("Run Godot integration tests...");
        run_tests()
    }
}

impl GodotClass for IntegrationTests {
    type Base = api::Node;
    type Declarer = UserClass;

    fn class_name() -> String {
        "IntegrationTests".to_string()
    }
}

impl GodotExtensionClass for IntegrationTests {}

impl GodotExtensionClassMethods for IntegrationTests {
    fn virtual_call(name: &str) -> gdext_sys::GDNativeExtensionClassCallVirtual {
        out!("[IntegrationTests] virtual_call: {name}");

        match name {
            "run" => gdext_virtual_method_body!(IntegrationTests, fn run(&mut self) -> bool),
            _ => None,
        }
    }

    fn register_methods() {
        out!("[IntegrationTests] register_methods");

        gdext_wrap_method!(IntegrationTests, fn run(&mut self) -> bool);
    }
}

impl GodotMethods for IntegrationTests {
    fn construct(_base: gdext_sys::GDNativeObjectPtr) -> Self {
        Self {}
    }
}

gdext_builtin::gdext_init!(itest_init, |init: &mut gdext_builtin::InitOptions| {
    out!("itest_init()");
    init.register_init_function(gdext_builtin::InitLevel::Scene, || {
        out!("  register_class()");
        gdext_class::register_class::<IntegrationTests>();
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
    }
}
