/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::cell::RefCell;
use std::ffi::c_void;

use crate::builtin::meta::{MethodInfo, PropertyInfo};
use crate::builtin::{GString, StringName, Variant, VariantType};
use crate::obj::Gd;
use crate::sys;

use super::{Script, ScriptLanguage};

/// Interface for Godot's `GDExtensionScriptInstance`, which cannot be directly constructed by an extension. Instead a type that implements this
/// trait has to be passed to the [`create_script_instance`] function, which returns a [`sys::GDExtensionScriptInstancePtr`] pointer. This pointer
/// can then be returned from [`IScriptExtension::instance_create`](crate::engine::IScriptExtension::instance_create).
pub trait ScriptInstance {
    /// Name of the new class the script implements.
    fn class_name(&self) -> GString;

    /// Property setter for Godots virtual dispatch system. The engine will call this function when it wants to change a property on the script.
    fn set(&mut self, name: StringName, value: &Variant) -> bool;

    /// Property getter for Godots virtual dispatch system. The engine will call this function when it wants to read a property on the script.
    fn get(&self, name: StringName) -> Option<Variant>;

    /// A list of all the properties a script exposes to the engine.
    fn get_property_list(&self) -> &[PropertyInfo];

    /// A list of all the methods a script exposes to the engine.
    fn get_method_list(&self) -> &[MethodInfo];

    /// Method invoker for Godots virtual dispatch system. The engine will call this function when it wants to call a method on the script.
    ///
    /// All method calls are taking a mutable reference of the script instance, as the engine does not differentiate between immutable and
    /// mutable method calls like rust.
    ///
    /// It's important that the script does not cause a second call to this function while executing a method call. This would result in a panic.
    // TODO: map the sys::GDExtensionCallErrorType to some public API type.
    fn call(
        &mut self,
        method: StringName,
        args: &[&Variant],
    ) -> Result<Variant, sys::GDExtensionCallErrorType>;

    /// Identifies the script instance as a placeholder. If this function and
    /// [IScriptExtension::is_placeholder_fallback_enabled](crate::engine::IScriptExtension::is_placeholder_fallback_enabled) return true,
    /// Godot will call [`Self::property_set_fallback`] instead of [`Self::set`].
    fn is_placeholder(&self) -> bool;

    /// Validation function for the engine to verify if the script exposes a certain method.
    fn has_method(&self, method: StringName) -> bool;

    /// Lets the engine get a reference to the script this instance was created for.
    ///
    /// This function has to return a reference, because Scripts are reference counted in Godot and it has to be guaranteed that the object is
    /// not freed before the engine increased the reference count. (every time a `Gd<T>` which contains a reference counted object is dropped the
    /// reference count is decremented.)
    fn get_script(&self) -> &Gd<Script>;

    /// Lets the engine fetch the type of a particular property.
    fn get_property_type(&self, name: StringName) -> VariantType;

    /// String representation of the script instance.
    fn to_string(&self) -> GString;

    /// A dump of all property names and values that are exposed to the engine.
    fn get_property_state(&self) -> Vec<(StringName, Variant)>;

    /// Lets the engine get a reference to the [`ScriptLanguage`] this instance belongs to.
    fn get_language(&self) -> Gd<ScriptLanguage>;

    /// Callback from the engine when the reference count of the base object has been decreased. When this method returns `true` the engine will
    /// not free the object the script is attached to.
    fn on_refcount_decremented(&self) -> bool;

    /// Callback from the engine when the reference count of the base object has been increased.
    fn on_refcount_incremented(&self);

    /// The engine may call this function if it failed to get a property value via [ScriptInstance::get] or the native types getter.
    fn property_get_fallback(&self, name: StringName) -> Option<Variant>;

    /// The engine may call this function if ScriptLanguage::is_placeholder_fallback_enabled is enabled.
    fn property_set_fallback(&mut self, name: StringName, value: &Variant) -> bool;
}

impl<T: ScriptInstance + ?Sized> ScriptInstance for Box<T> {
    fn class_name(&self) -> GString {
        self.as_ref().class_name()
    }

    fn set(&mut self, name: StringName, value: &Variant) -> bool {
        self.as_mut().set(name, value)
    }

    fn get(&self, name: StringName) -> Option<Variant> {
        self.as_ref().get(name)
    }

    fn get_property_list(&self) -> &[PropertyInfo] {
        self.as_ref().get_property_list()
    }

