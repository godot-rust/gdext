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

use std::any::Any;

use godot_ffi as sys;
use sys::conv::u32_to_usize;
use sys::interface_fn;

use crate::builder::ClassBuilder;
use crate::builtin::{StringName, Variant};
use crate::classes::Object;
use crate::meta::PropertyInfo;
use crate::obj::{bounds, cap, AsDyn, Base, Bounds, Gd, GodotClass, Inherits, UserClass};
use crate::private::{handle_panic, PanicPayload};
use crate::registry::plugin::ErasedDynGd;
use crate::storage::{as_storage, InstanceStorage, Storage, StorageRefCounted};

/// Godot FFI default constructor.
///
/// If the `init()` constructor panics, null is returned.
///
/// Creation callback has `p_notify_postinitialize` parameter since 4.4: <https://github.com/godotengine/godot/pull/91018>.
#[cfg(since_api = "4.4")]
pub unsafe extern "C" fn create<T: cap::GodotDefault>(
    _class_userdata: *mut std::ffi::c_void,
    _notify_postinitialize: sys::GDExtensionBool,
) -> sys::GDExtensionObjectPtr {
    create_custom(T::__godot_user_init).unwrap_or(std::ptr::null_mut())
}

#[cfg(before_api = "4.4")]
pub unsafe extern "C" fn create<T: cap::GodotDefault>(
    _class_userdata: *mut std::ffi::c_void,
) -> sys::GDExtensionObjectPtr {
    create_custom(T::__godot_user_init).unwrap_or(std::ptr::null_mut())
}

/// Workaround for <https://github.com/godot-rust/gdext/issues/874> before Godot 4.5.
///
/// Godot expects a creator function, but doesn't require an actual object to be instantiated.
#[cfg(all(since_api = "4.4", before_api = "4.5"))]
pub unsafe extern "C" fn create_null<T>(
    _class_userdata: *mut std::ffi::c_void,
    _notify_postinitialize: sys::GDExtensionBool,
) -> sys::GDExtensionObjectPtr {
    std::ptr::null_mut()
}

#[cfg(before_api = "4.4")]
pub unsafe extern "C" fn create_null<T>(
    _class_userdata: *mut std::ffi::c_void,
) -> sys::GDExtensionObjectPtr {
    std::ptr::null_mut()
}

/// Godot FFI function for recreating a GDExtension instance, e.g. after a hot reload.
///
/// If the `init()` constructor panics, null is returned.
#[cfg(since_api = "4.2")]
pub unsafe extern "C" fn recreate<T: cap::GodotDefault>(
    _class_userdata: *mut std::ffi::c_void,
    object: sys::GDExtensionObjectPtr,
) -> sys::GDExtensionClassInstancePtr {
    create_rust_part_for_existing_godot_part(T::__godot_user_init, object)
        .unwrap_or(std::ptr::null_mut())
}

/// Workaround for <https://github.com/godot-rust/gdext/issues/874> before Godot 4.5.
///
/// Godot expects a creator function, but doesn't require an actual object to be instantiated.
#[cfg(all(since_api = "4.2", before_api = "4.5"))]
pub unsafe extern "C" fn recreate_null<T>(
    _class_userdata: *mut std::ffi::c_void,
    _object: sys::GDExtensionObjectPtr,
) -> sys::GDExtensionClassInstancePtr {
    std::ptr::null_mut()
}

pub(crate) fn create_custom<T, F>(
    make_user_instance: F,
) -> Result<sys::GDExtensionObjectPtr, PanicPayload>
where
    T: GodotClass,
    F: FnOnce(Base<T::Base>) -> T,
{
    let base_class_name = T::Base::class_name();
    let base_ptr = unsafe { interface_fn!(classdb_construct_object)(base_class_name.string_sys()) };

    match create_rust_part_for_existing_godot_part(make_user_instance, base_ptr) {
        Ok(_extension_ptr) => Ok(base_ptr),
        Err(payload) => {
            // Creation of extension object failed; we must now also destroy the base object to avoid leak.
            // SAFETY: `base_ptr` was just created above.
            unsafe { interface_fn!(object_destroy)(base_ptr) };

            Err(payload)
        }
    }

    // std::mem::forget(base_class_name);
}

