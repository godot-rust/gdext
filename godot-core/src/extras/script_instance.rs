/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::collections::HashMap;
use std::ffi::c_void;
use std::ops::{Deref, DerefMut};
use std::pin::Pin;
use std::sync::Mutex;

use godot_cell::{GdCell, MutGuard};

use crate::builtin::meta::{MethodInfo, PropertyInfo};
use crate::builtin::{GString, StringName, Variant, VariantType};
use crate::engine::{Script, ScriptLanguage};
use crate::obj::{Base, Gd, GodotClass, ScriptBaseMut, ScriptBaseRef};
use crate::sys;

/// Implement custom scripts that can be attached to objects in Godot.
///
/// To use script instances, implement this trait for your own type.
///
/// You can use the [`create_script_instance()`] function to create a low-level pointer to your script instance.
/// This pointer should then be returned from [`IScriptExtension::instance_create()`](crate::engine::IScriptExtension::instance_create).
///
/// # Example
///
/// ```no_run
/// # use godot::prelude::*;
/// # use godot::engine::{IScriptExtension, Script, ScriptExtension};
/// # trait ScriptInstance {} // trick 17 to avoid listing all the methods. Needs also a method.
/// # fn create_script_instance(_: MyScriptInstance) -> *mut std::ffi::c_void { std::ptr::null_mut() }
/// // 1) Define the script.
/// #[derive(GodotClass)]
/// #[class(init, base=ScriptExtension)]
/// struct MyScript {
///    base: Base<ScriptExtension>,
///    // ... other fields
/// }
///
/// // 2) Define the script _instance_, and implement the trait for it.
/// struct MyScriptInstance;
/// impl MyScriptInstance {
///     fn from_gd(script: Gd<Script>) -> Self {
///         Self { /* ... */ }
///     }
/// }
///
/// impl ScriptInstance for MyScriptInstance {
///     // Implement all the methods...
/// }
///
/// // 3) Implement the script's virtual interface to wire up 1) and 2).
/// #[godot_api]
/// impl IScriptExtension for MyScript {
///     unsafe fn instance_create(&self, _for_object: Gd<Object>) -> *mut std::ffi::c_void {
///         // Upcast Gd<ScriptExtension> to Gd<Script>.
///         let script = self.to_gd().upcast();
///         let script_instance = MyScriptInstance::from_gd(script);
///
///         // Note on safety: the returned pointer must be obtained
///         // through create_script_instance().
///         create_script_instance(script_instance)
///     }
/// }
/// ```
pub trait ScriptInstance: Sized {
    type Base: GodotClass;

    /// Name of the new class the script implements.
    fn class_name(&self) -> GString;

    /// Property setter for Godot's virtual dispatch system.
    ///
    /// The engine will call this function when it wants to change a property on the script.
    fn set_property(this: SiMut<Self>, name: StringName, value: &Variant) -> bool;

    /// Property getter for Godot's virtual dispatch system.
    ///
    /// The engine will call this function when it wants to read a property on the script.
    fn get_property(&self, name: StringName) -> Option<Variant>;

    /// A list of all the properties a script exposes to the engine.
    fn get_property_list(&self) -> Vec<PropertyInfo>;

    /// A list of all the methods a script exposes to the engine.
    fn get_method_list(&self) -> Vec<MethodInfo>;

    /// Method invoker for Godot's virtual dispatch system. The engine will call this function when it wants to call a method on the script.
    ///
    /// All method calls are taking a mutable reference of the script instance, as the engine does not differentiate between immutable and
    /// mutable method calls like rust.
    ///
    /// It's important that the script does not cause a second call to this function while executing a method call. This would result in a panic.
    // TODO: map the sys::GDExtensionCallErrorType to some public API type.
    fn call(
        this: SiMut<Self>,
        method: StringName,
        args: &[&Variant],
    ) -> Result<Variant, sys::GDExtensionCallErrorType>;

    /// Identifies the script instance as a placeholder. If this function and
    /// [IScriptExtension::is_placeholder_fallback_enabled](crate::engine::IScriptExtension::is_placeholder_fallback_enabled) return true,
    /// Godot will call [`Self::property_set_fallback`] instead of [`Self::set_property`].
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

