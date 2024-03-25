/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! Callbacks that are passed as function pointers to Godot upon class registration.
//!
//! Re-exported to `crate::private`.
#![allow(clippy::missing_safety_doc)]

use crate::builder::ClassBuilder;
use crate::builtin::{StringName, Variant};
use crate::obj::{cap, Base, GodotClass, UserClass};
use crate::storage::{as_storage, InstanceStorage, Storage, StorageRefCounted};
use godot_ffi as sys;
use std::any::Any;
use sys::interface_fn;

pub unsafe extern "C" fn create<T: cap::GodotDefault>(
    _class_userdata: *mut std::ffi::c_void,
) -> sys::GDExtensionObjectPtr {
    create_custom(T::__godot_user_init)
}

#[cfg(since_api = "4.2")]
pub unsafe extern "C" fn recreate<T: cap::GodotDefault>(
    _class_userdata: *mut std::ffi::c_void,
    object: sys::GDExtensionObjectPtr,
) -> sys::GDExtensionClassInstancePtr {
    create_rust_part_for_existing_godot_part(T::__godot_user_init, object)
}

pub(crate) fn create_custom<T, F>(make_user_instance: F) -> sys::GDExtensionObjectPtr
where
    T: GodotClass,
    F: FnOnce(Base<T::Base>) -> T,
{
    let base_class_name = T::Base::class_name();

    let base_ptr = unsafe { interface_fn!(classdb_construct_object)(base_class_name.string_sys()) };

    create_rust_part_for_existing_godot_part(make_user_instance, base_ptr);

    // std::mem::forget(base_class_name);
    base_ptr
}

// with GDExt, custom object consists from two parts: Godot object and Rust object, that are
// bound to each other. this method takes the first by pointer, creates the second with
// supplied state and binds them together. that's used for both brand-new objects creation and
// hot reload - during hot-reload, Rust objects are disposed and then created again with a
// updated code, so that's necessary to link them to Godot objects again.
fn create_rust_part_for_existing_godot_part<T, F>(
    make_user_instance: F,
    base_ptr: sys::GDExtensionObjectPtr,
) -> sys::GDExtensionClassInstancePtr
where
    T: GodotClass,
    F: FnOnce(Base<T::Base>) -> T,
{
    let class_name = T::class_name();

    //out!("create callback: {}", class_name.backing);

    let base = unsafe { Base::from_sys(base_ptr) };
    let user_instance = make_user_instance(unsafe { Base::from_base(&base) });

    let instance = InstanceStorage::<T>::construct(user_instance, base);
    let instance_ptr = instance.into_raw();
    let instance_ptr = instance_ptr as sys::GDExtensionClassInstancePtr;

    let binding_data_callbacks = crate::storage::nop_instance_callbacks();
    unsafe {
        interface_fn!(object_set_instance)(base_ptr, class_name.string_sys(), instance_ptr);
        interface_fn!(object_set_instance_binding)(
            base_ptr,
            sys::get_library() as *mut std::ffi::c_void,
            instance_ptr as *mut std::ffi::c_void,
            &binding_data_callbacks,
        );
    }

    // std::mem::forget(class_name);
    instance_ptr
}

pub unsafe extern "C" fn free<T: GodotClass>(
    _class_user_data: *mut std::ffi::c_void,
    instance: sys::GDExtensionClassInstancePtr,
) {
    {
        let storage = as_storage::<T>(instance);
        storage.mark_destroyed_by_godot();
    } // Ref no longer valid once next statement is executed.

    crate::storage::destroy_storage::<T>(instance);
}

pub unsafe extern "C" fn get_virtual<T: cap::ImplementsGodotVirtual>(
    _class_user_data: *mut std::ffi::c_void,
    name: sys::GDExtensionConstStringNamePtr,
) -> sys::GDExtensionClassCallVirtual {
    // This string is not ours, so we cannot call the destructor on it.
    let borrowed_string = StringName::borrow_string_sys(name);
    let method_name = borrowed_string.to_string();

    T::__virtual_call(method_name.as_str())
}

pub unsafe extern "C" fn default_get_virtual<T: UserClass>(
    _class_user_data: *mut std::ffi::c_void,
    name: sys::GDExtensionConstStringNamePtr,
) -> sys::GDExtensionClassCallVirtual {
    // This string is not ours, so we cannot call the destructor on it.
    let borrowed_string = StringName::borrow_string_sys(name);
    let method_name = borrowed_string.to_string();

    T::__default_virtual_call(method_name.as_str())
}