/// Add Rust-side state for a GDExtension base object.
///
/// With godot-rust, custom objects consist of two parts: the Godot object and the Rust object. This method takes the Godot part by pointer,
/// creates the Rust part with the supplied state, and links them together. This is used for both brand-new object creation and hot reload.
/// During hot reload, Rust objects are disposed of and then created again with updated code, so it's necessary to re-link them to Godot objects.
fn create_rust_part_for_existing_godot_part<T, F>(
    make_user_instance: F,
    base_ptr: sys::GDExtensionObjectPtr,
) -> Result<sys::GDExtensionClassInstancePtr, PanicPayload>
where
    T: GodotClass,
    F: FnOnce(Base<T::Base>) -> T,
{
    let class_name = T::class_name();
    //out!("create callback: {}", class_name.backing);

    let base = unsafe { Base::from_sys(base_ptr) };

    // User constructor init() can panic, which crashes the engine if unhandled.
    let context = || format!("panic during {class_name}::init() constructor");
    let code = || make_user_instance(unsafe { Base::from_base(&base) });
    let user_instance = handle_panic(context, std::panic::AssertUnwindSafe(code))?;

    // Print shouldn't be necessary as panic itself is printed. If this changes, re-enable in error case:
    // godot_error!("failed to create instance of {class_name}; Rust init() panicked");

    let mut base_copy = unsafe { Base::from_base(&base) };

    let instance = InstanceStorage::<T>::construct(user_instance, base);
    let instance_rust_ptr = instance.into_raw();
    let instance_ptr = instance_rust_ptr as sys::GDExtensionClassInstancePtr;

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

    // Mark initialization as complete, now that user constructor has finished.
    if base_copy.mark_initialized() {
        // If an extra RefCounted reference was handed out, notify storage about it, so it can unreference later.
        let instance_ref = unsafe { &*instance_rust_ptr };
        instance_ref.mark_surplus_ref();
    }

    // No std::mem::forget(base_copy) here, since Base may stores other fields that need deallocation.
    Ok(instance_ptr)
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

#[cfg(since_api = "4.4")]
pub unsafe extern "C" fn get_virtual<T: cap::ImplementsGodotVirtual>(
    _class_user_data: *mut std::ffi::c_void,
    name: sys::GDExtensionConstStringNamePtr,
    hash: u32,
) -> sys::GDExtensionClassCallVirtual {
    // This string is not ours, so we cannot call the destructor on it.
    let borrowed_string = StringName::borrow_string_sys(name);
    let method_name = borrowed_string.to_string();

    T::__virtual_call(method_name.as_str(), hash)
}

#[cfg(before_api = "4.4")]
pub unsafe extern "C" fn get_virtual<T: cap::ImplementsGodotVirtual>(
    _class_user_data: *mut std::ffi::c_void,
    name: sys::GDExtensionConstStringNamePtr,
) -> sys::GDExtensionClassCallVirtual {
    // This string is not ours, so we cannot call the destructor on it.
    let borrowed_string = StringName::borrow_string_sys(name);
    let method_name = borrowed_string.to_string();

    T::__virtual_call(method_name.as_str())
}

#[cfg(since_api = "4.4")]
pub unsafe extern "C" fn default_get_virtual<T: UserClass>(
    _class_user_data: *mut std::ffi::c_void,
    name: sys::GDExtensionConstStringNamePtr,
    hash: u32,
) -> sys::GDExtensionClassCallVirtual {
    // This string is not ours, so we cannot call the destructor on it.
    let borrowed_string = StringName::borrow_string_sys(name);
    let method_name = borrowed_string.to_string();

    T::__default_virtual_call(method_name.as_str(), hash)
}

#[cfg(before_api = "4.4")]
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
    is_valid: *mut sys::GDExtensionBool,
    out_string: sys::GDExtensionStringPtr,
) {
    // Note: to_string currently always succeeds, as it is only provided for classes that have a working implementation.

    let storage = as_storage::<T>(instance);
    let instance = storage.get();
    let string = T::__godot_to_string(&*instance);

    // Transfer ownership to Godot
    string.move_into_string_ptr(out_string);

    // Note: is_valid comes uninitialized and must be set.
    *is_valid = sys::conv::SYS_TRUE;
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
            sys::conv::SYS_TRUE
        }
        None => sys::conv::SYS_FALSE,
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

    sys::conv::bool_to_sys(T::__godot_set_property(&mut *instance, property, value))
}

