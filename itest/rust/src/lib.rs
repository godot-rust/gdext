use gdext_class::dom::UserDomain;
use gdext_class::{
    api, gdext_register_method, gdext_virtual_method_callback, out, traits, Base, GodotClass,
    GodotDefault, GodotMethods, UserMethodBinds, UserVirtuals,
};
use gdext_macros::itest;

mod gdscript_ffi_test;
mod object_test;
mod string_test;
mod variant_test;
mod virtual_methods_test;

fn run_tests() -> bool {
    let mut ok = true;
    ok &= object_test::run();
    ok &= variant_test::run();
    ok &= string_test::run();
    ok &= gdscript_ffi_test::run();
    ok &= virtual_methods_test::run();
    ok
}

fn register_classes() {
    object_test::register();
    gdscript_ffi_test::register();
    virtual_methods_test::register();
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
    type Declarer = UserDomain;
    type Mem = traits::mem::ManualMemory;

    fn class_name() -> String {
        "IntegrationTests".to_string()
    }
}

impl UserMethodBinds for IntegrationTests {
    fn register_methods() {
        out!("[IntegrationTests] register_methods");

        gdext_register_method!(IntegrationTests, fn run(&mut self) -> bool;);
    }
}

impl UserVirtuals for IntegrationTests {
    fn virtual_call(name: &str) -> gdext_sys::GDNativeExtensionClassCallVirtual {
        out!("[IntegrationTests] virtual_call: {name}");

        match name {
            "run" => gdext_virtual_method_callback!(IntegrationTests, fn run(&mut self) -> bool),
            _ => None,
        }
    }
}

impl GodotDefault for IntegrationTests {
    fn godot_default(_base: Base<Self::Base>) -> Self {
        Self {}
    }
}

impl GodotMethods for IntegrationTests {}

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