pub unsafe extern "C" fn to_string<T: cap::GodotToString>(
    instance: sys::GDExtensionClassInstancePtr,
    _is_valid: *mut sys::GDExtensionBool,
    out_string: sys::GDExtensionStringPtr,
) {
    // Note: to_string currently always succeeds, as it is only provided for classes that have a working implementation.
    // is_valid output parameter thus not needed.

    let storage = as_storage::<T>(instance);
    let instance = storage.get();
    let string = T::__godot_to_string(&*instance);

    // Transfer ownership to Godot
    string.move_into_string_ptr(out_string);
}

#[cfg(before_api = "4.2")]
pub unsafe extern "C" fn on_notification<T: cap::GodotNotification>(
    instance: sys::GDExtensionClassInstancePtr,
    what: i32,
) {
    let storage = as_storage::<T>(instance);
    let mut instance = storage.get_mut();

    T::__godot_notification(&mut *instance, what);
}

#[cfg(since_api = "4.2")]
pub unsafe extern "C" fn on_notification<T: cap::GodotNotification>(
    instance: sys::GDExtensionClassInstancePtr,
    what: i32,
    _reversed: sys::GDExtensionBool,
) {
    let storage = as_storage::<T>(instance);
    let mut instance = storage.get_mut();

    T::__godot_notification(&mut *instance, what);
}

pub unsafe extern "C" fn get_property<T: cap::GodotGet>(
    instance: sys::GDExtensionClassInstancePtr,
    name: sys::GDExtensionConstStringNamePtr,
    ret: sys::GDExtensionVariantPtr,
) -> sys::GDExtensionBool {
    let storage = as_storage::<T>(instance);
    let instance = storage.get();
    let property = StringName::new_from_string_sys(name);

    match T::__godot_get_property(&*instance, property) {
        Some(value) => {
            value.move_into_var_ptr(ret);
            true as sys::GDExtensionBool
        }
        None => false as sys::GDExtensionBool,
    }
}

pub unsafe extern "C" fn set_property<T: cap::GodotSet>(
    instance: sys::GDExtensionClassInstancePtr,
    name: sys::GDExtensionConstStringNamePtr,
    value: sys::GDExtensionConstVariantPtr,
) -> sys::GDExtensionBool {
    let storage = as_storage::<T>(instance);
    let mut instance = storage.get_mut();

    let property = StringName::new_from_string_sys(name);
    let value = Variant::new_from_var_sys(value);

    T::__godot_set_property(&mut *instance, property, value) as sys::GDExtensionBool
}

pub unsafe extern "C" fn reference<T: GodotClass>(instance: sys::GDExtensionClassInstancePtr) {
    let storage = as_storage::<T>(instance);
    storage.on_inc_ref();
}

pub unsafe extern "C" fn unreference<T: GodotClass>(instance: sys::GDExtensionClassInstancePtr) {
    let storage = as_storage::<T>(instance);
    storage.on_dec_ref();
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Safe, higher-level methods

/// Abstracts the `GodotDefault` away, for contexts where this trait bound is not statically available
pub fn erased_init<T: cap::GodotDefault>(base: Box<dyn Any>) -> Box<dyn Any> {
    let concrete = base
        .downcast::<Base<<T as GodotClass>::Base>>()
        .expect("erased_init: bad type erasure");
    let extracted: Base<_> = sys::unbox(concrete);

    let instance = T::__godot_user_init(extracted);
    Box::new(instance)
}

pub fn register_class_by_builder<T: cap::GodotRegisterClass>(_class_builder: &mut dyn Any) {
    // TODO use actual argument, once class builder carries state
    // let class_builder = class_builder
    //     .downcast_mut::<ClassBuilder<T>>()
    //     .expect("bad type erasure");

    let mut class_builder = ClassBuilder::new();
    T::__godot_register_class(&mut class_builder);
}

pub fn register_user_properties<T: cap::ImplementsGodotExports>(_class_builder: &mut dyn Any) {
    T::__register_exports();
}

pub fn register_user_methods_constants<T: cap::ImplementsGodotApi>(_class_builder: &mut dyn Any) {
    // let class_builder = class_builder
    //     .downcast_mut::<ClassBuilder<T>>()
    //     .expect("bad type erasure");

    //T::register_methods(class_builder);
    T::__register_methods();
    T::__register_constants();
}