    /// The engine may call this function if it failed to get a property value via [ScriptInstance::get_property] or the native types getter.
    fn property_get_fallback(&self, name: StringName) -> Option<Variant>;

    /// The engine may call this function if ScriptLanguage::is_placeholder_fallback_enabled is enabled.
    fn property_set_fallback(this: SiMut<Self>, name: StringName, value: &Variant) -> bool;
}

#[cfg(before_api = "4.2")]
type ScriptInstanceInfo = sys::GDExtensionScriptInstanceInfo;
#[cfg(since_api = "4.2")]
type ScriptInstanceInfo = sys::GDExtensionScriptInstanceInfo2;

struct ScriptInstanceData<T: ScriptInstance> {
    inner: Pin<Box<GdCell<T>>>,
    script_instance_ptr: *mut ScriptInstanceInfo,
    property_list: Mutex<HashMap<*const sys::GDExtensionPropertyInfo, Vec<PropertyInfo>>>,
    method_list: Mutex<HashMap<*const sys::GDExtensionMethodInfo, Vec<MethodInfo>>>,
    base: Base<T::Base>,
}

impl<T: ScriptInstance> Drop for ScriptInstanceData<T> {
    fn drop(&mut self) {
        // SAFETY: The ownership of ScriptInstanceData is transferred to Godot after its creation. The engine then calls
        // script_instance_info::free_func when it frees its own GDExtensionScriptInstance and subsequently wants to drop the ScriptInstanceData.
        // After the data has been dropped, the instance info is no longer being used, but never freed. It is therefore safe to drop the
        // instance info at the same time.
        let instance = unsafe { Box::from_raw(self.script_instance_ptr) };

        drop(instance);
    }
}

/// Creates a new  from a type that implements [`ScriptInstance`].
///
/// See [`ScriptInstance`] for usage. Discarding the resulting value will result in a memory leak.
///
/// The exact GDExtension type of the pointer is `sys::GDExtensionScriptInstancePtr`, but you can treat it like an opaque pointer.
///
/// # Safety
/// The caller must ensure that `for_object` is not freed before passing the returned pointer back to Godot.
#[must_use]
pub unsafe fn create_script_instance<T: ScriptInstance>(
    rust_instance: T,
    for_object: Gd<T::Base>,
) -> *mut c_void {
    // Field grouping matches C header.
    let gd_instance = ScriptInstanceInfo {
        set_func: Some(script_instance_info::set_property_func::<T>),
        get_func: Some(script_instance_info::get_property_func::<T>),
        get_property_list_func: Some(script_instance_info::get_property_list_func::<T>),
        free_property_list_func: Some(script_instance_info::free_property_list_func::<T>),

        #[cfg(since_api = "4.2")]
        get_class_category_func: None, // not yet implemented.

        property_can_revert_func: None, // unimplemented until needed.
        property_get_revert_func: None, // unimplemented until needed.

        // ScriptInstance::get_owner() is apparently not called by Godot 4.0 to 4.2 (to verify).
        get_owner_func: None,
        get_property_state_func: Some(script_instance_info::get_property_state_func::<T>),

        get_method_list_func: Some(script_instance_info::get_method_list_func::<T>),
        free_method_list_func: Some(script_instance_info::free_method_list_func::<T>),
        get_property_type_func: Some(script_instance_info::get_property_type_func::<T>),
        #[cfg(since_api = "4.2")]
        validate_property_func: None, // not yet implemented.

        has_method_func: Some(script_instance_info::has_method_func::<T>),

        call_func: Some(script_instance_info::call_func::<T>),
        notification_func: None, // not yet implemented.

        to_string_func: Some(script_instance_info::to_string_func::<T>),

        refcount_incremented_func: Some(script_instance_info::refcount_incremented_func::<T>),
        refcount_decremented_func: Some(script_instance_info::refcount_decremented_func::<T>),

        get_script_func: Some(script_instance_info::get_script_func::<T>),

        is_placeholder_func: Some(script_instance_info::is_placeholder_func::<T>),

        get_fallback_func: Some(script_instance_info::get_fallback_func::<T>),
        set_fallback_func: Some(script_instance_info::set_fallback_func::<T>),

        get_language_func: Some(script_instance_info::get_language_func::<T>),

        free_func: Some(script_instance_info::free_func::<T>),
    };

    let instance_ptr = Box::into_raw(Box::new(gd_instance));

    let data = ScriptInstanceData {
        inner: GdCell::new(rust_instance),
        script_instance_ptr: instance_ptr,
        property_list: Default::default(),
        method_list: Default::default(),
        // SAFETY: The script instance is always freed before the base object is destroyed. The weak reference should therefore never be
        // accessed after it has been freed.
        base: unsafe { Base::from_gd(&for_object) },
    };

    let data_ptr = Box::into_raw(Box::new(data));

    // SAFETY: `script_instance_create` expects a `GDExtensionScriptInstanceInfoPtr` and a generic `GDExtensionScriptInstanceDataPtr` of our
    // choice. The validity of the instance info struct is ensured by code generation.
    //
    // It is expected that the engine upholds the safety invariants stated on each of the GDEXtensionScriptInstanceInfo functions.
    unsafe {
        #[cfg(before_api = "4.2")]
        let create_fn = sys::interface_fn!(script_instance_create);

        #[cfg(since_api = "4.2")]
        let create_fn = sys::interface_fn!(script_instance_create2);

        create_fn(
            instance_ptr,
            data_ptr as sys::GDExtensionScriptInstanceDataPtr,
        ) as *mut c_void
    }
}