pub unsafe extern "C" fn reference<T: GodotClass>(instance: sys::GDExtensionClassInstancePtr) {
    let storage = as_storage::<T>(instance);
    storage.on_inc_ref();
}

pub unsafe extern "C" fn unreference<T: GodotClass>(instance: sys::GDExtensionClassInstancePtr) {
    let storage = as_storage::<T>(instance);
    storage.on_dec_ref();
}

/// # Safety
///
/// Must only be called by Godot as a callback for `get_property_list` for a rust-defined class of type `T`.
#[deny(unsafe_op_in_unsafe_fn)]
pub unsafe extern "C" fn get_property_list<T: cap::GodotGetPropertyList>(
    instance: sys::GDExtensionClassInstancePtr,
    count: *mut u32,
) -> *const sys::GDExtensionPropertyInfo {
    // SAFETY: Godot provides us with a valid instance pointer to a `T`. And it will live until the end of this function.
    let storage = unsafe { as_storage::<T>(instance) };
    let mut instance = storage.get_mut();

    let property_list = T::__godot_get_property_list(&mut *instance);
    let property_list_sys: Box<[sys::GDExtensionPropertyInfo]> = property_list
        .into_iter()
        .map(|prop| prop.into_owned_property_sys())
        .collect();

    // SAFETY: Godot ensures that `count` is initialized and valid to write into.
    unsafe {
        *count = property_list_sys
            .len()
            .try_into()
            .expect("property list cannot be longer than `u32::MAX`");
    }

    Box::leak(property_list_sys).as_mut_ptr()
}

/// # Safety
///
/// - Must only be called by Godot as a callback for `free_property_list` for a rust-defined class of type `T`.
/// - Must only be passed to Godot as a callback when [`get_property_list`] is the corresponding `get_property_list` callback.
#[deny(unsafe_op_in_unsafe_fn)]
pub unsafe extern "C" fn free_property_list<T: cap::GodotGetPropertyList>(
    _instance: sys::GDExtensionClassInstancePtr,
    list: *const sys::GDExtensionPropertyInfo,
    count: u32,
) {
    let list = list as *mut sys::GDExtensionPropertyInfo;

    // SAFETY: `list` comes from `get_property_list` above, and `count` also comes from the same function.
    // This means that `list` is a pointer to a `&[sys::GDExtensionPropertyInfo]` slice of length `count`.
    // This means all the preconditions of this function are satisfied except uniqueness of this point.
    // Uniqueness is guaranteed as Godot called this function at a point where the list is no longer accessed
    // through any other pointer, and we don't access the slice through any other pointer after this call either.
    let property_list_slice = unsafe { std::slice::from_raw_parts_mut(list, u32_to_usize(count)) };

    // SAFETY: This slice was created by calling `Box::leak` on a `Box<[sys::GDExtensionPropertyInfo]>`, we can thus
    // call `Box::from_raw` on this slice to get back the original boxed slice.
    // Note that this relies on coercion of `&mut` -> `*mut`.
    let property_list_sys = unsafe { Box::from_raw(property_list_slice) };

    for property_info in property_list_sys.iter() {
        // SAFETY: The structs contained in this list were all returned from `into_owned_property_sys`.
        // We only call this method once for each struct and for each list.
        unsafe {
            crate::meta::PropertyInfo::free_owned_property_sys(*property_info);
        }
    }
}

/// # Safety
///
/// * `instance` must be a valid `T` instance pointer for the duration of this function call.
/// * `property_name` must be a valid `StringName` pointer for the duration of this function call.
#[deny(unsafe_op_in_unsafe_fn)]
unsafe fn raw_property_get_revert<T: cap::GodotPropertyGetRevert>(
    instance: sys::GDExtensionClassInstancePtr,
    property_name: sys::GDExtensionConstStringNamePtr,
) -> Option<Variant> {
    // SAFETY: `instance` is a valid `T` instance pointer for the duration of this function call.
    let storage = unsafe { as_storage::<T>(instance) };
    let instance = storage.get();

    // SAFETY: `property_name` is a valid `StringName` pointer for the duration of this function call.
    let property = unsafe { StringName::borrow_string_sys(property_name) };
    T::__godot_property_get_revert(&*instance, property.clone())
}

