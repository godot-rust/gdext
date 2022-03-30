use crate::private::as_storage;
use crate::storage::InstanceStorage;
use crate::sys;
use crate::traits::*;
use gdext_builtin::godot_ffi::GodotFfi as _;
use gdext_sys::interface_fn;

pub fn register_class<T: GodotExtensionClass + GodotExtensionClassMethods + Default>() {
    let creation_info = sys::GDNativeExtensionClassCreationInfo {
        set_func: None,
        get_func: None,
        get_property_list_func: None,
        free_property_list_func: None,
        notification_func: None,
        to_string_func: if T::has_to_string() {
            Some({
                unsafe extern "C" fn to_string<T: GodotExtensionClassMethods>(
                    instance: *mut std::ffi::c_void,
                    out_string: *mut std::ffi::c_void,
                ) {
                    let storage = as_storage::<T>(instance);
                    let instance = storage.get();
                    let string = instance.to_string();

                    // Transfer ownership to Godot, disable destructor
                    string.write_sys(out_string);
                    std::mem::forget(string);
                }
                to_string::<T>
            })
        } else {
            None
        },
        reference_func: Some({
            unsafe extern "C" fn reference<T: GodotExtensionClass>(
                instance: *mut std::ffi::c_void,
            ) {
                let storage = as_storage::<T>(instance);
                let instance = storage.get_mut();
                instance.reference();
            }
            reference::<T>
        }),
        unreference_func: Some({
            unsafe extern "C" fn unreference<T: GodotExtensionClass>(
                instance: *mut std::ffi::c_void,
            ) {
                let storage = as_storage::<T>(instance);
                let instance = storage.get_mut();
                instance.unreference();
            }
            unreference::<T>
        }),
        create_instance_func: Some({
            unsafe extern "C" fn instance<T: GodotClass + Default>(
                _class_userdata: *mut std::ffi::c_void,
            ) -> *mut std::ffi::c_void {
                let class_name = ClassName::new::<T>();
                let base_class_name = ClassName::new::<T::Base>();

                let base = interface_fn!(classdb_construct_object)(base_class_name.c_str());
                let instance = InstanceStorage::<T>::construct_default(base);
                let instance_ptr = instance.into_raw();

                interface_fn!(object_set_instance)(
                    base,
                    class_name.c_str(),
                    instance_ptr as *mut std::ffi::c_void,
                );

                let binding_data_callbacks = sys::GDNativeInstanceBindingCallbacks {
                    create_callback: None,
                    free_callback: None,
                    reference_callback: None,
                };

                interface_fn!(object_set_instance_binding)(
                    base,
                    sys::get_library() as *mut _,
                    instance_ptr as *mut std::ffi::c_void,
                    &binding_data_callbacks,
                );

                base
            }
            instance::<T>
        }),
        free_instance_func: Some({
            unsafe extern "C" fn free<T: GodotExtensionClass>(
                _class_user_data: *mut std::ffi::c_void,
                instance: *mut std::ffi::c_void,
            ) {
                Box::from_raw(as_storage::<T>(instance));
            }
            free::<T>
        }),
        get_virtual_func: Some({
            unsafe extern "C" fn get_virtual<T: GodotExtensionClassMethods>(
                _class_user_data: *mut std::ffi::c_void,
                p_name: *const std::os::raw::c_char,
            ) -> sys::GDNativeExtensionClassCallVirtual {
                let name = std::ffi::CStr::from_ptr(p_name);
                T::virtual_call(name.to_str().unwrap())
            }
            get_virtual::<T>
        }),
        get_rid_func: None,
        class_userdata: std::ptr::null_mut(),
    };

    let class_name = ClassName::new::<T>();
    let parent_class_name = ClassName::new::<T::Base>();

    unsafe {
        interface_fn!(classdb_register_extension_class)(
            sys::get_library(),
            class_name.c_str(),
            parent_class_name.c_str(),
            &creation_info as *const _,
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