/// Mutable/exclusive reference guard for a `T` where `T` implements [`ScriptInstance`].
///
/// This can be used to access the base object of a [`ScriptInstance`], which in turn can be used to make reentrant calls to engine APIs.
/// For details see [`SiMut::base_mut()`].
pub struct SiMut<'a, T: ScriptInstance> {
    mut_ref: &'a mut T,
    cell: Pin<&'a GdCell<T>>,
    base_ref: &'a Base<T::Base>,
}

impl<'a, T: ScriptInstance> SiMut<'a, T> {
    fn new(
        cell: Pin<&'a GdCell<T>>,
        cell_guard: &'a mut MutGuard<T>,
        base_ref: &'a Base<T::Base>,
    ) -> Self {
        let mut_ref = cell_guard.deref_mut();

        Self {
            mut_ref,
            cell,
            base_ref,
        }
    }

    /// Returns a shared reference suitable for calling engine methods on this object.
    ///
    /// ```no_run
    /// # use godot::prelude::*;
    /// # use godot::engine::{ScriptInstance, SiMut, ScriptLanguage, Script};
    /// # use godot::builtin::meta::{MethodInfo, PropertyInfo};
    /// # use godot::sys;
    /// struct ExampleScriptInstance;
    ///
    /// impl ScriptInstance for ExampleScriptInstance {
    ///     type Base = Node;
    ///
    ///     fn call(
    ///         this: SiMut<Self>,
    ///         method: StringName,
    ///         args: &[&Variant],
    ///     ) -> Result<Variant, sys::GDExtensionCallErrorType>{
    ///         let name = this.base().get_name();
    ///         godot_print!("name is {name}");
    ///         // However, we cannot call methods that require `&mut Base`, such as:
    ///         // this.base().add_child(node);
    ///         Ok(Variant::nil())
    ///     }
    ///     # fn class_name(&self) -> GString { todo!() }
    ///     # fn set_property(_: SiMut<'_, Self>, _: godot::prelude::StringName, _: &godot::prelude::Variant) -> bool { todo!() }
    ///     # fn get_property(&self, _: godot::prelude::StringName) -> Option<godot::prelude::Variant> { todo!() }
    ///     # fn get_property_list(&self) -> Vec<PropertyInfo> { todo!() }
    ///     # fn get_method_list(&self) -> Vec<MethodInfo> { todo!() }
    ///     # fn is_placeholder(&self) -> bool { todo!() }
    ///     # fn has_method(&self, _: godot::prelude::StringName) -> bool { todo!() }
    ///     # fn get_script(&self) -> &Gd<Script> { todo!() }
    ///     # fn get_property_type(&self, _: godot::prelude::StringName) -> VariantType { todo!() }
    ///     # fn to_string(&self) -> GString { todo!() }
    ///     # fn get_property_state(&self) -> Vec<(godot::prelude::StringName, godot::prelude::Variant)> { todo!() }
    ///     # fn get_language(&self) -> Gd<ScriptLanguage> { todo!() }
    ///     # fn on_refcount_decremented(&self) -> bool { todo!() }
    ///     # fn on_refcount_incremented(&self) { todo!() }
    ///     # fn property_get_fallback(&self, _: godot::prelude::StringName) -> Option<godot::prelude::Variant> { todo!() }
    ///     # fn property_set_fallback(_: SiMut<'_, Self>, _: godot::prelude::StringName, _: &godot::prelude::Variant) -> bool { todo!() }
    /// }
    /// ```
    pub fn base(&self) -> ScriptBaseRef<T> {
        ScriptBaseRef::new(self.base_ref.to_gd(), self.mut_ref)
    }

