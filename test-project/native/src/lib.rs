use gdext_builtin::{gdext_init, GodotString, InitLevel, Variant, Vector2, Vector3};
use std::str::FromStr;

use gdext_class::api::Node3D;
use gdext_class::*;

use gdext_sys as sys;

// ----------------------------------------------------------------------------------------------------------------------------------------------
// RustTest

#[derive(Debug)]
pub struct RustTest {
    base: Obj<Node3D>,
    time: f64,
}

impl GodotClass for RustTest {
    type Base = Node3D;
    type Declarer = marker::UserClass;

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

impl DefaultConstructible for RustTest {
    fn construct(base_ptr: sys::GDNativeObjectPtr) -> Self {
        out!("[RustTest] construct: base={base_ptr:?}");

        // FIXME build Rust object to represent Godot's own types, like Node3D
        //let obj = unsafe { Obj::from_sys(base) };
        let obj = unsafe { Obj::<Node3D>::from_obj_sys(base_ptr) };

        RustTest::new(obj)
    }
}

impl RustTest {
    // fn new(base: *mut std::ffi::c_void) -> Self {
    //     Self { time: 0.0 }
    // }

    fn new(base: Obj<Node3D>) -> Self {
        out!("[RustTest] new.");
        // out!("[RustTest] new: base={:?}", base.inner());

        Self { time: 0.0, base }
    }

    fn test_method(&mut self, some_int: i64, some_string: GodotString) -> GodotString {
        //let id = Obj::emplace(self).instance_id();

        let some_string = some_string.clone();

        let msg = format!(
            "Hello from `RustTest.test_method()`:\
            \n\tyou passed some_int={some_int} and some_string={some_string}"
        );
        msg.into()
    }

    fn add(&self, a: i32, b: i32, c: Vector2) -> i64 {
        a as i64 + b as i64 + c.inner().length() as i64
    }

    fn vec_add(&self, a: Vector2, b: Vector2) -> Vector2 {
        Vector2::from_inner(a.inner() + b.inner())
    }

    fn accept_obj(&self, mut obj: Obj<Entity>) {
        let m = obj.inner_mut();
        m.hitpoints -= 10;

        out!(
            "[RustTest] accept_obj:\n  id={},\n  obj={:?}",
            obj.instance_id(),
            obj.inner()
        );
    }

    fn return_obj(&self) -> Obj<Entity> {
        let rust_obj = Entity {
            name: "New name!".to_string(),
            hitpoints: 20,
        };

        let obj = Obj::new(rust_obj);

        out!(
            "[RustTest] return_obj:\n  id={},\n  obj={:?}",
            obj.instance_id(),
            obj.inner()
        );

        obj
    }

    fn find_obj(&self, instance_id: InstanceId) -> Obj<Entity> {
        out!("[RustTest] find_obj()...");

        let obj = Obj::<Entity>::try_from_instance_id(instance_id).expect("Obj is null");
        let inner = obj.inner();
        out!(
            "[RustTest] find_obj():\n  id={},\n  obj={:?}",
            instance_id,
            inner
        );
        obj
    }

    fn call_base_method(&self) -> Vector3 {
        println!("to_global()...");
        //return Vector3::new(1.0, 2.0,3.0);

        let arg = Vector3::new(2.0, 3.0, 4.0);
        let res = self.base.inner().to_global(arg);

        println!("to_global({arg}) == {res}");
        res
    }

    fn call_node_method(&self, node: Obj<Node3D>) -> Vector3 {
        println!("call_node_method - to_global()...");
        println!("  instance_id: {}", node.instance_id());

        //let node = Obj::<Node3D>::from_instance_id(node.instance_id()).unwrap();
        let node = Node3D::new();
        let inner = node.inner();
        let arg = Vector3::new(11.0, 22.0, 33.0);
        inner.set_position(arg);

        let res = inner.get_position();
        println!("  get_position() == {res}");

        let string = GodotString::from_str("hello string").unwrap();
        let copy = string.clone();
        let back = copy.to_string();
        let variant = Variant::from(&copy);
        // drop(back);
        // drop(copy);
        // drop(string);
        println!("var: {variant}");

        println!("<<");
        let back2 = GodotString::from(&variant);

        println!(">>");

        println!(
            "string={}\ncopy=  {}\nback=  {}\nvar=   {}\nback2= {}",
            string, copy, back, variant, back2
        );

        res
    }

    fn _ready(&mut self) {
        out!("[RustTest] _ready()");
    }

    fn _process(&mut self, delta: f64) {
        let mod_before = self.time % 1.0;
        self.time += delta;
        let mod_after = self.time % 1.0;

        if mod_before > mod_after {
            out!("[RustTest] _process(): {}", self.time);
        }
    }
}

impl GodotExtensionClass for RustTest {
    fn virtual_call(name: &str) -> sys::GDNativeExtensionClassCallVirtual {
        out!("[RustTest] virtual_call: {name}");

        match name {
            "_ready" => gdext_virtual_method_body!(RustTest, fn _ready(&mut self)),
            "_process" => gdext_virtual_method_body!(RustTest, fn _process(&mut self, delta: f64)),
            _ => None,
        }
    }

    fn register_methods() {
        out!("[RustTest] register_methods");

        gdext_wrap_method!(RustTest,
            fn accept_obj(&self, obj: Obj<Entity>)
        );

        gdext_wrap_method!(RustTest,
            fn return_obj(&self) -> Obj<Entity>
        );

        gdext_wrap_method!(RustTest,
            fn find_obj(&self, instance_id: InstanceId) -> Obj<Entity>
        );

        gdext_wrap_method!(RustTest,
            fn test_method(&mut self, some_int: i64, some_string: GodotString) -> GodotString
        );

        gdext_wrap_method!(RustTest,
            fn add(&self, a: i32, b: i32, c: Vector2) -> i64
        );

        gdext_wrap_method!(RustTest,
            fn vec_add(&self, a: Vector2, b: Vector2) -> Vector2
        );

        gdext_wrap_method!(RustTest,
            fn call_base_method(&self) -> Vector3
        );

        gdext_wrap_method!(RustTest,
            fn call_node_method(&self, node: Obj<Node3D>) -> Vector3
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

impl DefaultConstructible for Entity {
    fn construct(base: sys::GDNativeObjectPtr) -> Self {
        out!("[Entity] construct: base={base:?}");

        Entity {
            name: "No name yet".to_string(),
            hitpoints: 100,
        }
    }
}

impl GodotClass for Entity {
    type Base = gdext_class::api::RefCounted;
    type Declarer = gdext_class::traits::marker::UserClass;

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
    //     out!("[Entity] construct");
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

    fn virtual_call(name: &str) -> sys::GDNativeExtensionClassCallVirtual {
        out!("[Entity] virtual_call: {name}");
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