    fn get_method_list(&self) -> &[MethodInfo] {
        self.as_ref().get_method_list()
    }

    fn call(
        &mut self,
        method: StringName,
        args: &[&Variant],
    ) -> Result<Variant, sys::GDExtensionCallErrorType> {
        self.as_mut().call(method, args)
    }

    fn get_script(&self) -> &Gd<Script> {
        self.as_ref().get_script()
    }

    fn is_placeholder(&self) -> bool {
        self.as_ref().is_placeholder()
    }

    fn has_method(&self, method: StringName) -> bool {
        self.as_ref().has_method(method)
    }

    fn get_property_type(&self, name: StringName) -> VariantType {
        self.as_ref().get_property_type(name)
    }

    fn to_string(&self) -> GString {
        self.as_ref().to_string()
    }

    fn get_property_state(&self) -> Vec<(StringName, Variant)> {
        self.as_ref().get_property_state()
    }

    fn get_language(&self) -> Gd<ScriptLanguage> {
        self.as_ref().get_language()
    }

    fn on_refcount_incremented(&self) {
        self.as_ref().on_refcount_incremented();
    }

    fn on_refcount_decremented(&self) -> bool {
        self.as_ref().on_refcount_decremented()
    }

    fn property_get_fallback(&self, name: StringName) -> Option<Variant> {
        self.as_ref().property_get_fallback(name)
    }

    fn property_set_fallback(&mut self, name: StringName, value: &Variant) -> bool {
        self.as_mut().property_set_fallback(name, value)
    }
}

struct ScriptInstanceData<T: ScriptInstance> {
    inner: RefCell<T>,
    gd_instance_ptr: *mut sys::GDExtensionScriptInstanceInfo,
}

impl<T: ScriptInstance> Drop for ScriptInstanceData<T> {
    fn drop(&mut self) {
        // SAFETY: The ownership of ScriptInstaceData is transferred to Godot after it's creation. The engine then calls
        // script_instance_info::free_func when it frees its own GDExtensionScriptInstance and subsequently wants to drop the ScriptInstanceData.
        // After the the data has been dropped the instance info is no longer being used, but never freed. It is therefore safe to drop the
        // instace info at the same time.
        let instance = unsafe { Box::from_raw(self.gd_instance_ptr) };

        drop(instance);
    }
}

/// Creates a new [`sys::GDExtensionScriptInstancePtr`] from a type that implements [`ScriptInstance`].
///
// The return type of `*mut c_void` is actually a `sys::GDExtensionScriptInstancePtr` but this type is not used in the public API.
pub fn create_script_instance<T: ScriptInstance>(rs_instance: T) -> *mut c_void {
    let gd_instance = sys::GDExtensionScriptInstanceInfo {
        set_func: Some(script_instance_info::set_func::<T>),
        get_func: Some(script_instance_info::get_func::<T>),
        get_property_list_func: Some(script_instance_info::get_property_list_func::<T>),
        free_property_list_func: Some(script_instance_info::free_property_list_func::<T>),
        // unimplemented until it's clear if it's needed.
        property_get_revert_func: None,
        // unimplemented until it's clear if it's needed.
        property_can_revert_func: None,
        // script_instance::get_owner is never called by godot 4.0 to 4.2
        get_owner_func: None,
        get_property_state_func: Some(script_instance_info::get_property_state_func::<T>),
        get_script_func: Some(script_instance_info::get_script_func::<T>),
        get_language_func: Some(script_instance_info::get_language_func::<T>),
        get_method_list_func: Some(script_instance_info::get_method_list_func::<T>),
        get_property_type_func: Some(script_instance_info::get_property_type_func::<T>),
        free_func: Some(script_instance_info::free_func::<T>),
        free_method_list_func: Some(script_instance_info::free_method_list_func::<T>),
        has_method_func: Some(script_instance_info::has_method_func::<T>),
        call_func: Some(script_instance_info::call_func::<T>),
        // deprecated by Godot.
        notification_func: None,
        to_string_func: Some(script_instance_info::to_string_func::<T>),
        refcount_decremented_func: Some(script_instance_info::refcount_decremented_func::<T>),
        refcount_incremented_func: Some(script_instance_info::refcount_incremented_func::<T>),
        is_placeholder_func: Some(script_instance_info::is_placeholder_func::<T>),
        get_fallback_func: Some(script_instance_info::get_fallback_func::<T>),
        set_fallback_func: Some(script_instance_info::set_fallback_func::<T>),
    };

    let instance_ptr = Box::into_raw(Box::new(gd_instance));

    let data = ScriptInstanceData {
        inner: RefCell::new(rs_instance),
        gd_instance_ptr: instance_ptr,
    };

    let data_ptr = Box::into_raw(Box::new(data));

    // SAFETY: `script_instance_create` expects a `GDExtensionScriptInstanceInfoPtr` and a generic `GDExtensionScriptInstanceDataPtr` of our
    // choice. The validity of the instance info struct is ensured by code generation.
    //
    // It is expected that the engine upholds the safety invariants stated on each of the GDEXtensionScriptInstanceInfo functions.
    unsafe {
        sys::interface_fn!(script_instance_create)(
            instance_ptr,
            data_ptr as sys::GDExtensionScriptInstanceDataPtr,
        ) as *mut c_void
    }
}

