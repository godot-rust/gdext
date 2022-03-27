use gdext_builtin::string::GodotString;
use std::ffi::CStr;

use gdext_sys::{self as sys, interface_fn};

pub mod macros;
mod obj;
pub mod property_info;

pub use obj::*;

pub trait GodotClass {
    type Base: GodotClass;

    fn class_name() -> String;

    fn native_object_ptr(&self) -> sys::GDNativeObjectPtr {
        self.upcast().native_object_ptr()
    }
    fn upcast(&self) -> &Self::Base;
    fn upcast_mut(&mut self) -> &mut Self::Base;
}

pub trait GodotExtensionClass: GodotClass {
    fn construct(base: sys::GDNativeObjectPtr) -> Self;

    fn reference(&mut self) {}
    fn unreference(&mut self) {}
    fn has_to_string() -> bool {
        false
    }
}

pub trait GodotExtensionClassMethods {
    fn virtual_call(name: &str) -> sys::GDNativeExtensionClassCallVirtual;
    fn register_methods();
    fn to_string(&self) -> GodotString {
        GodotString::new()
    }
}

pub fn register_class<T: GodotExtensionClass + GodotExtensionClassMethods>() {
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
                ) -> *const std::os::raw::c_char {
                    let instance = &mut *(instance as *mut T);
                    let string = instance.to_string();

                    string.leak_c_string()
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
                let instance = &mut *(instance as *mut T);
                instance.reference();
            }
            reference::<T>
        }),
        unreference_func: Some({
            unsafe extern "C" fn unreference<T: GodotExtensionClass>(
                instance: *mut std::ffi::c_void,
            ) {
                let instance = &mut *(instance as *mut T);
                instance.unreference();
            }
            unreference::<T>
        }),
        create_instance_func: Some({
            unsafe extern "C" fn instance<T: GodotExtensionClass>(
                _class_userdata: *mut std::ffi::c_void,
            ) -> *mut std::ffi::c_void {
                let class_name = format!("{}\0", T::class_name());
                let parent_class_name = format!("{}\0", T::Base::class_name());

                let obj =
                    interface_fn!(classdb_construct_object)(parent_class_name.as_ptr() as *const _);
                let instance = Box::new(T::construct(obj));
                let instance_ptr = Box::into_raw(instance);

                interface_fn!(object_set_instance)(
                    obj,
                    class_name.as_ptr() as *const _,
                    instance_ptr as *mut _,
                );

                let binding_data_callbacks = sys::GDNativeInstanceBindingCallbacks {
                    create_callback: None,
                    free_callback: None,
                    reference_callback: None,
                };

                interface_fn!(object_set_instance_binding)(
                    obj,
                    sys::get_library() as *mut _,
                    instance_ptr as *mut _,
                    &binding_data_callbacks,
                );

                obj
            }
            instance::<T>
        }),
        free_instance_func: Some({
            unsafe extern "C" fn free<T: GodotExtensionClass>(
                _class_user_data: *mut std::ffi::c_void,
                instance: *mut std::ffi::c_void,
            ) {
                Box::from_raw(instance as *mut T);
            }
            free::<T>
        }),
        get_virtual_func: Some({
            unsafe extern "C" fn get_virtual<T: GodotExtensionClassMethods>(
                _class_user_data: *mut std::ffi::c_void,
                p_name: *const std::os::raw::c_char,
            ) -> sys::GDNativeExtensionClassCallVirtual {
                let name = CStr::from_ptr(p_name);
                T::virtual_call(name.to_str().unwrap())
            }
            get_virtual::<T>
        }),
        class_userdata: std::ptr::null_mut(),
    };

    let class_name = format!("{}\0", T::class_name());
    let parent_class_name = format!("{}\0", T::Base::class_name());

    unsafe {
        interface_fn!(classdb_register_extension_class)(
            sys::get_library(),
            class_name.as_ptr() as *const _,
            parent_class_name.as_ptr() as *const _,
            &creation_info as *const _,
        );
    }

    T::register_methods();
}

pub unsafe extern "C" fn do_instance<T: GodotExtensionClass>(
    _class_userdata: *mut std::ffi::c_void,
) -> *mut std::ffi::c_void {
    let class_name = format!("{}\0", T::class_name());
    let parent_class_name = format!("{}\0", T::Base::class_name());

    let obj = interface_fn!(classdb_construct_object)(parent_class_name.as_ptr() as *const _);
    let instance = Box::new(T::construct(obj));
    let instance_ptr = Box::into_raw(instance);

    interface_fn!(object_set_instance)(
        obj,
        class_name.as_ptr() as *const _,
        instance_ptr as *mut _,
    );

    let binding_data_callbacks = sys::GDNativeInstanceBindingCallbacks {
        create_callback: None,
        free_callback: None,
        reference_callback: None,
    };

    interface_fn!(object_set_instance_binding)(
        obj,
        sys::get_library() as *mut _,
        instance_ptr as *mut _,
        &binding_data_callbacks,
    );

    obj
}
