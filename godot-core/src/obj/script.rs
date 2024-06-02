/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! Functionality related to script instances (Rust code that can be attached as node scripts).
//!
//! The features in this module are complemented by the [`ScriptExtension` class][crate::classes::ScriptExtension] and
//! the [`IScriptExtension` trait][crate::classes::IScriptExtension].
//!
//! See [`ScriptInstance`](trait.ScriptInstance.html) for usage.

// Re-export guards.
pub use crate::obj::guards::{ScriptBaseMut, ScriptBaseRef};

use std::ffi::c_void;
use std::ops::{Deref, DerefMut};

#[cfg(not(feature = "experimental-threads"))]
use godot_cell::panicking::{GdCell, MutGuard, RefGuard};

#[cfg(feature = "experimental-threads")]
use godot_cell::blocking::{GdCell, MutGuard, RefGuard};

use crate::builtin::{GString, StringName, Variant, VariantType};
use crate::classes::{Script, ScriptLanguage};
use crate::meta::{MethodInfo, PropertyInfo};
use crate::obj::{Base, Gd, GodotClass};
use crate::sys;

use self::ptrlist_container::PtrlistContainer;

/// Implement custom scripts that can be attached to objects in Godot.
///
/// To use script instances, implement this trait for your own type.
///
/// You can use the [`create_script_instance()`] function to create a low-level pointer to your script instance.
/// This pointer should then be returned from [`IScriptExtension::instance_create()`](crate::classes::IScriptExtension::instance_create).
///
/// # Example
///
/// ```no_run
/// # // Trick 17 to avoid listing all the methods. Needs also a method.
/// # mod godot {
/// #     pub use ::godot::*;
/// #     pub mod extras { pub trait ScriptInstance {} }
/// # }
/// # fn create_script_instance(_: MyInstance) -> *mut std::ffi::c_void { std::ptr::null_mut() }
/// use godot::prelude::*;
/// use godot::classes::{IScriptExtension, Script, ScriptExtension};
/// use godot::extras::ScriptInstance;
///
/// // 1) Define the script.
/// #[derive(GodotClass)]
/// #[class(init, base=ScriptExtension)]
/// struct MyScript {
///    base: Base<ScriptExtension>,
///    // ... other fields
/// }
///
/// // 2) Define the script _instance_, and implement the trait for it.
/// struct MyInstance;
/// impl MyInstance {
///     fn from_gd(script: Gd<Script>) -> Self {
///         Self { /* ... */ }
///     }
/// }
///
/// impl ScriptInstance for MyInstance {
///     // Implement all the methods...
/// }
///
/// // 3) Implement the script's virtual interface to wire up 1) and 2).
/// #[godot_api]
/// impl IScriptExtension for MyScript {
///     unsafe fn instance_create(&self, _for_object: Gd<Object>) -> *mut std::ffi::c_void {
///         // Upcast Gd<ScriptExtension> to Gd<Script>.
///         let script = self.to_gd().upcast();
///         let script_instance = MyInstance::from_gd(script);
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
    /// [IScriptExtension::is_placeholder_fallback_enabled](crate::classes::IScriptExtension::is_placeholder_fallback_enabled) return true,
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
    inner: GdCell<T>,
    script_instance_ptr: *mut ScriptInstanceInfo,
    property_lists: PtrlistContainer<sys::GDExtensionPropertyInfo>,
    method_lists: PtrlistContainer<sys::GDExtensionMethodInfo>,
    base: Base<T::Base>,
}

impl<T: ScriptInstance> ScriptInstanceData<T> {
    unsafe fn borrow_script_sys<'a>(p_instance: sys::GDExtensionScriptInstanceDataPtr) -> &'a Self {
        &*(p_instance as *mut ScriptInstanceData<T>)
    }

