#![allow(dead_code)] // FIXME

use crate::private::as_storage;
use crate::storage::InstanceStorage;
use crate::traits::*;

use gdext_sys as sys;
use sys::interface_fn;

pub struct ClassPlugin {
    pub class_name: &'static str,
    pub component: PluginComponent,
}

pub enum PluginComponent {
    /// Class definition itself, must always be available
    Basic {
        base_class_name: &'static str,

        default_create_fn: Option<
            unsafe extern "C" fn(
                _class_userdata: *mut std::ffi::c_void, //
            ) -> sys::GDNativeObjectPtr,
        >,

        free_fn: unsafe extern "C" fn(
            _class_user_data: *mut std::ffi::c_void,
            instance: sys::GDExtensionClassInstancePtr,
        ),
    },

    /// Constructor defined by user
    UserConstruct {
        create_fn: unsafe extern "C" fn(
            _class_userdata: *mut std::ffi::c_void, //
        ) -> sys::GDNativeObjectPtr,
    },

    /// Methods in `#[godot_api] impl MyClass`
    UserMethodBinds {
        registration_method: fn(), //
    },

    /// Other lifecycle methods in `#[godot_api] impl GodotVirtuals for MyClass`
    UserVirtuals {
        get_virtual_fn: unsafe extern "C" fn(
            _class_user_data: *mut std::ffi::c_void,
            p_name: *const std::os::raw::c_char,
        ) -> sys::GDNativeExtensionClassCallVirtual,
    },

    /// Custom `to_string` method
    ToString {
        to_string_fn: unsafe extern "C" fn(
            instance: sys::GDExtensionClassInstancePtr,
            out_string: sys::GDNativeStringPtr,
        ),
    },
}

pub fn register_class<T: UserMethodBinds + UserVirtuals + GodotMethods>() {
    let creation_info = sys::GDNativeExtensionClassCreationInfo {
        set_func: None,
        get_func: None,
        get_property_list_func: None,
        free_property_list_func: None,
        property_can_revert_func: None,
        property_get_revert_func: None,
        notification_func: None,
        to_string_func: if T::has_to_string() {
            Some({
                unsafe extern "C" fn to_string<T: GodotMethods>(
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
                to_string::<T>
            })
        } else {
            None
        },
        reference_func: Some({
            unsafe extern "C" fn reference<T: GodotClass>(
                instance: sys::GDExtensionClassInstancePtr,
            ) {
                let storage = as_storage::<T>(instance);
                storage.on_inc_ref();
            }
            reference::<T>
        }),
        unreference_func: Some({
            unsafe extern "C" fn unreference<T: GodotClass>(
                instance: sys::GDExtensionClassInstancePtr,
            ) {
                let storage = as_storage::<T>(instance);
                storage.on_dec_ref();
            }
            unreference::<T>
        }),
        create_instance_func: Some({
            unsafe extern "C" fn create<T: GodotClass>(
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
            create::<T>
        }),
        free_instance_func: Some({
            unsafe extern "C" fn free<T: GodotClass>(
                _class_user_data: *mut std::ffi::c_void,
                instance: sys::GDExtensionClassInstancePtr,
            ) {
                let storage = as_storage::<T>(instance);
                storage.mark_destroyed_by_godot();
                Box::from_raw(storage as *mut InstanceStorage<_>); // aka. drop
            }
            free::<T>
        }),
        get_virtual_func: Some({
            unsafe extern "C" fn get_virtual<T: UserVirtuals>(
                _class_user_data: *mut std::ffi::c_void,
                p_name: *const std::os::raw::c_char,
            ) -> sys::GDNativeExtensionClassCallVirtual {
                let name = std::ffi::CStr::from_ptr(p_name);
                T::virtual_call(name.to_str().expect("T::virtual_call"))
            }
            get_virtual::<T>
        }),
        get_rid_func: None,
        class_userdata: std::ptr::null_mut(), // will be passed to create fn, but global per class
    };

    let class_name = ClassName::new::<T>();
    let parent_class_name = ClassName::new::<T::Base>();

    unsafe {
        interface_fn!(classdb_register_extension_class)(
            sys::get_library(),
            class_name.c_str(),
            parent_class_name.c_str(),
            std::ptr::addr_of!(creation_info),
        );
    }

    T::register_methods();
}

/// Utility to convert `String` to C `const char*`.
/// Cannot be a function since the backing string must be retained.
pub(crate) struct ClassName {
    backing: String,
}

impl ClassName {
    pub fn new<T: GodotClass>() -> Self {
        Self {
            backing: format!("{}\0", T::class_name()),
        }
    }

    pub fn c_str(&self) -> *const std::os::raw::c_char {
        self.backing.as_ptr() as *const _
    }
}
