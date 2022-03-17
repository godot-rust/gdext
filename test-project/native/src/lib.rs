use gdext_builtin::{
    gdext_init, gdext_print_warning, string::GodotString, variant::Variant, vector2::Vector2,
    vector3::Vector3, InitLevel,
};
use gdext_class::*;
use gdext_sys::{self as sys, interface_fn};

pub struct Node3D(sys::GDNativeObjectPtr);

impl GodotClass for Node3D {
    type Base = Node3D;

    fn class_name() -> String {
        "Node3D".to_string()
    }

    fn native_object_ptr(&self) -> sys::GDNativeObjectPtr {
        self.0
    }

    fn upcast(&self) -> &Self::Base {
        self
    }

    fn upcast_mut(&mut self) -> &mut Self::Base {
        self
    }
}

pub struct RustTest {
    base: Node3D,
    time: f64,
}

impl GodotClass for RustTest {
    type Base = Node3D;

    fn class_name() -> String {
        "RustTest".to_string()
    }

    fn upcast(&self) -> &Self::Base {
        &self.base
    }

    fn upcast_mut(&mut self) -> &mut Self::Base {
        &mut self.base
    }
}

impl GodotExtensionClass for RustTest {
    fn construct(base: sys::GDNativeObjectPtr) -> Self {
        RustTest {
            base: Node3D(base),
            time: 0.0,
        }
    }
}

impl RustTest {
    fn test_method(&mut self, some_int: u64, some_string: GodotString) -> GodotString {
        let msg = format!("Hello from `RustTest.test_method()`, you passed some_int={some_int} and some_string={some_string}");
        msg.into()
    }

    fn add(&self, a: i32, b: i32, c: Vector2) -> i64 {
        a as i64 + b as i64 + c.length() as i64
    }

    fn print_int(&self, i: i8) {
        println!("printing int: {i}");
    }

    fn vec_add(&self, a: Vector2, b: Vector2) -> Vector2 {
        a + b
    }

    fn _ready(&mut self) {
        gdext_print_warning!("Hello from _ready()!");
    }

    fn _process(&mut self, delta: f64) {
        let mod_before = self.time % 1.0;
        self.time += delta;
        let mod_after = self.time % 1.0;

        if mod_before > mod_after {
            eprintln!("Boop! {}", self.time);
        }
    }
}

impl GodotExtensionClassMethods for RustTest {
    fn virtual_call(name: &str) -> sys::GDNativeExtensionClassCallVirtual {
        match name {
            "_ready" => gdext_virtual_method_body!(RustTest, fn _ready(&mut self)),
            "_process" => gdext_virtual_method_body!(RustTest, fn _process(&mut self, delta: f64)),
            _ => None,
        }
    }

    fn register_methods() {
        gdext_wrap_method!(RustTest,
            fn test_method(&mut self, some_int: u64, some_string: GodotString) -> GodotString
        );

        gdext_wrap_method!(RustTest,
            fn add(&self, a: i32, b: i32, c: Vector2) -> i64
        );

        gdext_wrap_method!(RustTest,
            fn print_int(&self, i: i8)
        );

        gdext_wrap_method!(RustTest,
            fn vec_add(&self, a: Vector2, b: Vector2) -> Vector2
        );
    }
}

gdext_init!(gdext_rust_test, |init: &mut gdext_builtin::InitOptions| {
    init.register_init_function(InitLevel::Scene, || {
        register_class::<RustTest>();

        variant_tests();
    });
});

fn variant_tests() {
    let _v = Variant::nil();

    let _v = Variant::from(false);

    {
        let vec = Vector2::new(1.0, 4.0);
        let vec_var = Variant::from(vec);

        dbg!(Vector2::from(&vec_var));
    }

    {
        let vec = Vector3::new(1.0, 4.0, 6.0);
        let vec_var = Variant::from(vec);

        dbg!(Vector3::from(&vec_var));
    }

    {
        let s = GodotString::from("Hello from Rust! ♥");
        dbg!(s.to_string());
    }

    {
        let s = GodotString::new();
        dbg!(s.to_string());
    }

    {
        let x = Variant::from(12u32);
        dbg!(u32::from(&x));
    }

    {
        let x = Variant::from(true);
        dbg!(bool::from(&x));
    }
}