    fn borrow(&self) -> RefGuard<'_, T> {
        self.inner
            .borrow()
            .unwrap_or_else(|err| Self::borrow_panic(err))
    }

    fn borrow_mut(&self) -> MutGuard<'_, T> {
        self.inner
            .borrow_mut()
            .unwrap_or_else(|err| Self::borrow_panic(err))
    }

    fn cell_ref(&self) -> &GdCell<T> {
        &self.inner
    }

    fn borrow_panic(err: Box<dyn std::error::Error>) -> ! {
        panic!(
            "\
                ScriptInstance borrow failed, already bound; T = {}.\n  \
                Make sure to use `SiMut::base_mut()` when possible.\n  \
                Details: {err}.\
            ",
            std::any::type_name::<T>(),
        )
    }
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
        property_lists: PtrlistContainer::new(),
        method_lists: PtrlistContainer::new(),
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
    cell: &'a GdCell<T>,
    base_ref: &'a Base<T::Base>,
}

impl<'a, T: ScriptInstance> SiMut<'a, T> {
    fn new(
        cell: &'a GdCell<T>,
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
    /// # use godot::classes::{ScriptLanguage, Script};
    /// # use godot::obj::script::{ScriptInstance, SiMut};
    /// # use godot::meta::{MethodInfo, PropertyInfo};
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
    ///     # fn set_property(_: SiMut<'_, Self>, _: StringName, _: &Variant) -> bool { todo!() }
    ///     # fn get_property(&self, _: StringName) -> Option<Variant> { todo!() }
    ///     # fn get_property_list(&self) -> Vec<PropertyInfo> { todo!() }
    ///     # fn get_method_list(&self) -> Vec<MethodInfo> { todo!() }
    ///     # fn is_placeholder(&self) -> bool { todo!() }
    ///     # fn has_method(&self, _: StringName) -> bool { todo!() }
    ///     # fn get_script(&self) -> &Gd<Script> { todo!() }
    ///     # fn get_property_type(&self, _: StringName) -> VariantType { todo!() }
    ///     # fn to_string(&self) -> GString { todo!() }
    ///     # fn get_property_state(&self) -> Vec<(StringName, Variant)> { todo!() }
    ///     # fn get_language(&self) -> Gd<ScriptLanguage> { todo!() }
    ///     # fn on_refcount_decremented(&self) -> bool { todo!() }
    ///     # fn on_refcount_incremented(&self) { todo!() }
    ///     # fn property_get_fallback(&self, _: StringName) -> Option<Variant> { todo!() }
    ///     # fn property_set_fallback(_: SiMut<'_, Self>, _: StringName, _: &Variant) -> bool { todo!() }
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
    /// # use godot::classes::{ScriptLanguage, Script};
    /// # use godot::obj::script::{ScriptInstance, SiMut};
    /// # use godot::meta::{MethodInfo, PropertyInfo};
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
    ///     # fn set_property(_: SiMut<'_, Self>, _: StringName, _: &Variant) -> bool { todo!() }
    ///     # fn get_property(&self, _: StringName) -> Option<Variant> { todo!() }
    ///     # fn get_property_list(&self) -> Vec<PropertyInfo> { todo!() }
    ///     # fn get_method_list(&self) -> Vec<MethodInfo> { todo!() }
    ///     # fn is_placeholder(&self) -> bool { todo!() }
    ///     # fn has_method(&self, _: StringName) -> bool { todo!() }
    ///     # fn get_script(&self) -> &Gd<Script> { todo!() }
    ///     # fn get_property_type(&self, _: StringName) -> VariantType { todo!() }
    ///     # fn to_string(&self) -> GString { todo!() }
    ///     # fn get_property_state(&self) -> Vec<(StringName, Variant)> { todo!() }
    ///     # fn get_language(&self) -> Gd<ScriptLanguage> { todo!() }
    ///     # fn on_refcount_decremented(&self) -> bool { todo!() }
    ///     # fn on_refcount_incremented(&self) { todo!() }
    ///     # fn property_get_fallback(&self, _: StringName) -> Option<Variant> { todo!() }
    ///     # fn property_set_fallback(_: SiMut<'_, Self>, _: StringName, _: &Variant) -> bool { todo!() }
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

// Encapsulate PtrlistContainer to help ensure safety.
mod ptrlist_container {
    use std::collections::HashMap;
    use std::sync::Mutex;

    pub struct PtrlistContainer<T> {
        list_lengths: Mutex<HashMap<*mut T, u32>>,
    }

    impl<T> PtrlistContainer<T> {
        pub fn new() -> Self {
            Self {
                list_lengths: Mutex::new(HashMap::new()),
            }
        }

        pub fn list_into_sys(&self, list: Vec<T>) -> (*const T, u32) {
            let len: u32 = list
                .len()
                .try_into()
                .expect("list must have length that fits in u32");
            let ptr = Box::leak(list.into_boxed_slice()).as_mut_ptr();

            let old_value = self.list_lengths.lock().unwrap().insert(ptr, len);
            assert_eq!(
                old_value, None,
                "attempted to insert the same list twice, this is a bug"
            );

            (ptr as *const T, len)
        }

        /// # Safety
        /// - `ptr` must have been returned from a call to `list_into_sys` on `self`.
        /// - `ptr` must not have been used in a call to this function before.
        /// - `ptr` must not have been mutated since the call to `list_into_sys`.
        /// - `ptr` must not be accessed after calling this function.
        #[deny(unsafe_op_in_unsafe_fn)]
        pub unsafe fn list_from_sys(&self, ptr: *const T) -> Box<[T]> {
            let ptr: *mut T = ptr as *mut T;
            let len = self
                .list_lengths
                .lock()
                .unwrap()
                .remove(&ptr)
                .expect("attempted to free list from wrong collection, this is a bug");
            let len: usize = len
                .try_into()
                .expect("gdext only supports targets where u32 <= usize");

            // SAFETY: `ptr` was created in `list_into_sys` from a slice of length `len`.
            // And has not been mutated since.
            let slice = unsafe { std::slice::from_raw_parts_mut(ptr, len) };

            // SAFETY: This is the first call to this function, and the list will not be accessed again after this function call.
            unsafe { Box::from_raw(slice) }
        }
    }
}

mod script_instance_info {
    use std::any::type_name;
    use std::ffi::c_void;

    use crate::builtin::{StringName, Variant};
    use crate::private::handle_panic;
    use crate::sys;

    use super::{ScriptInstance, ScriptInstanceData, SiMut};

    const fn bool_into_sys(value: bool) -> sys::GDExtensionBool {
        value as sys::GDExtensionBool
    }

    const SYS_TRUE: sys::GDExtensionBool = bool_into_sys(true);
    const SYS_FALSE: sys::GDExtensionBool = bool_into_sys(true);

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
            let instance = ScriptInstanceData::<T>::borrow_script_sys(p_instance);
            let mut guard = instance.borrow_mut();

            let instance_guard = SiMut::new(instance.cell_ref(), &mut guard, &instance.base);

            ScriptInstance::set_property(instance_guard, name, value)
        })
        // Unwrapping to a default of false, to indicate that the assignment is not handled by the script.
        .unwrap_or_default();

