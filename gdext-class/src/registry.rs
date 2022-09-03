#![allow(dead_code)] // FIXME

use crate::private::as_storage;
use crate::storage::InstanceStorage;
use crate::traits::*;
use std::collections::HashMap;
use std::fmt::Formatter;
use std::ptr;

use gdext_sys as sys;
use sys::interface_fn;

#[derive(Debug)]
pub struct ClassPlugin {
    pub class_name: &'static str,
    pub component: PluginComponent,
}

/// Type-erased function object, holding a `register_class` function.
#[derive(Copy, Clone)]
pub struct ErasedRegisterFn {
    // Wrapper needed because Debug can't be derived on function pointers with reference parameters, so this won't work:
    // pub type ErasedRegisterFn = fn(&mut dyn std::any::Any);
    // (see https://stackoverflow.com/q/53380040)
    pub raw: fn(&mut dyn std::any::Any),
}

impl std::fmt::Debug for ErasedRegisterFn {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "0x{:0>16x}", self.raw as u64)
    }
}

#[derive(Debug)]
pub enum PluginComponent {
    /// Class definition itself, must always be available
    ClassDef {
        base_class_name: &'static str,

        generated_create_fn: Option<
            unsafe extern "C" fn(
                _class_userdata: *mut std::ffi::c_void, //
            ) -> sys::GDNativeObjectPtr,
        >,

        free_fn: unsafe extern "C" fn(
            _class_user_data: *mut std::ffi::c_void,
            instance: sys::GDExtensionClassInstancePtr,
        ),
    },

    /// Collected from `#[godot_api] impl MyClass`
    UserMethodBinds {
        /// Callback to library-generated function which registers functions in the `impl`
        ///
        /// Always present since that's the entire point of this `impl` block.
        generated_register_fn: ErasedRegisterFn,
    },

    /// Collected from `#[godot_api] impl GodotMethods for MyClass`
    UserVirtuals {
        /// Callback to user-defined `register_class` function
        user_register_fn: Option<ErasedRegisterFn>,

        /// User-defined `init` function
        user_create_fn: Option<
            unsafe extern "C" fn(
                _class_userdata: *mut std::ffi::c_void, //
            ) -> sys::GDNativeObjectPtr,
        >,

        /// User-defined `to_string` function
        user_to_string_fn: Option<
            unsafe extern "C" fn(
                instance: sys::GDExtensionClassInstancePtr,
                out_string: sys::GDNativeStringPtr,
            ),
        >,

        /// Callback for other virtuals
        get_virtual_fn: unsafe extern "C" fn(
            _class_user_data: *mut std::ffi::c_void,
            p_name: *const std::os::raw::c_char,
        ) -> sys::GDNativeExtensionClassCallVirtual,
    },
}

pub fn register_class<T: UserMethodBinds + UserVirtuals + GodotMethods>() {
    println!("Manually register class {}", std::any::type_name::<T>());

    let godot_params = sys::GDNativeExtensionClassCreationInfo {
        set_func: None,
        get_func: None,
        get_property_list_func: None,
        free_property_list_func: None,
        property_can_revert_func: None,
        property_get_revert_func: None,
        notification_func: None,
        to_string_func: if T::has_to_string() {
            Some(callbacks::to_string::<T>)
        } else {
            None
        },
        reference_func: Some(callbacks::reference::<T>),
        unreference_func: Some(callbacks::unreference::<T>),
        create_instance_func: Some(callbacks::create::<T>),
        free_instance_func: Some(callbacks::free::<T>),
        get_virtual_func: Some(callbacks::get_virtual::<T>),
        get_rid_func: None,
        class_userdata: ptr::null_mut(), // will be passed to create fn, but global per class
    };

    let class_name = Some(ClassName::new::<T>());
    let parent_class_name = Some(ClassName::new::<T::Base>());

    register_class_raw(ClassRegistrationInfo {
        class_name,
        parent_class_name,
        generated_register_fn: None,
        user_register_fn: Some(ErasedRegisterFn {
            raw: callbacks::register_class_by_builder::<T>,
        }),
        godot_params,
    });
}

struct ClassRegistrationInfo {
    class_name: Option<ClassName>,
    parent_class_name: Option<ClassName>,
    generated_register_fn: Option<ErasedRegisterFn>,
    user_register_fn: Option<ErasedRegisterFn>,
    godot_params: sys::GDNativeExtensionClassCreationInfo,
}

/// Lets Godot know about all classes that have self-registered through the plugin system.
pub fn auto_register_classes() {
    println!("Auto-register classes...");

    // Note: many errors are already caught by the compiler, before this runtime validation even takes place:
    // * missing #[derive(GodotClass)] or impl GodotClass for T
    // * duplicate impl GodotInit for T
    //

    let mut map = HashMap::<ClassName, ClassRegistrationInfo>::new();

    crate::private::iterate_plugins(|elem: &ClassPlugin| {
        println!("* Plugin: {elem:#?}");

        let name = ClassName::from_static(elem.class_name);
        let c = map
            .entry(name.clone())
            .or_insert_with(default_creation_info);
        c.class_name = Some(name);

        match elem.component {
            PluginComponent::ClassDef {
                base_class_name,
                generated_create_fn,
                free_fn,
            } => {
                c.parent_class_name = Some(ClassName::from_static(base_class_name));
                c.godot_params.create_instance_func = generated_create_fn;
                c.godot_params.free_instance_func = Some(free_fn);
            }

            PluginComponent::UserMethodBinds {
                generated_register_fn,
            } => {
                c.generated_register_fn = Some(generated_register_fn);
            }

            PluginComponent::UserVirtuals {
                user_register_fn,
                user_create_fn,
                user_to_string_fn,
                get_virtual_fn,
            } => {
                c.user_register_fn = user_register_fn;
                c.godot_params.create_instance_func = user_create_fn;
                c.godot_params.to_string_func = user_to_string_fn;
                c.godot_params.get_virtual_func = Some(get_virtual_fn);
            }
        }
    });

    for info in map.into_values() {
        register_class_raw(info);
    }

    println!("All classes auto-registered.");
}

