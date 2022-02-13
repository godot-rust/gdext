use gdext_builtin::{
    string::GodotString, variant::Variant, vector2::Vector2, vector3::Vector3, PtrCallArg,
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
        RustTest { base: Node3D(base) }
    }
}

impl RustTest {
    fn test_method(&mut self, some_int: u64, some_string: GodotString) -> GodotString {
        let msg = format!("Hello from `RustTest.test_method()`, you passed some_int={some_int} and some_string={some_string}");
        msg.into()
    }

    fn add(&self, a: i32, b: i32) -> i64 {
        a as i64 + b as i64
    }
}

impl GodotExtensionClassMethods for RustTest {
    fn virtual_call(name: &str) -> sys::GDNativeExtensionClassCallVirtual {
        match name {
            "_ready" => Some({
                unsafe extern "C" fn call(
                    _inst: sys::GDExtensionClassInstancePtr,
                    _args: *const sys::GDNativeTypePtr,
                    _ret: sys::GDNativeTypePtr,
                ) {
                    eprintln!("hello!!");
                }
                call
            }),
            "_process" => Some({
                unsafe extern "C" fn call(
                    inst: sys::GDExtensionClassInstancePtr,
                    args: *const sys::GDNativeTypePtr,
                    _ret: sys::GDNativeTypePtr,
                ) {
                    let _inst = &mut *(inst as *mut RustTest);
                    let _delta = f64::from_ptr_call_arg(args.offset(0));

                    //dbg!(delta);
                }
                call
            }),
            _ => None,
        }
    }

    fn register_methods() {
        gdext_wrap_method!(RustTest,
            fn test_method(&mut self, some_int: u64, some_string: GodotString) -> GodotString
        );

        gdext_wrap_method!(RustTest,
            fn add(&self, a: i32, b: i32) -> i64
        );
    }
}

#[no_mangle]
unsafe extern "C" fn gdext_rust_test(
    interface: *const sys::GDNativeInterface,
    library: sys::GDNativeExtensionClassLibraryPtr,
    init: *mut sys::GDNativeInitialization,
) {
    sys::set_interface(interface);
    sys::set_library(library);

    *init = sys::GDNativeInitialization {
        minimum_initialization_level:
            sys::GDNativeInitializationLevel_GDNATIVE_INITIALIZATION_SCENE,
        userdata: std::ptr::null_mut(),
        initialize: Some(initialise),
        deinitialize: Some(deinitialise),
    };

    interface_fn!(print_warning)(
        b"Hello there!\0".as_ptr() as *const _,
        b"gdext_rust_test\0".as_ptr() as *const _,
        concat!(file!(), "\0").as_ptr() as *const _,
        line!() as _,
    );

    variant_tests();

    eprintln!("teeest");
}

unsafe extern "C" fn initialise(
    _userdata: *mut std::ffi::c_void,
    init_level: sys::GDNativeInitializationLevel,
) {
    eprintln!("hello from initialise with level {}", init_level);

    if init_level != sys::GDNativeInitializationLevel_GDNATIVE_INITIALIZATION_SCENE {
        return;
    }

    register_class::<RustTest>();
}

extern "C" fn deinitialise(
    _userdata: *mut std::ffi::c_void,
    init_level: sys::GDNativeInitializationLevel,
) {
    eprintln!("hello from deinitialise with level {}", init_level);
}

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
