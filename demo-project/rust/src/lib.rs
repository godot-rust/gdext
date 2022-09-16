use gdext_builtin::{gdext_init, FromVariant, GodotString, InitLevel, ToVariant, Vector2, Vector3};
use std::str::FromStr;

use gdext_class::api::{Node3D, RefCounted};
use gdext_class::init::{ExtensionLib, InitHandle};
use gdext_class::obj::{Base, Gd, InstanceId};
use gdext_class::out;
use gdext_class::traits::{GodotExt, Share};
use gdext_macros::{gdextension, godot_api, GodotClass};

use gdext_sys as sys;

// ----------------------------------------------------------------------------------------------------------------------------------------------
// RustTest

#[derive(GodotClass, Debug)]
#[godot(base = Node3D)]
pub struct RustTest {
    #[base]
    base: Base<Node3D>,
    #[allow(dead_code)]
    time: f64,
}

#[godot_api]
impl RustTest {
    fn new(base: Base<Node3D>) -> Self {
        out!("[RustTest] construct: base={base:?}");

        Self { time: 0.0, base }
    }

    #[godot]
    fn test_method(&mut self, some_int: i64, some_string: GodotString) -> GodotString {
        //let id = Gd::emplace(self).instance_id();

        let some_string = some_string.clone();

        let msg = format!(
            "Hello from `RustTest.test_method()`:\
            \n\tyou passed some_int={some_int} and some_string={some_string}"
        );
        msg.into()
    }

    #[godot]
    fn add(&self, a: i32, b: i32, c: Vector2) -> i64 {
        a as i64 + b as i64 + c.inner().length() as i64
    }

    #[godot]
    fn vec_add(&self, a: Vector2, b: Vector2) -> Vector2 {
        Vector2::from_inner(a.inner() + b.inner())
    }

    // FIXME: allow mut params
    //fn accept_obj(&self, mut obj: Gd<Entity>) {

    #[godot]
    fn accept_obj(&self, obj: Gd<Entity>) {
        let mut obj = obj;

        //let obj = Gd::new(Entity { name: "h".to_string(), hitpoints: 77 }); // upcacsting local object works
        let up: Gd<RefCounted> = obj.share().upcast(); // FIXME Godot cast to RefCount panics
        out!("upcast: up={:?}", up);

        {
            let mut m = obj.bind_mut();
            m.hitpoints -= 10;
        }

        out!(
            "[RustTest] accept_obj:\n  id={},\n  obj={:?}",
            obj.instance_id(),
            obj.bind()
        );
    }

    #[godot]
    fn return_obj(&self) -> Gd<Entity> {
        let rust_obj = Entity {
            name: "New name!".to_string(),
            hitpoints: 20,
        };

        let obj = Gd::new(rust_obj);

        out!(
            "[RustTest] return_obj:\n  id={},\n  obj={:?}",
            obj.instance_id(),
            obj.bind()
        );

        obj
    }

    #[godot]
    fn find_obj(&self, instance_id: InstanceId) -> Gd<Entity> {
        out!("[RustTest] find_obj()...");

        let obj = Gd::<Entity>::try_from_instance_id(instance_id).expect("Gd is null");
        {
            let inner = obj.bind();
            out!(
                "[RustTest] find_obj():\n  id={},\n  obj={:?}",
                instance_id,
                inner
            );
        }
        obj
    }

    #[godot]
    fn call_base_method(&self) -> Vector3 {
        println!("to_global()...");
        //return Vector3::new(1.0, 2.0,3.0);

        let arg = Vector3::new(2.0, 3.0, 4.0);
        let res = self.base.to_global(arg);

        println!("to_global({arg}) == {res}");
        res
    }

    #[godot]
    fn call_node_method(&self, node: Gd<Node3D>) -> Vector3 {
        println!("call_node_method - to_global()...");
        println!("  instance_id: {}", node.instance_id());

        //let node = Gd::<Node3D>::from_instance_id(node.instance_id()).unwrap();
        let mut node = Node3D::new_alloc();
        let arg = Vector3::new(11.0, 22.0, 33.0);
        node.set_position(arg);

        let res = node.get_position();
        println!("  get_position() == {res}");

        let string = GodotString::from_str("hello string").unwrap();
        let copy = string.clone();
        let back = copy.to_string();
        let variant = copy.to_variant();
        // drop(back);
        // drop(copy);
        // drop(string);
        println!("var: {variant}");

        println!("<<");
        let back2 = GodotString::from_variant(&variant);

        println!(">>");

        println!(
            "string={}\ncopy=  {}\nback=  {}\nvar=   {}\nback2= {}",
            string, copy, back, variant, back2
        );

        res
    }
}

#[godot_api]
impl GodotExt for RustTest {
    fn init(base: Base<Self::Base>) -> Self {
        Self::new(base)
    }

    fn ready(&mut self) {
        out!("[RustTest] _ready()");
    }

    fn process(&mut self, delta: f64) {
        let mod_before = self.time % 1.0;
        self.time += delta;
        let mod_after = self.time % 1.0;

        if mod_before > mod_after {
            out!("[RustTest] _process(): {}", self.time);
        }
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Entity

#[derive(GodotClass, Debug)]
pub struct Entity {
    #[allow(dead_code)]
    name: String,
    hitpoints: i32,
}

#[godot_api]
impl GodotExt for Entity {
    fn init(base: Base<Self::Base>) -> Self {
        out!("[Entity] construct: base={base:?}");

        Entity {
            name: "No name yet".to_string(),
            hitpoints: 100,
        }
    }

    fn to_string(&self) -> GodotString {
        format!("{self:?}").into()
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Init + Test

struct Demo;

#[gdextension]
impl ExtensionLib for Demo {
    fn load_library(handle: &mut InitHandle) -> bool {
        todo!()
    }
}
