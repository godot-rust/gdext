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
use crate::builtin::meta::PropertyInfo;
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
    let borrowed_string = StringName::from_string_sys(sys::force_mut_ptr(name));
    let method_name = borrowed_string.to_string();
    std::mem::forget(borrowed_string);

    T::__virtual_call(method_name.as_str())
}

pub unsafe extern "C" fn default_get_virtual<T: UserClass>(
    _class_user_data: *mut std::ffi::c_void,
    name: sys::GDExtensionConstStringNamePtr,
) -> sys::GDExtensionClassCallVirtual {
    // This string is not ours, so we cannot call the destructor on it.
    let borrowed_string = StringName::from_string_sys(sys::force_mut_ptr(name));
    let method_name = borrowed_string.to_string();
    std::mem::forget(borrowed_string);

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
    string.move_string_ptr(out_string);
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
    let property = StringName::from_string_sys(sys::force_mut_ptr(name));

    std::mem::forget(property.clone());

    match T::__godot_get_property(&*instance, property) {
        Some(value) => {
            value.move_var_ptr(ret);
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

    let property = StringName::from_string_sys(sys::force_mut_ptr(name));
    let value = Variant::from_var_sys(sys::force_mut_ptr(value));

    std::mem::forget(property.clone());
    std::mem::forget(value.clone());

    T::__godot_set_property(&mut *instance, property, value) as sys::GDExtensionBool
}

pub unsafe extern "C" fn get_property_list<T: cap::GodotGetPropertyList>(
    instance: sys::GDExtensionClassInstancePtr,
    count: *mut u32,
) -> *const sys::GDExtensionPropertyInfo {
    let storage = as_storage::<T>(instance);
    let instance = storage.get();

    let property_infos = T::__godot_get_property_list(&*instance);
    let property_count: u32 = property_infos
        .len()
        .try_into()
        .expect("cannot pass more properties than `u32::MAX`");
    let vec_length: usize = property_count
        .try_into()
        .expect("gdext does not support targets with `u32` bigger than `usize`");
    // This can only fail if vec_length = u32::MAX and usize is the same size as u32.
    let vec_length = vec_length
        .checked_add(1)
        .expect("cannot pass more properties than `usize::MAX - 1`");

    // Use `ManuallyDrop` here so we intentionally leak this vec, as we want to pass ownership of this array to Godot until ownership is
    // returned to us in `free_property_list`.
    let mut list = Vec::with_capacity(vec_length);
    list.extend(
        property_infos
            .into_iter()
            .map(PropertyInfo::into_property_sys),
    );

    // Use as null-terminator. `PropertyInfo::into_property_sys` will never create a `sys::GDExtensionPropertyInfo` with values like this.
    // This is *important*. If we do have a value before the final one that is equal to `empty_sys()` then we would later call
    // `Vec::from_raw_parts` with an incorrect capacity and trigger UB.
    list.push(PropertyInfo::empty_sys());

    // So at least in debug mode, let's check that our assumptions about `list` hold true.
    if cfg!(debug_assertions) {
        for prop in list.iter().take(vec_length - 1) {
            assert!(
                !prop.name.is_null(),
                "Invalid property info found: {:?}",
                prop
            );
        }
        assert_eq!(list.len(), vec_length);
        assert_eq!(list.capacity(), vec_length);
        assert!((property_count as usize) < vec_length)
    }

    // SAFETY: Godot gives us exclusive ownership over `count` for the purposes of returning the length of the property list, so we can safely
    // write a value of type `u32` to `count`.
    unsafe {
        count.write(property_count);
    }

    let slice = Box::into_raw(list.into_boxed_slice());

    // Since `list` is in a `ManuallyDrop`, this leaks the `list` and thus passes ownership of the vec to the caller (which is gonna be Godot).
    (*slice).as_mut_ptr() as *const _
}

/// Get the length of a "null"-terminated array.
///
/// Where "null" here is defined by `terminator_fn` returning true.
///
/// # Panics
///
/// The given array has more than `isize::MAX` elements.
///
/// # Safety
///
/// `arr` must be dereferencable to `&T`.
///
/// Whenever `terminator_fn(&*arr.offset(i))` is false, for every i in `0..n`, then:
/// - `arr.offset(n)` must be safe, see safety docs for `offset`.
/// - `arr.offset(n)` must be dereferencable to `&T`.
unsafe fn arr_length<T>(arr: *const T, terminator_fn: impl Fn(&T) -> bool) -> usize {
    let mut list_index = 0;
    loop {
        // SAFETY: `terminator_fn` has not returned `true` yet, therefore we are allowed to do `arr.offset(list_index)`.
        let arr_offset = unsafe { arr.offset(list_index) };

        // SAFETY: `terminator_fn` has not returned `true` yet, therefore we can dereference `arr_offset` to `&T`.
        let elem = unsafe { &*arr_offset };

        if terminator_fn(elem) {
            break;
        }

        list_index = list_index
            .checked_add(1)
            .expect("there should not be more than `isize::MAX` elements in the array");
    }

    usize::try_from(list_index).expect("the size of the array should never be negative") + 1
}

pub unsafe extern "C" fn free_property_list<T: cap::GodotGetPropertyList>(
    _instance: sys::GDExtensionClassInstancePtr,
    list: *const sys::GDExtensionPropertyInfo,
) {
    // SAFETY: `list` was created in `get_property_list` from a `Vec` with some fixed length, where the final element is a
    // `sys::GDExtensionPropertyInfo` with a null `name` field. So all the given safety conditions hold.
    let list_length = unsafe { arr_length(list, |prop| prop.name.is_null()) };

    // SAFETY: `list` was created in `get_property_list` from a `Vec` with length `list_length`. The length and capacity of this list
    // are the same, as the vec was made using `with_capacity` to have exactly the same capacity as the amount of elements we put in the vec.
    let v = unsafe { Vec::from_raw_parts(sys::force_mut_ptr(list), list_length, list_length) };

    for prop in v.into_iter().take(list_length - 1) {
        // SAFETY: All elements of `v` were created using `into_property_sys`, except for the last one. We are iterating over all elements
        // except the last one, so all `prop` values were created with `into_property_sys`.
        unsafe { crate::builtin::meta::PropertyInfo::drop_property_sys(prop) }
    }
}

pub unsafe extern "C" fn property_can_revert<T: cap::GodotPropertyCanRevert>(
    instance: sys::GDExtensionClassInstancePtr,
    name: sys::GDExtensionConstStringNamePtr,
) -> sys::GDExtensionBool {
    let storage = as_storage::<T>(instance);
    let instance = storage.get();

    let property = StringName::from_string_sys(sys::force_mut_ptr(name));

    std::mem::forget(property.clone());

    T::__godot_property_can_revert(&*instance, property) as sys::GDExtensionBool
}

pub unsafe extern "C" fn user_property_get_revert_fn<T: cap::GodotPropertyGetRevert>(
    instance: sys::GDExtensionClassInstancePtr,
    name: sys::GDExtensionConstStringNamePtr,
    ret: sys::GDExtensionVariantPtr,
) -> sys::GDExtensionBool {
    let storage = as_storage::<T>(instance);
    let instance = storage.get();

    let property = StringName::from_string_sys(sys::force_mut_ptr(name));

    std::mem::forget(property.clone());

    let value = T::__godot_property_get_revert(&*instance, property);

    match value {
        Some(value) => {
            // SAFETY: `ret` is a pointer Godot has given us exclusive ownership over for the purpose of writing a `Variant` to when we return
            // `true` from this function. So this write is safe.
            unsafe {
                value.move_var_ptr(ret);
            }
            true as sys::GDExtensionBool
        }
        None => false as sys::GDExtensionBool,
    }
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