/// # Safety
///
/// - Must only be called by Godot as a callback for `property_can_revert` for a rust-defined class of type `T`.
#[deny(unsafe_op_in_unsafe_fn)]
pub unsafe extern "C" fn property_can_revert<T: cap::GodotPropertyGetRevert>(
    instance: sys::GDExtensionClassInstancePtr,
    property_name: sys::GDExtensionConstStringNamePtr,
) -> sys::GDExtensionBool {
    // SAFETY: Godot provides us with a valid `T` instance pointer and `StringName` pointer for the duration of this call.
    let revert = unsafe { raw_property_get_revert::<T>(instance, property_name) };

    sys::conv::bool_to_sys(revert.is_some())
}

/// # Safety
///
/// - Must only be called by Godot as a callback for `property_get_revert` for a rust-defined class of type `T`.
#[deny(unsafe_op_in_unsafe_fn)]
pub unsafe extern "C" fn property_get_revert<T: cap::GodotPropertyGetRevert>(
    instance: sys::GDExtensionClassInstancePtr,
    property_name: sys::GDExtensionConstStringNamePtr,
    ret: sys::GDExtensionVariantPtr,
) -> sys::GDExtensionBool {
    // SAFETY: Godot provides us with a valid `T` instance pointer and `StringName` pointer for the duration of this call.
    let Some(revert) = (unsafe { raw_property_get_revert::<T>(instance, property_name) }) else {
        return sys::conv::SYS_FALSE;
    };

    // SAFETY: Godot provides us with a valid `Variant` pointer.
    unsafe {
        revert.move_into_var_ptr(ret);
    }

    sys::conv::SYS_TRUE
}

/// Callback for `validate_property`.
///
/// Exposes `PropertyInfo` created out of `*mut GDExtensionPropertyInfo` ptr to user and moves edited values back to the pointer.
///
/// # Safety
///
/// - Must only be called by Godot as a callback for `validate_property` for a rust-defined class of type `T`.
/// - `property_info_ptr` must be valid for the whole duration of this function call (i.e. - can't be freed nor consumed).
///
#[deny(unsafe_op_in_unsafe_fn)]
#[cfg(since_api = "4.2")]
pub unsafe extern "C" fn validate_property<T: cap::GodotValidateProperty>(
    instance: sys::GDExtensionClassInstancePtr,
    property_info_ptr: *mut sys::GDExtensionPropertyInfo,
) -> sys::GDExtensionBool {
    // SAFETY: `instance` is a valid `T` instance pointer for the duration of this function call.
    let storage = unsafe { as_storage::<T>(instance) };
    let instance = storage.get();

    // SAFETY: property_info_ptr must be valid.
    let mut property_info = unsafe { PropertyInfo::new_from_sys(property_info_ptr) };
    T::__godot_validate_property(&*instance, &mut property_info);

    // SAFETY: property_info_ptr remains valid & unchanged.
    unsafe { property_info.move_into_property_info_ptr(property_info_ptr) };

    sys::conv::SYS_TRUE
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Safe, higher-level methods

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

pub fn register_user_rpcs<T: cap::ImplementsGodotApi>(object: &mut dyn Any) {
    T::__register_rpcs(object);
}

/// # Safety
///
/// `obj` must be castable to `T`.
#[deny(unsafe_op_in_unsafe_fn)]
pub unsafe fn dynify_fn<T, D>(obj: Gd<Object>) -> ErasedDynGd
where
    T: GodotClass + Inherits<Object> + AsDyn<D> + Bounds<Declarer = bounds::DeclUser>,
    D: ?Sized + 'static,
{
    // SAFETY: `obj` is castable to `T`.
    let obj = unsafe { obj.try_cast::<T>().unwrap_unchecked() };
    let obj = obj.into_dyn::<D>();
    let obj = obj.upcast::<Object>();

    ErasedDynGd {
        boxed: Box::new(obj),
    }
}
