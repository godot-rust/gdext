use gdext_builtin::{
    gdext_init, string::GodotString, variant::Variant, vector2::Vector2, vector3::Vector3,
    InitLevel,
};
use gdext_class::*;
use gdext_sys::{self as sys, interface_fn};
use std::ffi::c_void;

#[cfg(feature = "trace")]
macro_rules! log {
    ()                          => (println!());
    ($fmt:literal)              => (println!($fmt));
    ($fmt:literal, $($arg:tt)*) => (println!($fmt, $($arg)*);)
}

#[cfg(not(feature = "trace"))]
// TODO find a better way than sink-writing to avoid warnings, #[allow(unused_variables)] doesn't work
macro_rules! log {
    ()                          => ({});
    ($fmt:literal)              => ({ use std::io::{sink, Write}; let _ = write!(sink(), $fmt); });
    ($fmt:literal, $($arg:tt)*) => ({ use std::io::{sink, Write}; let _ = write!(sink(), $fmt, $($arg)*); };)
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Node3D (base)

pub struct Node3D(sys::GDNativeObjectPtr);

impl GodotClass for Node3D {
    type Base = Node3D;

    fn class_name() -> String {
        "Node3D".to_string()
    }

    // fn native_object_ptr(&self) -> sys::GDNativeObjectPtr {
    //     self.0
    // }

    // fn upcast(&self) -> &Self::Base {
    //     self
    // }
    //
    // fn upcast_mut(&mut self) -> &mut Self::Base {
    //     self
    // }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// RefCounted (base)

#[derive(Debug)]
pub struct RefCounted(sys::GDNativeObjectPtr);

impl GodotClass for RefCounted {
    type Base = RefCounted;

    fn class_name() -> String {
        "RefCounted".to_string()
    }

    // fn native_object_ptr(&self) -> sys::GDNativeObjectPtr {
    //     self.0
    // }

    // fn upcast(&self) -> &Self::Base {
    //     self
    // }
    //
    // fn upcast_mut(&mut self) -> &mut Self::Base {
    //     self
    // }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// RustTest

pub struct RustTest {
    //base: Node3D,
    time: f64,
}

impl GodotClass for RustTest {
    type Base = Node3D;

    fn class_name() -> String {
        "RustTest".to_string()
    }

    // fn upcast(&self) -> &Self::Base {
    //     &self.base
    // }
    //
    // fn upcast_mut(&mut self) -> &mut Self::Base {
    //     &mut self.base
    // }
}

impl GodotMethods for RustTest {
    fn construct(base: *mut std::ffi::c_void) -> Self {
        log!("[RustTest] construct: base={base:?}");
        RustTest::new(base)
    }
}

impl GodotExtensionClass for RustTest {
    // fn construct(base: sys::GDNativeObjectPtr) -> Self {
    //     log!("[RustTest] construct");
    //
    //     RustTest {
    //         base: Node3D(base),
    //         time: 0.0,
    //     }
    // }
}

impl RustTest {
    fn new(base: *mut std::ffi::c_void) -> Self {
        Self { time: 0.0 }
    }

    fn test_method(&mut self, some_int: u64, some_string: GodotString) -> GodotString {
        //let id = Obj::emplace(self).instance_id();

        let some_string = some_string.clone();

        let msg = format!(
            "Hello from `RustTest.test_method()`:\
            \n\tyou passed some_int={some_int} and some_string={some_string}"
        );
        msg.into()
    }

    fn add(&self, a: i32, b: i32, c: Vector2) -> i64 {
        a as i64 + b as i64 + c.length() as i64
    }

    fn vec_add(&self, a: Vector2, b: Vector2) -> Vector2 {
        a + b
    }

    fn accept_obj(&self, obj: Obj<Entity>) {
        //log!("[RustTest] accept_obj: id={:x}, dec={}", obj.instance_id(), obj.instance_id() as i64);
        log!(
            "[RustTest] accept_obj:\n  id={},\n  obj={:?}",
            obj.instance_id() as i64,
            obj.inner()
        );
    }

    fn return_obj(&self) -> Obj<Entity> {
        log!("[RustTest] return_obj()");

        let rust_obj = Entity {
            name: "New name!".to_string(),
            hitpoints: 20,
        };

        Obj::new(rust_obj)
    }

    fn find_obj(&self, instance_id: u64) -> Obj<Entity> {
        let obj = Obj::from_instance_id(instance_id).expect("Obj is null");
        let inner = obj.inner();
        log!(
            "[RustTest] find_obj():\n  id={},\n  obj={:?}",
            instance_id,
            inner
        );
        obj
    }

    fn _ready(&mut self) {
        log!("[RustTest] _ready()");
    }

    fn _process(&mut self, delta: f64) {
        let mod_before = self.time % 1.0;
        self.time += delta;
        let mod_after = self.time % 1.0;

        if mod_before > mod_after {
            log!("[RustTest] _process(): {}", self.time);
        }
    }
}

impl GodotExtensionClassMethods for RustTest {
    fn virtual_call(name: &str) -> sys::GDNativeExtensionClassCallVirtual {
        log!("[RustTest] virtual_call: {name}");

        match name {
            "_ready" => gdext_virtual_method_body!(RustTest, fn _ready(&mut self)),
            "_process" => gdext_virtual_method_body!(RustTest, fn _process(&mut self, delta: f64)),
            _ => None,
        }
    }

    fn register_methods() {
        log!("[RustTest] register_methods");

        gdext_wrap_method!(RustTest,
            fn accept_obj(&self, obj: Obj<Entity>)
        );

        gdext_wrap_method!(RustTest,
            fn return_obj(&self) -> Obj<Entity>
        );

        gdext_wrap_method!(RustTest,
            fn find_obj(&self, instance_id: u64) -> Obj<Entity>
        );

        gdext_wrap_method!(RustTest,
            fn test_method(&mut self, some_int: u64, some_string: GodotString) -> GodotString
        );

        gdext_wrap_method!(RustTest,
            fn add(&self, a: i32, b: i32, c: Vector2) -> i64
        );

        gdext_wrap_method!(RustTest,
            fn vec_add(&self, a: Vector2, b: Vector2) -> Vector2
        );
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Entity

#[derive(Debug)]
#[allow(dead_code)] // TODO
pub struct Entity {
    // base: RefCounted,
    name: String,
    hitpoints: i32,
}

impl GodotMethods for Entity {
    fn construct(base: *mut c_void) -> Self {
        log!("[Entity] construct: base={base:?}");

        Entity {
            name: "No name yet".to_string(),
            hitpoints: 100,
        }
    }
}

impl GodotClass for Entity {
    type Base = RefCounted;

    fn class_name() -> String {
        "Entity".to_string()
    }

    // fn upcast(&self) -> &Self::Base {
    //     todo!()
    //     //&self.base
    // }
    //
    // fn upcast_mut(&mut self) -> &mut Self::Base {
    //     //&mut self.base
    //     todo!()
    // }
}

impl GodotExtensionClass for Entity {
    // fn construct(base: sys::GDNativeObjectPtr) -> Self {
    //     log!("[Entity] construct");
    //
    //     Entity {
    //         base: RefCounted(base),
    //         name: "No name yet".to_string(),
    //         hitpoints: 100,
    //     }
    // }

    fn has_to_string() -> bool {
        true
    }
}

impl GodotExtensionClassMethods for Entity {
    fn virtual_call(name: &str) -> sys::GDNativeExtensionClassCallVirtual {
        log!("[Entity] virtual_call: {name}");
        match name {
            //"xy" => {
            //    gdext_virtual_method_body!(Entity, fn xy(&mut self))
            //}
            _ => None,
        }
    }

    fn register_methods() {
        gdext_wrap_method!(Entity,
            fn _to_string(&mut self) -> GodotString
        );
    }

    fn to_string(&self) -> GodotString {
        return self._to_string();
    }
}

impl Entity {
    fn _to_string(&self) -> GodotString {
        format!("{self:?}").into()
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Init + Test

gdext_init!(gdext_rust_test, |init: &mut gdext_builtin::InitOptions| {
    init.register_init_function(InitLevel::Scene, || {
        register_class::<RustTest>();
        register_class::<Entity>();

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