    /// Returns a mutable reference suitable for calling engine methods on this object.
    ///
    /// This method will allow you to call back into the same object from Godot.
    ///
    /// ```no_run
    /// # use godot::prelude::*;
    /// # use godot::engine::{ScriptInstance, SiMut, ScriptLanguage, Script};
    /// # use godot::builtin::meta::{MethodInfo, PropertyInfo};
    /// # use godot::sys;
    /// struct ExampleScriptInstance;
    ///
    /// impl ScriptInstance for ExampleScriptInstance {
    ///     type Base = Object;
    ///
    ///     fn call(
    ///         mut this: SiMut<Self>,
    ///         method: StringName,
    ///         args: &[&Variant],
    ///     ) -> Result<Variant, sys::GDExtensionCallErrorType> {
    ///         // Check whether method is available on this script
    ///         if method == StringName::from("script_method") {
    ///             godot_print!("script_method called!");
    ///             return Ok(true.to_variant());
    ///         }
    ///
    ///         let node = Node::new_alloc();
    ///
    ///         // We can call back into `self` through Godot:
    ///         this.base_mut().call("script_method".into(), &[]);
    ///
    ///         Ok(Variant::nil())
    ///     }
    ///     # fn class_name(&self) -> GString { todo!() }
    ///     # fn set_property(_: SiMut<'_, Self>, _: godot::prelude::StringName, _: &godot::prelude::Variant) -> bool { todo!() }
    ///     # fn get_property(&self, _: godot::prelude::StringName) -> Option<godot::prelude::Variant> { todo!() }
    ///     # fn get_property_list(&self) -> Vec<PropertyInfo> { todo!() }
    ///     # fn get_method_list(&self) -> Vec<MethodInfo> { todo!() }
    ///     # fn is_placeholder(&self) -> bool { todo!() }
    ///     # fn has_method(&self, _: godot::prelude::StringName) -> bool { todo!() }
    ///     # fn get_script(&self) -> &Gd<Script> { todo!() }
    ///     # fn get_property_type(&self, _: godot::prelude::StringName) -> VariantType { todo!() }
    ///     # fn to_string(&self) -> GString { todo!() }
    ///     # fn get_property_state(&self) -> Vec<(godot::prelude::StringName, godot::prelude::Variant)> { todo!() }
    ///     # fn get_language(&self) -> Gd<ScriptLanguage> { todo!() }
    ///     # fn on_refcount_decremented(&self) -> bool { todo!() }
    ///     # fn on_refcount_incremented(&self) { todo!() }
    ///     # fn property_get_fallback(&self, _: godot::prelude::StringName) -> Option<godot::prelude::Variant> { todo!() }
    ///     # fn property_set_fallback(_: SiMut<'_, Self>, _: godot::prelude::StringName, _: &godot::prelude::Variant) -> bool { todo!() }
    /// }
    /// ```
    pub fn base_mut(&mut self) -> ScriptBaseMut<T> {
        let guard = self.cell.make_inaccessible(self.mut_ref).unwrap();

        ScriptBaseMut::new(self.base_ref.to_gd(), guard)
    }
}

impl<'a, T: ScriptInstance> Deref for SiMut<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.mut_ref
    }
}