fn register_class_raw(info: ClassRegistrationInfo) {
    unsafe {
        interface_fn!(classdb_register_extension_class)(
            sys::get_library(),
            info.class_name.expect("class defined (class_name)").c_str(),
            info.parent_class_name
                .expect("class defined (parent_class_name)")
                .c_str(),
            ptr::addr_of!(info.godot_params),
        );
    }
}

/// Utility to convert `String` to C `const char*`.
/// Cannot be a function since the backing string must be retained.
#[derive(Eq, PartialEq, Hash, Clone)]
pub(crate) struct ClassName {
    backing: String,
}

impl ClassName {
    pub fn new<T: GodotClass>() -> Self {
        Self {
            backing: format!("{}\0", T::class_name()),
        }
    }

    fn from_static(string: &'static str) -> Self {
        Self {
            backing: format!("{}\0", string),
        }
    }

    pub fn c_str(&self) -> *const std::os::raw::c_char {
        self.backing.as_ptr() as *const _
    }
}

pub mod callbacks {
    use super::*;
    use crate::builder::ClassBuilder;

    pub unsafe extern "C" fn create<T: GodotClass>(
        _class_userdata: *mut std::ffi::c_void,
    ) -> sys::GDNativeObjectPtr {
        let class_name = ClassName::new::<T>();
        let base_class_name = ClassName::new::<T::Base>();

        let base = interface_fn!(classdb_construct_object)(base_class_name.c_str());
        let instance = InstanceStorage::<T>::construct_uninit(base);
        let instance_ptr = instance.into_raw();
        let instance_ptr = instance_ptr as *mut std::ffi::c_void;

        interface_fn!(object_set_instance)(base, class_name.c_str(), instance_ptr);

        let binding_data_callbacks = crate::storage::nop_instance_callbacks();

        interface_fn!(object_set_instance_binding)(
            base,
            sys::get_library(),
            instance_ptr,
            &binding_data_callbacks,
        );

        base
    }

    pub unsafe extern "C" fn free<T: GodotClass>(
        _class_user_data: *mut std::ffi::c_void,
        instance: sys::GDExtensionClassInstancePtr,
    ) {
        let storage = as_storage::<T>(instance);
        storage.mark_destroyed_by_godot();
        Box::from_raw(storage as *mut InstanceStorage<_>); // aka. drop
    }

    pub unsafe extern "C" fn get_virtual<T: UserVirtuals>(
        _class_user_data: *mut std::ffi::c_void,
        p_name: *const std::os::raw::c_char,
    ) -> sys::GDNativeExtensionClassCallVirtual {
        let name = std::ffi::CStr::from_ptr(p_name);
        T::virtual_call(name.to_str().expect("T::virtual_call"))
    }

    pub unsafe extern "C" fn to_string<T: GodotMethods>(
        instance: sys::GDExtensionClassInstancePtr,
        out_string: sys::GDNativeStringPtr,
    ) {
        let storage = as_storage::<T>(instance);
        let instance = storage.get();
        let string = GodotMethods::to_string(instance);

        // Transfer ownership to Godot, disable destructor
        string.write_string_sys(out_string);
        std::mem::forget(string);
    }

    pub unsafe extern "C" fn reference<T: GodotClass>(instance: sys::GDExtensionClassInstancePtr) {
        let storage = as_storage::<T>(instance);
        storage.on_inc_ref();
    }

    pub unsafe extern "C" fn unreference<T: GodotClass>(
        instance: sys::GDExtensionClassInstancePtr,
    ) {
        let storage = as_storage::<T>(instance);
        storage.on_dec_ref();
    }

    pub fn register_class_by_builder<T: GodotClass + GodotMethods>(
        class_builder: &mut dyn std::any::Any,
    ) {
        let class_builder = class_builder
            .downcast_mut::<ClassBuilder<T>>()
            .expect("bad type erasure");

        T::register_class(class_builder);
    }
}

// Substitute for Default impl
// Yes, bindgen can implement Default, but only for _all_ types (with single exceptions).
// For FFI types, it's better to have explicit initialization in the general case though.
fn default_creation_info() -> ClassRegistrationInfo {
    ClassRegistrationInfo {
        class_name: None,
        parent_class_name: None,
        generated_register_fn: None,
        user_register_fn: None,
        godot_params: sys::GDNativeExtensionClassCreationInfo {
            set_func: None,
            get_func: None,
            get_property_list_func: None,
            free_property_list_func: None,
            property_can_revert_func: None,
            property_get_revert_func: None,
            notification_func: None,
            to_string_func: None,
            reference_func: None,
            unreference_func: None,
            create_instance_func: None,
            free_instance_func: None,
            get_virtual_func: None,
            get_rid_func: None,
            class_userdata: ptr::null_mut(),
        },
    }
}