        bool_into_sys(result)
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
            ScriptInstanceData::<T>::borrow_script_sys(p_instance)
                .borrow()
                .get_property(name)
        });

        match return_value {
            Ok(Some(variant)) => {
                variant.move_into_var_ptr(r_ret);
                SYS_TRUE
            }
            _ => SYS_FALSE,
        }
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

        let property_list = handle_panic(ctx, || {
            let property_list = ScriptInstanceData::<T>::borrow_script_sys(p_instance)
                .borrow()
                .get_property_list();

            property_list
                .into_iter()
                .map(|prop| prop.into_owned_property_sys())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

        let (list_ptr, list_length) = ScriptInstanceData::<T>::borrow_script_sys(p_instance)
            .property_lists
            .list_into_sys(property_list);

        unsafe {
            *r_count = list_length;
        }

        list_ptr
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

        let method_list = handle_panic(ctx, || {
            let method_list = ScriptInstanceData::<T>::borrow_script_sys(p_instance)
                .borrow()
                .get_method_list();

            let method_list = method_list
                .into_iter()
                .map(|method| method.into_owned_method_sys())
                .collect();

            method_list
        })
        .unwrap_or_default();

        let instance = ScriptInstanceData::<T>::borrow_script_sys(p_instance);

        let (return_pointer, list_length) = instance.method_lists.list_into_sys(method_list);

        unsafe {
            *r_count = list_length;
        }

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
        let instance = ScriptInstanceData::<T>::borrow_script_sys(p_instance);

        let _drop = instance.property_lists.list_from_sys(p_prop_info);
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
            let instance = ScriptInstanceData::<T>::borrow_script_sys(p_self);
            let mut guard = instance.borrow_mut();

            let instance_guard = SiMut::new(instance.cell_ref(), &mut guard, &instance.base);

            ScriptInstance::call(instance_guard, method.clone(), args)
        });

        match result {
            Ok(Ok(variant)) => {
                variant.move_into_var_ptr(r_return);
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
            ScriptInstanceData::<T>::borrow_script_sys(p_instance)
                .borrow()
                .get_script()
                .clone()
        });

        match script {
            Ok(script) => script.obj_sys(),
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
            ScriptInstanceData::<T>::borrow_script_sys(p_instance)
                .borrow()
                .is_placeholder()
        })
        .unwrap_or_default();

        bool_into_sys(is_placeholder)
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
            ScriptInstanceData::<T>::borrow_script_sys(p_instance)
                .borrow()
                .has_method(method)
        })
        .unwrap_or_default();

        bool_into_sys(has_method)
    }

    /// # Safety
    ///
    /// - `p_instance` has to be a valid pointer that can be cast to `*mut ScriptInstanceData<T>`.
    /// - The length of `p_method_info` list is expected to not have changed since it was transferred to the engine.
    pub(super) unsafe extern "C" fn free_method_list_func<T: ScriptInstance>(
        p_instance: sys::GDExtensionScriptInstanceDataPtr,
        p_method_info: *const sys::GDExtensionMethodInfo,
    ) {
        let instance = ScriptInstanceData::<T>::borrow_script_sys(p_instance);

        let _drop = instance.method_lists.list_from_sys(p_method_info);
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
            ScriptInstanceData::<T>::borrow_script_sys(p_instance)
                .borrow()
                .get_property_type(name.clone())
        });

        if let Ok(result) = result {
            *r_is_valid = SYS_TRUE;
            result.sys()
        } else {
            *r_is_valid = SYS_FALSE;
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
            ScriptInstanceData::<T>::borrow_script_sys(p_instance)
                .borrow()
                .to_string()
        })
        .ok();

        let Some(string) = string else {
            return;
        };

        *r_is_valid = SYS_TRUE;
        string.move_into_string_ptr(r_str);
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
            ScriptInstanceData::<T>::borrow_script_sys(p_instance)
                .borrow()
                .get_property_state()
        })
        .unwrap_or_default();

        let Some(property_state_add) = property_state_add else {
            return;
        };

        for (name, value) in property_states {
            property_state_add(
                name.into_owned_string_sys(),
                value.into_owned_var_sys(),
                userdata,
            );
        }
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
            ScriptInstanceData::<T>::borrow_script_sys(p_instance)
                .borrow()
                .get_language()
        });

        if let Ok(language) = language {
            language.obj_sys().cast()
        } else {
            std::ptr::null_mut()
        }
    }

    /// # Safety
    ///
    /// - `p_instance` has to be a valid pointer that can be cast to `*mut ScriptInstanceData<T>`.
    /// - The instance data will be freed and the pointer won't be valid anymore after this function has been called.
    pub(super) unsafe extern "C" fn free_func<T: ScriptInstance>(
        p_instance: sys::GDExtensionScriptInstanceDataPtr,
    ) {
        drop(Box::from_raw(p_instance.cast::<ScriptInstanceData<T>>()));
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
            ScriptInstanceData::<T>::borrow_script_sys(p_instance)
                .borrow()
                .on_refcount_decremented()
        })
        .unwrap_or(true);

        bool_into_sys(result)
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
            ScriptInstanceData::<T>::borrow_script_sys(p_instance)
                .borrow()
                .on_refcount_incremented();
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
            ScriptInstanceData::<T>::borrow_script_sys(p_instance)
                .borrow()
                .property_get_fallback(name)
        });

        match return_value {
            Ok(Some(variant)) => {
                variant.move_into_var_ptr(r_ret);
                SYS_TRUE
            }
            _ => SYS_FALSE,
        }
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
            let instance = ScriptInstanceData::<T>::borrow_script_sys(p_instance);
            let mut guard = instance.borrow_mut();

            let instance_guard = SiMut::new(instance.cell_ref(), &mut guard, &instance.base);
            ScriptInstance::property_set_fallback(instance_guard, name, value)
        })
        .unwrap_or_default();

        bool_into_sys(result)
    }
}