mod script_instance_info {
    use std::any::type_name;
    use std::cell::{BorrowError, Ref, RefMut};
    use std::ffi::c_void;
    use std::mem::ManuallyDrop;
    use std::ops::Deref;

    use crate::builtin::{GString, StringName, Variant};
    use crate::engine::ScriptLanguage;
    use crate::obj::Gd;
    use crate::private::handle_panic;
    use crate::sys;

    use super::{ScriptInstance, ScriptInstanceData};

    fn borrow_instance_mut<T: ScriptInstance>(instance: &ScriptInstanceData<T>) -> RefMut<'_, T> {
        instance.inner.borrow_mut()
    }

    fn borrow_instance<T: ScriptInstance>(instance: &ScriptInstanceData<T>) -> Ref<'_, T> {
        instance.inner.borrow()
    }

    fn try_borrow_instance<T: ScriptInstance>(
        instance: &ScriptInstanceData<T>,
    ) -> Result<Ref<'_, T>, BorrowError> {
        instance.inner.try_borrow()
    }

    /// # Safety
    ///
    /// `p_instance` must be a valid pointer to a `ScriptInstanceData<T>` for the duration of `'a`.
    /// This pointer must have been created by [super::create_script_instance] and transfered to Godot.
    unsafe fn instance_data_as_script_instance<'a, T: ScriptInstance>(
        p_instance: sys::GDExtensionScriptInstanceDataPtr,
    ) -> &'a ScriptInstanceData<T> {
        &*(p_instance as *mut ScriptInstanceData<T>)
    }

    /// # Safety
    ///
    /// - We expect the engine to provide a valid const string name pointer.
    unsafe fn transfer_string_name_from_godot(
        p_name: sys::GDExtensionConstStringNamePtr,
    ) -> StringName {
        // This `StringName` is not ours and the engine will decrement the reference count later. To own our own `StringName` reference we
        // cannot call the destructor on the "original" `StringName`, but have to clone it to increase the reference count. This new instance can
        // then be passed around and eventually droped which will decrement the reference count again.
        //
        // By wrapping the initial `StringName` in a `ManuallyDrop` the destructor is prevented from being executed, which would decrement the
        // reference count.
        ManuallyDrop::new(StringName::from_string_sys(sys::force_mut_ptr(p_name)))
            .deref()
            .clone()
    }

    fn transfer_bool_to_godot(value: bool) -> sys::GDExtensionBool {
        value as sys::GDExtensionBool
    }

    /// # Safety
    ///
    /// - We expect the engine to provide a valid variant pointer the return value can be moved into.
    unsafe fn transfer_variant_to_godot(variant: Variant, return_ptr: sys::GDExtensionVariantPtr) {
        variant.move_var_ptr(return_ptr)
    }

    /// # Safety
    ///
    /// - The returned `*const T` is guaranteed to point to a list that has an equal length and capacity.
    fn transfer_ptr_list_to_godot<T>(ptr_list: Vec<T>, list_length: &mut u32) -> *const T {
        *list_length = ptr_list.len() as u32;

        let ptr = Box::into_raw(ptr_list.into_boxed_slice());

        // SAFETY: `ptr` was just created in the line above and should be safe to dereference.
        unsafe { (*ptr).as_ptr() }
    }

    /// The returned pointer's lifetime is equal to the lifetime of `script`
    fn transfer_script_to_godot(script: &Gd<crate::engine::Script>) -> sys::GDExtensionObjectPtr {
        script.obj_sys()
    }

    /// # Safety
    ///
    /// - `ptr` is expected to point to a list with both a length and capacity of `list_length`.
    /// - The list pointer `ptr` must have been created with [`transfer_ptr_list_to_godot`].
    unsafe fn transfer_ptr_list_from_godot<T>(ptr: *const T, list_length: usize) -> Vec<T> {
        Vec::from_raw_parts(sys::force_mut_ptr(ptr), list_length, list_length)
    }

    /// # Safety
    ///
    /// - The engine has to provide a valid string return pointer.
    unsafe fn transfer_string_to_godot(string: GString, return_ptr: sys::GDExtensionStringPtr) {
        string.move_string_ptr(return_ptr);
    }

    /// # Safety
    ///
    /// - `userdata` has to be a valid pointer that upholds the invariants of `sys::GDExtensionScriptInstancePropertyStateAdd`.
    unsafe fn transfer_property_state_to_godot(
        propery_states: Vec<(StringName, Variant)>,
        property_state_add: sys::GDExtensionScriptInstancePropertyStateAdd,
        userdata: *mut c_void,
    ) {
        let Some(property_state_add) = property_state_add else {
            return;
        };

        propery_states.into_iter().for_each(|(name, value)| {
            let name = ManuallyDrop::new(name);
            let value = ManuallyDrop::new(value);

            // SAFETY: Godot expects a string name and a variant pointer for each property. After receiving the pointer, the engine is responsible
            // for managing the memory behind those references.
            unsafe { property_state_add(name.string_sys(), value.var_sys(), userdata) };
        });
    }

    /// # Safety
    ///
    /// - `script_lang` must live for as long as the pointer may be dereferenced.
    unsafe fn transfer_script_lang_to_godot(
        script_lang: Gd<ScriptLanguage>,
    ) -> sys::GDExtensionScriptLanguagePtr {
        script_lang.obj_sys() as sys::GDExtensionScriptLanguagePtr
    }

    /// # Safety
    ///
    /// - `p_instance` has to be a valid pointer that can be cast to `*mut ScriptInstaceData<T>`.
    /// - `p_name` has to be a valid pointer to a StringName.
    /// - `p_value` has to be a valid pointer to a Variant.
    pub(super) unsafe extern "C" fn set_func<T: ScriptInstance>(
        p_instance: sys::GDExtensionScriptInstanceDataPtr,
        p_name: sys::GDExtensionConstStringNamePtr,
        p_value: sys::GDExtensionConstVariantPtr,
    ) -> sys::GDExtensionBool {
        let name = transfer_string_name_from_godot(p_name);
        let value = &*Variant::ptr_from_sys(p_value);
        let ctx = || format!("error when calling {}::set", type_name::<T>());

        let result = handle_panic(ctx, || {
            let instance = instance_data_as_script_instance::<T>(p_instance);

            borrow_instance_mut(instance).set(name, value)
        })
        // unwrapping to a default of false to indicate that the assignment as not handled by the script.
        .unwrap_or_default();

        transfer_bool_to_godot(result)
    }

    /// # Safety
    ///
    /// - `p_instance` has to be a valid pointer that can be cast to `*mut ScriptInstaceData<T>`.
    /// - `r_ret` will be a valid owned variant pointer after this call.
    pub(super) unsafe extern "C" fn get_func<T: ScriptInstance>(
        p_instance: sys::GDExtensionScriptInstanceDataPtr,
        p_name: sys::GDExtensionConstStringNamePtr,
        r_ret: sys::GDExtensionVariantPtr,
    ) -> sys::GDExtensionBool {
        let name = transfer_string_name_from_godot(p_name);
        let ctx = || format!("error when calling {}::get", type_name::<T>());

        let return_value = handle_panic(ctx, || {
            let instance = instance_data_as_script_instance::<T>(p_instance);

            borrow_instance(instance).get(name.clone())
        });

        let result = match return_value {
            Some(return_value) => return_value
                .map(|variant| transfer_variant_to_godot(variant, r_ret))
                .is_some(),
            None => false,
        };

        transfer_bool_to_godot(result)
    }

    /// # Safety
    ///
    /// - `p_instance` has to be a valid pointer that can be cast to `*mut ScriptInstaceData<T>`.
    /// - `r_count` is expected to be a valid pointer to an u32.
    pub(super) unsafe extern "C" fn get_property_list_func<T: ScriptInstance>(
        p_instance: sys::GDExtensionScriptInstanceDataPtr,
        r_count: *mut u32,
    ) -> *const sys::GDExtensionPropertyInfo {
        let ctx = || format!("error when calling {}::get_property_list", type_name::<T>());

        let property_list = handle_panic(ctx, || {
            let instance = instance_data_as_script_instance::<T>(p_instance);

            let property_list: Vec<_> = borrow_instance(instance)
                .get_property_list()
                .iter()
                .map(|prop| prop.property_sys())
                .collect();

            property_list
        })
        .unwrap_or_default();

        // SAFETY: list_length has to be a valid pointer to a u32.
        let list_length = unsafe { &mut *r_count };

        transfer_ptr_list_to_godot(property_list, list_length)
    }

    /// # Safety
    ///
    /// - `p_instance` has to be a valid pointer that can be cast to `*mut ScriptInstaceData<T>`.
    /// - `r_count` is expected to be a valid pointer to an u32.
    pub(super) unsafe extern "C" fn get_method_list_func<T: ScriptInstance>(
        p_instance: sys::GDExtensionScriptInstanceDataPtr,
        r_count: *mut u32,
    ) -> *const sys::GDExtensionMethodInfo {
        let ctx = || format!("error when calling {}::get_method_list", type_name::<T>());

        let method_list = handle_panic(ctx, || {
            let instance = instance_data_as_script_instance::<T>(p_instance);

            let method_list: Vec<_> = borrow_instance(instance)
                .get_method_list()
                .iter()
                .map(|method| method.method_sys())
                .collect();

            method_list
        })
        .unwrap_or_default();

        // SAFETY: list_length has to be a valid pointer to a u32.
        let list_length = unsafe { &mut *r_count };

        transfer_ptr_list_to_godot(method_list, list_length)
    }

    /// # Safety
    ///
    /// - `p_instance` has to be a valid pointer that can be cast to `*mut ScriptInstaceData<T>`.
    /// - The length of `p_prop_info` list is expected to not have changed since it was transferred to the engine.
    pub(super) unsafe extern "C" fn free_property_list_func<T: ScriptInstance>(
        p_instance: sys::GDExtensionScriptInstanceDataPtr,
        p_prop_info: *const sys::GDExtensionPropertyInfo,
    ) {
        let ctx = || {
            format!(
                "error while calling {}::get_property_list",
                type_name::<T>()
            )
        };

        let length = handle_panic(ctx, || {
            let instance = instance_data_as_script_instance::<T>(p_instance);

            borrow_instance(instance).get_property_list().len()
        })
        .unwrap_or_default();

        // SAFETY: p_prop_info is expected to have been created by get_property_list_func
        // and therefore should have the same length as before. get_propery_list_func
        // also guarantees that both vector length and capacity are equal.
        let _drop = transfer_ptr_list_from_godot(p_prop_info, length);
    }

    /// # Safety
    ///
    /// - `p_instance` has to be a valid pointer that can be cast to `*mut ScriptInstaceData<T>`.
    /// - `p_method` has to be a valid Godot string name ptr.
    /// - `p_args` has to point to a list of Variant pointers of length `p_argument_count`.
    /// - `r_return` has to be a valid variant pointer into which the return value can be moved.
    /// - `r_error` has to point to a valid [`GDExtenstionCallError`] which can be modified to reflect the outcome of the method call.
    pub(super) unsafe extern "C" fn call_func<T: ScriptInstance>(
        p_self: sys::GDExtensionScriptInstanceDataPtr,
        p_method: sys::GDExtensionConstStringNamePtr,
        p_args: *const sys::GDExtensionConstVariantPtr,
        p_argument_count: sys::GDExtensionInt,
        r_return: sys::GDExtensionVariantPtr,
        r_error: *mut sys::GDExtensionCallError,
    ) {
        let method = transfer_string_name_from_godot(p_method);
        let args = Variant::unbounded_refs_from_sys(p_args, p_argument_count as usize);
        let ctx = || format!("error when calling {}::call", type_name::<T>());

        let result = handle_panic(ctx, || {
            let instance = instance_data_as_script_instance::<T>(p_self);
            borrow_instance_mut(instance).call(method.clone(), args)
        });

        match result {
            Some(Ok(ret)) => {
                transfer_variant_to_godot(ret, r_return);
                (*r_error).error = sys::GDEXTENSION_CALL_OK;
            }

            Some(Err(err)) => (*r_error).error = err,
            None => {
                (*r_error).error = sys::GDEXTENSION_CALL_ERROR_INVALID_METHOD;
            }
        };
    }

    /// # Safety
    ///
    /// - `p_instance` has to be a valid pointer that can be cast to `*mut ScriptInstaceData<T>`.
    /// - A pointer to a `Script` reference will be returned. The caller is then responsible for freeing the reference, including the adjustment
    ///   of the reference count.
    pub(super) unsafe extern "C" fn get_script_func<T: ScriptInstance>(
        p_instance: sys::GDExtensionScriptInstanceDataPtr,
    ) -> sys::GDExtensionObjectPtr {
        let ctx = || format!("error when calling {}::get_script", type_name::<T>());

        let script = handle_panic(ctx, || {
            let instance = instance_data_as_script_instance::<T>(p_instance);

            borrow_instance(instance).get_script().to_owned()
        });

        match script {
            Some(script) => transfer_script_to_godot(&script),
            None => std::ptr::null_mut(),
        }
    }

    /// # Safety
    ///
    /// - `p_instance` has to be a valid pointer that can be cast to `*mut ScriptInstaceData<T>`.
    /// - The boolean result is returned as an GDExtensionBool.
    pub(super) unsafe extern "C" fn is_placeholder_func<T: ScriptInstance>(
        p_instance: sys::GDExtensionScriptInstanceDataPtr,
    ) -> sys::GDExtensionBool {
        let ctx = || format!("error when calling {}::is_placeholder", type_name::<T>());

        let is_placeholder = handle_panic(ctx, || {
            let instance = instance_data_as_script_instance::<T>(p_instance);

            borrow_instance(instance).is_placeholder()
        })
        .unwrap_or_default();

        transfer_bool_to_godot(is_placeholder)
    }

    /// # Safety
    ///
    /// - `p_instance` has to be a valid pointer that can be cast to `*mut ScriptInstaceData<T>`.
    /// - `p_method` has to point to a valid string name.
    pub(super) unsafe extern "C" fn has_method_func<T: ScriptInstance>(
        p_instance: sys::GDExtensionScriptInstanceDataPtr,
        p_method: sys::GDExtensionConstStringNamePtr,
    ) -> sys::GDExtensionBool {
        let method = transfer_string_name_from_godot(p_method);
        let ctx = || format!("error when calling {}::has_method", type_name::<T>());

        let has_method = handle_panic(ctx, || {
            let instance = instance_data_as_script_instance::<T>(p_instance);

            borrow_instance(instance).has_method(method.clone())
        })
        .unwrap_or_default();

        transfer_bool_to_godot(has_method)
    }

    /// # Safety
    ///
    /// - `p_instance` has to be a valid pointer that can be cast to `*mut ScriptInstaceData<T>`.
    /// - The length of `p_method_info` list is expected to not have changed since it was transferred to the engine.
    pub(super) unsafe extern "C" fn free_method_list_func<T: ScriptInstance>(
        p_instance: sys::GDExtensionScriptInstanceDataPtr,
        p_method_info: *const sys::GDExtensionMethodInfo,
    ) {
        let ctx = || format!("error while calling {}::get_method_list", type_name::<T>());

        let length = handle_panic(ctx, || {
            let instance = instance_data_as_script_instance::<T>(p_instance);

            borrow_instance(instance).get_method_list().len()
        })
        .unwrap_or_default();

        // SAFETY: p_method_info is expected to have been created by get_method_list_func and therefore should have the same length as before.
        // get_method_list_func also guarantees that both vector length and capacity are equal.
        let vec = transfer_ptr_list_from_godot(p_method_info, length);

        vec.into_iter().for_each(|method_info| {
            transfer_ptr_list_from_godot(
                method_info.arguments,
                method_info.argument_count as usize,
            );
            transfer_ptr_list_from_godot(
                method_info.default_arguments,
                method_info.default_argument_count as usize,
            );
        })
    }

    /// # Safety
    ///
    /// - `p_instance` has to be a valid pointer that can be cast to `*mut ScriptInstaceData<T>`.
    /// - `p_name` is expected to be a valid ptr to a Godot string name.
    /// - `r_is_valid` is expected to be a valid ptr to a [`GDExtensionBool`] which can be modified to reflect the validity of the return value.
    pub(super) unsafe extern "C" fn get_property_type_func<T: ScriptInstance>(
        p_instance: sys::GDExtensionScriptInstanceDataPtr,
        p_name: sys::GDExtensionConstStringNamePtr,
        r_is_valid: *mut sys::GDExtensionBool,
    ) -> sys::GDExtensionVariantType {
        let ctx = || {
            format!(
                "error while calling {}::get_property_type",
                type_name::<T>()
            )
        };
        let name = transfer_string_name_from_godot(p_name);

        let result = handle_panic(ctx, || {
            let instance = instance_data_as_script_instance::<T>(p_instance);

            borrow_instance(instance).get_property_type(name.clone())
        });

        let Some(result) = result else {
            *r_is_valid = transfer_bool_to_godot(false);
            return 0;
        };

        *r_is_valid = transfer_bool_to_godot(true);
        result.sys()
    }

    /// # Safety
    ///
    /// - `p_instance` has to be a valid pointer that can be cast to `*mut ScriptInstaceData<T>`.
    /// - `r_is_valid` is expected to be a valid [`GDEXtensionBool`] ptr which can be modified to reflect the validity of the return value.
    /// - `r_str` is expected to be a string pointer into which the return value can be moved.
    pub(super) unsafe extern "C" fn to_string_func<T: ScriptInstance>(
        p_instance: sys::GDExtensionScriptInstanceDataPtr,
        r_is_valid: *mut sys::GDExtensionBool,
        r_str: sys::GDExtensionStringPtr,
    ) {
        let ctx = || format!("error when calling {}::to_string", type_name::<T>());

        let string = handle_panic(ctx, || {
            let instance = instance_data_as_script_instance::<T>(p_instance);

            let Ok(inner) = try_borrow_instance(instance) else {
                // to_string of a script instance can be called when calling to_string on the owning base object. In this case we pretend like we
                // can't handle the call and leave r_is_valid at it's default value of false.
                //
                // This is one of the  only members of GDExtensionScripInstanceInfo which appeares to be called from an API function
                // (beside get_func, set_func, call_func). The unexpected behavior here is that it is being called as a replacement of Godots
                // Object::to_string for the owner object. This then also happens when trying to call to_string on the base object inside a
                // script, which feels wrong, and most importantly, would obviously cause a panic when acquiring the Ref guard.

                return None;
            };

            Some(inner.to_string())
        })
        .flatten();

        let Some(string) = string else {
            return;
        };

        *r_is_valid = transfer_bool_to_godot(true);
        transfer_string_to_godot(string, r_str);
    }

    /// # Safety
    ///
    /// - `p_instance` has to be a valid pointer that can be cast to `*mut ScriptInstaceData<T>`.
    /// - A string name ptr and a const Variant ptr are passed for each script property to the `property_state_add` callback function. The callback is then
    ///   responsible for freeing the memory.
    /// - `userdata` has to be a valid pointer that satisfies the invariants of `property_state_add`.
    pub(super) unsafe extern "C" fn get_property_state_func<T: ScriptInstance>(
        p_instance: sys::GDExtensionScriptInstanceDataPtr,
        property_state_add: sys::GDExtensionScriptInstancePropertyStateAdd,
        userdata: *mut c_void,
    ) {
        let ctx = || {
            format!(
                "error when calling {}::get_property_state",
                type_name::<T>()
            )
        };

        let property_states = handle_panic(ctx, || {
            let instance = instance_data_as_script_instance::<T>(p_instance);

            borrow_instance(instance).get_property_state()
        })
        .unwrap_or_default();

        transfer_property_state_to_godot(property_states, property_state_add, userdata);
    }

    /// # Safety
    ///
    /// - `p_instance` has to be a valid pointer that can be cast to `*mut ScriptInstaceData<T>`.
    /// - A ptr to a [`ScriptLanguage´] reference will be returned, ScriptLanguage is a manually managed object and the caller has to verify
    ///   it's validity as it could be freed at any time.
    pub(super) unsafe extern "C" fn get_language_func<T: ScriptInstance>(
        p_instance: sys::GDExtensionScriptInstanceDataPtr,
    ) -> sys::GDExtensionScriptLanguagePtr {
        let ctx = || format!("error when calling {}::get_language", type_name::<T>());

        let language = handle_panic(ctx, || {
            let instance = instance_data_as_script_instance::<T>(p_instance);

            borrow_instance(instance).get_language()
        });

        let Some(language) = language else {
            return std::ptr::null::<c_void>() as sys::GDExtensionScriptLanguagePtr;
        };

        transfer_script_lang_to_godot(language)
    }

    /// # Safety
    ///
    /// - `p_instance` has to be a valid pointer that can be cast to `*mut ScriptInstaceData<T>`.
    /// - The instance data will be freed and the pointer won't be valid anymore after this function has been called.
    pub(super) unsafe extern "C" fn free_func<T: ScriptInstance>(
        p_instance: sys::GDExtensionScriptInstanceDataPtr,
    ) {
        drop(Box::from_raw(p_instance as *mut ScriptInstanceData<T>));
    }

    /// # Safety
    ///
    /// - `p_instance` has to be a valid pointer that can be cast to `*mut ScriptInstaceData<T>`.
    pub(super) unsafe extern "C" fn refcount_decremented_func<T: ScriptInstance>(
        p_instance: sys::GDExtensionScriptInstanceDataPtr,
    ) -> sys::GDExtensionBool {
        let ctx = || {
            format!(
                "error when calling {}::refcount_decremented",
                type_name::<T>()
            )
        };

        let result = handle_panic(ctx, || {
            let instance = instance_data_as_script_instance::<T>(p_instance);

            borrow_instance(instance).on_refcount_decremented()
        })
        .unwrap_or(true);

        transfer_bool_to_godot(result)
    }

    /// # Safety
    ///
    /// - `p_instance` has to be a valid pointer that can be cast to `*mut ScriptInstaceData<T>`.
    pub(super) unsafe extern "C" fn refcount_incremented_func<T: ScriptInstance>(
        p_instance: sys::GDExtensionScriptInstanceDataPtr,
    ) {
        let ctx = || {
            format!(
                "error when calling {}::refcount_incremented",
                type_name::<T>()
            )
        };

        handle_panic(ctx, || {
            let instance = instance_data_as_script_instance::<T>(p_instance);

            borrow_instance(instance).on_refcount_incremented();
        })
        .unwrap_or_default();
    }

    /// # Safety
    ///
    /// - `p_instance` has to be a valid pointer that can be cast to `*mut ScriptInstaceData<T>`.
    /// - `p_name` has to be a valid pointer to a `StringName`.
    /// - `r_ret` has to be a valid pointer to which the return value can be moved.
    pub(super) unsafe extern "C" fn get_fallback_func<T: ScriptInstance>(
        p_instance: sys::GDExtensionScriptInstanceDataPtr,
        p_name: sys::GDExtensionConstStringNamePtr,
        r_ret: sys::GDExtensionVariantPtr,
    ) -> sys::GDExtensionBool {
        let name = transfer_string_name_from_godot(p_name);

        let ctx = || {
            format!(
                "error when calling {}::property_get_fallback",
                type_name::<T>()
            )
        };

        let return_value = handle_panic(ctx, || {
            let instance = instance_data_as_script_instance::<T>(p_instance);

            borrow_instance(instance).property_get_fallback(name)
        });

        let result = match return_value {
            Some(return_value) => return_value
                .map(|value| transfer_variant_to_godot(value, r_ret))
                .is_some(),
            None => false,
        };

        transfer_bool_to_godot(result)
    }

    /// # Safety
    ///
    /// - `p_instance` has to be a valid pointer that can be cast to `*mut ScriptInstaceData<T>`.
    /// - `p_name` has to be a valid pointer to a `StringName`.
    pub(super) unsafe extern "C" fn set_fallback_func<T: ScriptInstance>(
        p_instance: sys::GDExtensionScriptInstanceDataPtr,
        p_name: sys::GDExtensionConstStringNamePtr,
        p_value: sys::GDExtensionConstVariantPtr,
    ) -> sys::GDExtensionBool {
        let name = transfer_string_name_from_godot(p_name);
        let value = &*Variant::ptr_from_sys(p_value);

        let ctx = || {
            format!(
                "error when calling {}::property_set_fallback",
                type_name::<T>()
            )
        };

        let result = handle_panic(ctx, || {
            let instance = instance_data_as_script_instance::<T>(p_instance);

            borrow_instance_mut(instance).property_set_fallback(name, value)
        })
        .unwrap_or_default();

        transfer_bool_to_godot(result)
    }
}