impl<'a, T: ScriptInstance> DerefMut for SiMut<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.mut_ref
    }
}

mod script_instance_info {
    use std::any::type_name;
    use std::ffi::c_void;
    use std::mem::ManuallyDrop;
    use std::pin::Pin;

    use godot_cell::{GdCell, RefGuard};

    use crate::builtin::{GString, StringName, Variant};
    use crate::engine::ScriptLanguage;
    use crate::obj::Gd;
    use crate::private::handle_panic;
    use crate::sys;

    use super::{ScriptInstance, ScriptInstanceData, SiMut};

    fn borrow_panic<T: ScriptInstance, R>(err: Box<dyn std::error::Error>) -> R {
        panic!(
            "\
                ScriptInstance borrow failed, already bound; T = {}.\n  \
                Make sure to use `SiMut::base_mut()` when possible.\n  \
                Details: {err}.\
            ",
            type_name::<T>(),
        );
    }

    fn borrow_instance<T: ScriptInstance>(instance: &ScriptInstanceData<T>) -> RefGuard<'_, T> {
        instance
            .inner
            .as_ref()
            .borrow()
            .unwrap_or_else(borrow_panic::<T, _>)
    }

    fn borrow_instance_cell<T: ScriptInstance>(
        instance: &ScriptInstanceData<T>,
    ) -> Pin<&GdCell<T>> {
        instance.inner.as_ref()
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

    fn transfer_bool_to_godot(value: bool) -> sys::GDExtensionBool {
        value as sys::GDExtensionBool
    }

    /// # Safety
    ///
    /// - We expect the engine to provide a valid variant pointer the return value can be moved into.
    unsafe fn transfer_variant_to_godot(variant: Variant, return_ptr: sys::GDExtensionVariantPtr) {
        variant.move_into_var_ptr(return_ptr)
    }

    /// # Safety
    ///
    /// - The returned `*const T` is guaranteed to point to a list that has an equal length and capacity.
    fn transfer_ptr_list_to_godot<T>(ptr_list: Box<[T]>, list_length: &mut u32) -> *mut T {
        *list_length = ptr_list.len() as u32;

        let ptr = Box::into_raw(ptr_list);

        // SAFETY: `ptr` was just created in the line above and should be safe to dereference.
        unsafe { (*ptr).as_mut_ptr() }
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
        string.move_into_string_ptr(return_ptr);
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
    /// - `p_instance` has to be a valid pointer that can be cast to `*mut ScriptInstanceData<T>`.
    /// - `p_name` has to be a valid pointer to a StringName.
    /// - `p_value` has to be a valid pointer to a Variant.
    pub(super) unsafe extern "C" fn set_property_func<T: ScriptInstance>(
        p_instance: sys::GDExtensionScriptInstanceDataPtr,
        p_name: sys::GDExtensionConstStringNamePtr,
        p_value: sys::GDExtensionConstVariantPtr,
    ) -> sys::GDExtensionBool {
        let name = StringName::new_from_string_sys(p_name);
        let value = Variant::borrow_var_sys(p_value);
        let ctx = || format!("error when calling {}::set", type_name::<T>());

        let result = handle_panic(ctx, || {
            let instance = instance_data_as_script_instance::<T>(p_instance);
            let cell = borrow_instance_cell(instance);
            let mut guard = cell
                .as_ref()
                .borrow_mut()
                .unwrap_or_else(borrow_panic::<T, _>);

            let instance_guard = SiMut::new(cell, &mut guard, &instance.base);

            ScriptInstance::set_property(instance_guard, name, value)
        })
        // Unwrapping to a default of false, to indicate that the assignment is not handled by the script.
        .unwrap_or_default();

        transfer_bool_to_godot(result)
    }

    /// # Safety
    ///
    /// - `p_instance` has to be a valid pointer that can be cast to `*mut ScriptInstanceData<T>`.
    /// - `r_ret` will be a valid owned variant pointer after this call.
    pub(super) unsafe extern "C" fn get_property_func<T: ScriptInstance>(
        p_instance: sys::GDExtensionScriptInstanceDataPtr,
        p_name: sys::GDExtensionConstStringNamePtr,
        r_ret: sys::GDExtensionVariantPtr,
    ) -> sys::GDExtensionBool {
        let name = StringName::new_from_string_sys(p_name);
        let ctx = || format!("error when calling {}::get", type_name::<T>());

        let return_value = handle_panic(ctx, || {
            let instance = instance_data_as_script_instance::<T>(p_instance);

            borrow_instance(instance).get_property(name.clone())
        });

        let result = match return_value {
            Ok(return_value) => return_value
                .map(|variant| transfer_variant_to_godot(variant, r_ret))
                .is_some(),
            Err(_) => false,
        };

        transfer_bool_to_godot(result)
    }

    /// # Safety
    ///
    /// - `p_instance` has to be a valid pointer that can be cast to `*mut ScriptInstanceData<T>`.
    /// - `r_count` is expected to be a valid pointer to an u32.
    pub(super) unsafe extern "C" fn get_property_list_func<T: ScriptInstance>(
        p_instance: sys::GDExtensionScriptInstanceDataPtr,
        r_count: *mut u32,
    ) -> *const sys::GDExtensionPropertyInfo {
        let ctx = || format!("error when calling {}::get_property_list", type_name::<T>());

        let (property_list, property_sys_list) = handle_panic(ctx, || {
            let instance = instance_data_as_script_instance::<T>(p_instance);

            let property_list = borrow_instance(instance).get_property_list();

            let property_sys_list: Box<[_]> = property_list
                .iter()
                .map(|prop| prop.property_sys())
                .collect();

            (property_list, property_sys_list)
        })
        .unwrap_or_default();

        // SAFETY: list_length has to be a valid pointer to a u32.
        let list_length = unsafe { &mut *r_count };
        let return_pointer = transfer_ptr_list_to_godot(property_sys_list, list_length);

        let instance = instance_data_as_script_instance::<T>(p_instance);
        instance
            .property_list
            .lock()
            .expect("Mutex should not be poisoned")
            .insert(return_pointer, property_list);

        return_pointer
    }

    /// # Safety
    ///
    /// - `p_instance` has to be a valid pointer that can be cast to `*mut ScriptInstanceData<T>`.
    /// - `r_count` is expected to be a valid pointer to an u32.
    pub(super) unsafe extern "C" fn get_method_list_func<T: ScriptInstance>(
        p_instance: sys::GDExtensionScriptInstanceDataPtr,
        r_count: *mut u32,
    ) -> *const sys::GDExtensionMethodInfo {
        let ctx = || format!("error when calling {}::get_method_list", type_name::<T>());

        let (method_list, method_sys_list) = handle_panic(ctx, || {
            let instance = instance_data_as_script_instance::<T>(p_instance);

            let method_list = borrow_instance(instance).get_method_list();

            let method_sys_list = method_list
                .iter()
                .map(|method| method.method_sys())
                .collect();

            (method_list, method_sys_list)
        })
        .unwrap_or_default();

        // SAFETY: list_length has to be a valid pointer to a u32.
        let list_length = unsafe { &mut *r_count };
        let return_pointer = transfer_ptr_list_to_godot(method_sys_list, list_length);

        let instance = instance_data_as_script_instance::<T>(p_instance);

        instance
            .method_list
            .lock()
            .expect("mutex should not be poisoned")
            .insert(return_pointer, method_list);

        return_pointer
    }

    /// # Safety
    ///
    /// - `p_instance` has to be a valid pointer that can be cast to `*mut ScriptInstanceData<T>`.
    /// - The length of `p_prop_info` list is expected to not have changed since it was transferred to the engine.
    pub(super) unsafe extern "C" fn free_property_list_func<T: ScriptInstance>(
        p_instance: sys::GDExtensionScriptInstanceDataPtr,
        p_prop_info: *const sys::GDExtensionPropertyInfo,
    ) {
        let instance = instance_data_as_script_instance::<T>(p_instance);

        let vec = instance
            .property_list
            .lock()
            .expect("mutex should not be poisoned")
            .remove(&p_prop_info)
            .expect("Godot is trying to free the property list, but none has been set");

        // SAFETY: p_prop_info is expected to have been created by get_property_list_func
        // and therefore should have the same length as before. get_propery_list_func
        // also guarantees that both vector length and capacity are equal.
        let _drop = transfer_ptr_list_from_godot(p_prop_info, vec.len());
    }

    /// # Safety
    ///
    /// - `p_instance` has to be a valid pointer that can be cast to `*mut ScriptInstanceData<T>`.
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
        let method = StringName::new_from_string_sys(p_method);
        let args = Variant::borrow_ref_slice(p_args, p_argument_count as usize);
        let ctx = || format!("error when calling {}::call", type_name::<T>());

        let result = handle_panic(ctx, || {
            let instance = instance_data_as_script_instance::<T>(p_self);
            let cell = borrow_instance_cell(instance);
            let mut guard = cell.borrow_mut().unwrap_or_else(borrow_panic::<T, _>);

            let instance_guard = SiMut::new(cell, &mut guard, &instance.base);

            ScriptInstance::call(instance_guard, method.clone(), args)
        });

        match result {
            Ok(Ok(ret)) => {
                transfer_variant_to_godot(ret, r_return);
                (*r_error).error = sys::GDEXTENSION_CALL_OK;
            }

            Ok(Err(err)) => (*r_error).error = err,

            Err(_) => {
                (*r_error).error = sys::GDEXTENSION_CALL_ERROR_INVALID_METHOD;
            }
        };
    }

    /// # Safety
    ///
    /// - `p_instance` has to be a valid pointer that can be cast to `*mut ScriptInstanceData<T>`.
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
            Ok(script) => transfer_script_to_godot(&script),
            Err(_) => std::ptr::null_mut(),
        }
    }

    /// # Safety
    ///
    /// - `p_instance` has to be a valid pointer that can be cast to `*mut ScriptInstanceData<T>`.
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
    /// - `p_instance` has to be a valid pointer that can be cast to `*mut ScriptInstanceData<T>`.
    /// - `p_method` has to point to a valid string name.
    pub(super) unsafe extern "C" fn has_method_func<T: ScriptInstance>(
        p_instance: sys::GDExtensionScriptInstanceDataPtr,
        p_method: sys::GDExtensionConstStringNamePtr,
    ) -> sys::GDExtensionBool {
        let method = StringName::new_from_string_sys(p_method);
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
    /// - `p_instance` has to be a valid pointer that can be cast to `*mut ScriptInstanceData<T>`.
    /// - The length of `p_method_info` list is expected to not have changed since it was transferred to the engine.
    pub(super) unsafe extern "C" fn free_method_list_func<T: ScriptInstance>(
        p_instance: sys::GDExtensionScriptInstanceDataPtr,
        p_method_info: *const sys::GDExtensionMethodInfo,
    ) {
        let instance = instance_data_as_script_instance::<T>(p_instance);

        let vec = instance
            .method_list
            .lock()
            .expect("method_list mutex should not be poisoned")
            .remove(&p_method_info)
            .expect("Godot is trying to free the method_list, but none has been set");

        // SAFETY: p_method_info is expected to have been created by get_method_list_func and therefore should have the same length as before.
        // get_method_list_func also guarantees that both vector length and capacity are equal.
        let vec_sys = transfer_ptr_list_from_godot(p_method_info, vec.len());

        vec_sys.into_iter().for_each(|method_info| {
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
    /// - `p_instance` has to be a valid pointer that can be cast to `*mut ScriptInstanceData<T>`.
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
        let name = StringName::new_from_string_sys(p_name);

        let result = handle_panic(ctx, || {
            let instance = instance_data_as_script_instance::<T>(p_instance);

            borrow_instance(instance).get_property_type(name.clone())
        });

        if let Ok(result) = result {
            *r_is_valid = transfer_bool_to_godot(true);
            result.sys()
        } else {
            *r_is_valid = transfer_bool_to_godot(false);
            0
        }
    }

    /// # Safety
    ///
    /// - `p_instance` has to be a valid pointer that can be cast to `*mut ScriptInstanceData<T>`.
    /// - `r_is_valid` is expected to be a valid `GDExtensionBool` ptr which can be modified to reflect the validity of the return value.
    /// - `r_str` is expected to be a string pointer into which the return value can be moved.
    pub(super) unsafe extern "C" fn to_string_func<T: ScriptInstance>(
        p_instance: sys::GDExtensionScriptInstanceDataPtr,
        r_is_valid: *mut sys::GDExtensionBool,
        r_str: sys::GDExtensionStringPtr,
    ) {
        let ctx = || format!("error when calling {}::to_string", type_name::<T>());

        let string = handle_panic(ctx, || {
            let instance = instance_data_as_script_instance::<T>(p_instance);

            borrow_instance(instance).to_string()
        })
        .ok();

        let Some(string) = string else {
            return;
        };

        *r_is_valid = transfer_bool_to_godot(true);
        transfer_string_to_godot(string, r_str);
    }

    /// # Safety
    ///
    /// - `p_instance` has to be a valid pointer that can be cast to `*mut ScriptInstanceData<T>`.
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
    /// - `p_instance` has to be a valid pointer that can be cast to `*mut ScriptInstanceData<T>`.
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

        if let Ok(language) = language {
            transfer_script_lang_to_godot(language)
        } else {
            std::ptr::null::<c_void>() as sys::GDExtensionScriptLanguagePtr
        }
    }

    /// # Safety
    ///
    /// - `p_instance` has to be a valid pointer that can be cast to `*mut ScriptInstanceData<T>`.
    /// - The instance data will be freed and the pointer won't be valid anymore after this function has been called.
    pub(super) unsafe extern "C" fn free_func<T: ScriptInstance>(
        p_instance: sys::GDExtensionScriptInstanceDataPtr,
    ) {
        drop(Box::from_raw(p_instance as *mut ScriptInstanceData<T>));
    }

    /// # Safety
    ///
    /// - `p_instance` has to be a valid pointer that can be cast to `*mut ScriptInstanceData<T>`.
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
    /// - `p_instance` has to be a valid pointer that can be cast to `*mut ScriptInstanceData<T>`.
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
    /// - `p_instance` has to be a valid pointer that can be cast to `*mut ScriptInstanceData<T>`.
    /// - `p_name` has to be a valid pointer to a `StringName`.
    /// - `r_ret` has to be a valid pointer to which the return value can be moved.
    pub(super) unsafe extern "C" fn get_fallback_func<T: ScriptInstance>(
        p_instance: sys::GDExtensionScriptInstanceDataPtr,
        p_name: sys::GDExtensionConstStringNamePtr,
        r_ret: sys::GDExtensionVariantPtr,
    ) -> sys::GDExtensionBool {
        let name = StringName::new_from_string_sys(p_name);

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
            Ok(return_value) => return_value
                .map(|value| transfer_variant_to_godot(value, r_ret))
                .is_some(),
            Err(_) => false,
        };

        transfer_bool_to_godot(result)
    }

    /// # Safety
    ///
    /// - `p_instance` has to be a valid pointer that can be cast to `*mut ScriptInstanceData<T>`.
    /// - `p_name` has to be a valid pointer to a `StringName`.
    pub(super) unsafe extern "C" fn set_fallback_func<T: ScriptInstance>(
        p_instance: sys::GDExtensionScriptInstanceDataPtr,
        p_name: sys::GDExtensionConstStringNamePtr,
        p_value: sys::GDExtensionConstVariantPtr,
    ) -> sys::GDExtensionBool {
        let name = StringName::new_from_string_sys(p_name);
        let value = Variant::borrow_var_sys(p_value);

        let ctx = || {
            format!(
                "error when calling {}::property_set_fallback",
                type_name::<T>()
            )
        };

        let result = handle_panic(ctx, || {
            let instance = instance_data_as_script_instance::<T>(p_instance);
            let cell = borrow_instance_cell(instance);
            let mut guard = cell.borrow_mut().unwrap_or_else(borrow_panic::<T, _>);

            let instance_guard = SiMut::new(cell, &mut guard, &instance.base);
            ScriptInstance::property_set_fallback(instance_guard, name, value)
        })
        .unwrap_or_default();

        transfer_bool_to_godot(result)
    }
}
