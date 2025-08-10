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

use std::ffi::c_void;
use std::ops::{Deref, DerefMut};

#[cfg(feature = "experimental-threads")]
use godot_cell::blocking::{GdCell, MutGuard, RefGuard};
#[cfg(not(feature = "experimental-threads"))]
use godot_cell::panicking::{GdCell, MutGuard, RefGuard};

use crate::builtin::{GString, StringName, Variant, VariantType};
use crate::classes::{Object, Script, ScriptLanguage};
use crate::meta::{MethodInfo, PropertyInfo};
use crate::obj::{Base, Gd, GodotClass};
use crate::sys;

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Public re-exports.

// Godot 4.2+.
#[cfg(since_api = "4.2")]
mod reexport_4_2 {
    pub use crate::classes::IScriptExtension;
    pub use crate::obj::Inherits;
}
#[cfg(since_api = "4.2")]
pub use reexport_4_2::*;

// Re-export guards.
pub use crate::obj::guards::{ScriptBaseMut, ScriptBaseRef};

/// Implement custom scripts that can be attached to objects in Godot.
///
/// To use script instances, implement this trait for your own type.
///
/// You can use the [`create_script_instance()`] function to create a low-level pointer to your script instance. This pointer should then be
/// returned from [`IScriptExtension::instance_create_rawptr()`](crate::classes::IScriptExtension::instance_create_rawptr).
///
/// # Example
///
/// ```no_run
/// # // Trick 17 to avoid listing all the methods. Needs also a method.
/// # mod godot {
/// #     pub use ::godot::*;
/// #     pub mod extras { pub trait ScriptInstance {} pub trait IScriptExtension {} }
/// # }
/// # fn create_script_instance(_: MyInstance) -> *mut std::ffi::c_void { std::ptr::null_mut() }
/// use godot::prelude::*;
/// use godot::classes::{Script, ScriptExtension};
/// use godot::extras::{IScriptExtension, ScriptInstance};
///
/// // 1) Define the script.
/// // This needs #[class(tool)] since the script extension runs in the editor.
/// #[derive(GodotClass)]
/// #[class(init, base=ScriptExtension, tool)]
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
///     // Implement all the methods...
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

    /// Identifies the script instance as a placeholder, routing property writes to a fallback if applicable.
    ///
    /// If this function and [IScriptExtension::is_placeholder_fallback_enabled] return true, Godot will call [`Self::property_set_fallback`]
    /// instead of [`Self::set_property`].
    fn is_placeholder(&self) -> bool;

    /// Validation function for the engine to verify if the script exposes a certain method.
    fn has_method(&self, method: StringName) -> bool;

    /// Lets the engine get a reference to the script this instance was created for.
    ///
    /// This function has to return a reference, because scripts are reference-counted in Godot, and it must be guaranteed that the object is
    /// not freed before the engine increased the reference count. (Every time a ref-counted `Gd<T>` is dropped, the reference count is
    /// decremented.)
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

    /// The engine may call this function if it failed to get a property value via [`ScriptInstance::get_property`] or the native type's getter.
    fn property_get_fallback(&self, name: StringName) -> Option<Variant>;

    /// The engine may call this function if [`IScriptExtension::is_placeholder_fallback_enabled`] is enabled.
    fn property_set_fallback(this: SiMut<Self>, name: StringName, value: &Variant) -> bool;

    /// This function will be called to handle calls to [`Object::get_method_argument_count`](crate::classes::Object::get_method_argument_count)
    /// and `Callable::get_argument_count`.
    ///
    /// If `None` is returned the public methods will return `0`.
    #[cfg(since_api = "4.3")]
    fn get_method_argument_count(&self, _method: StringName) -> Option<u32>;
}

#[cfg(before_api = "4.2")]
type ScriptInstanceInfo = sys::GDExtensionScriptInstanceInfo;
#[cfg(all(since_api = "4.2", before_api = "4.3"))]
type ScriptInstanceInfo = sys::GDExtensionScriptInstanceInfo2;
#[cfg(since_api = "4.3")]
type ScriptInstanceInfo = sys::GDExtensionScriptInstanceInfo3;

struct ScriptInstanceData<T: ScriptInstance> {
    inner: GdCell<T>,
    script_instance_ptr: *mut ScriptInstanceInfo,
    #[cfg(before_api = "4.3")]
    property_lists: BoundedPtrList<sys::GDExtensionPropertyInfo>,
    #[cfg(before_api = "4.3")]
    method_lists: BoundedPtrList<sys::GDExtensionMethodInfo>,
    base: Base<T::Base>,
}

impl<T: ScriptInstance> ScriptInstanceData<T> {
    ///  Convert a `ScriptInstanceData` sys pointer to a reference with unbounded lifetime.
    ///
    /// # Safety
    ///
    /// `ptr` must point to a live `ScriptInstanceData<T>` for the duration of `'a`.
    unsafe fn borrow_script_sys<'a>(ptr: sys::GDExtensionScriptInstanceDataPtr) -> &'a Self {
        &*(ptr.cast::<ScriptInstanceData<T>>())
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
        #[cfg(before_api = "4.3")]
        free_property_list_func: Some(script_instance_info::free_property_list_func::<T>),
        #[cfg(since_api = "4.3")]
        free_property_list_func: Some(script_instance_info::free_property_list_func),

        #[cfg(since_api = "4.2")]
        get_class_category_func: None, // not yet implemented.

        property_can_revert_func: None, // unimplemented until needed.
        property_get_revert_func: None, // unimplemented until needed.

        // ScriptInstance::get_owner() is apparently not called by Godot 4.1 to 4.2 (to verify).
        get_owner_func: None,
        get_property_state_func: Some(script_instance_info::get_property_state_func::<T>),

        get_method_list_func: Some(script_instance_info::get_method_list_func::<T>),
        #[cfg(before_api = "4.3")]
        free_method_list_func: Some(script_instance_info::free_method_list_func::<T>),
        #[cfg(since_api = "4.3")]
        free_method_list_func: Some(script_instance_info::free_method_list_func),
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

        #[cfg(since_api = "4.3")]
        get_method_argument_count_func: Some(
            script_instance_info::get_method_argument_count_func::<T>,
        ),
    };

    let instance_ptr = Box::into_raw(Box::new(gd_instance));

    let data = ScriptInstanceData {
        inner: GdCell::new(rust_instance),
        script_instance_ptr: instance_ptr,
        #[cfg(before_api = "4.3")]
        property_lists: BoundedPtrList::new(),
        #[cfg(before_api = "4.3")]
        method_lists: BoundedPtrList::new(),
        // SAFETY: The script instance is always freed before the base object is destroyed. The weak reference should therefore never be
        // accessed after it has been freed.
        base: unsafe { Base::from_script_gd(&for_object) },
    };

    let data_ptr = Box::into_raw(Box::new(data));

    // SAFETY: `script_instance_create` expects a `GDExtensionScriptInstanceInfoPtr` and a generic `GDExtensionScriptInstanceDataPtr` of our
    // choice. The validity of the instance info struct is ensured by code generation.
    //
    // It is expected that the engine upholds the safety invariants stated on each of the GDEXtensionScriptInstanceInfo functions.
    unsafe {
        #[cfg(before_api = "4.2")]
        let create_fn = sys::interface_fn!(script_instance_create);

        #[cfg(all(since_api = "4.2", before_api = "4.3"))]
        let create_fn = sys::interface_fn!(script_instance_create2);

        #[cfg(since_api = "4.3")]
        let create_fn = sys::interface_fn!(script_instance_create3);

        create_fn(
            instance_ptr,
            data_ptr as sys::GDExtensionScriptInstanceDataPtr,
        ) as *mut c_void
    }
}

/// Checks if an instance of the script exists for a given object.
///
/// This function both checks if the passed script matches the one currently assigned to the passed object, as well as verifies that
/// there is an instance for the script.
///
/// Use this function to implement [`IScriptExtension::instance_has`].
#[cfg(since_api = "4.2")]
pub fn script_instance_exists<O, S>(object: &Gd<O>, script: &Gd<S>) -> bool
where
    O: Inherits<Object>,
    S: Inherits<Script> + IScriptExtension + super::Bounds<Declarer = super::bounds::DeclUser>,
{
    let object_script_variant = object.upcast_ref().get_script();

    if object_script_variant.is_nil() {
        return false;
    }

    if object_script_variant
        .object_id()
        .is_none_or(|instance_id| instance_id != script.instance_id())
    {
        return false;
    }

    let Some(language) = script.bind().get_language() else {
        return false;
    };

    let get_instance_fn = sys::interface_fn!(object_get_script_instance);

    // SAFETY: Object and language are alive and their sys pointers are valid.
    let instance = unsafe { get_instance_fn(object.obj_sys(), language.obj_sys()) };

    !instance.is_null()
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
    /// Holding a shared guard prevents other code paths from obtaining a _mutable_ reference to `self`, as such it is recommended to drop the
    /// guard as soon as you no longer need it.
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
    ///         // this.base().add_child(&node);
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
    ///     # fn get_method_argument_count(&self, _: StringName) -> Option<u32> { todo!() }
    /// }
    /// ```
    pub fn base(&self) -> ScriptBaseRef<'_, T> {
        ScriptBaseRef::new(self.base_ref.to_script_gd(), self.mut_ref)
    }

    /// Returns a mutable reference suitable for calling engine methods on this object.
    ///
    /// This method will allow you to call back into the same object from Godot (re-entrancy).
    /// You have to keep the `ScriptBaseRef` guard bound for the entire duration the engine might re-enter a function of your
    /// `ScriptInstance`. The guard temporarily absorbs the `&mut self` reference, which allows for an additional mutable reference to be
    /// acquired.
    ///
    /// Holding a mutable guard prevents other code paths from obtaining _any_ reference to `self`, as such it is recommended to drop the
    /// guard as soon as you no longer need it.
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
    ///         this.base_mut().call("script_method", &[]);
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
    ///     # fn get_method_argument_count(&self, _: StringName) -> Option<u32> { todo!() }
    /// }
    /// ```
    pub fn base_mut(&mut self) -> ScriptBaseMut<'_, T> {
        let guard = self.cell.make_inaccessible(self.mut_ref).unwrap();

        ScriptBaseMut::new(self.base_ref.to_script_gd(), guard)
    }
}

impl<T: ScriptInstance> Deref for SiMut<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.mut_ref
    }
}

impl<T: ScriptInstance> DerefMut for SiMut<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.mut_ref
    }
}

// Encapsulate BoundedPtrList to help ensure safety.
#[cfg(before_api = "4.3")]
mod bounded_ptr_list {
    use std::collections::HashMap;
    use std::sync::Mutex;

    use godot_ffi as sys;

    /// Helper struct to store the lengths of lists, so they can be properly freed.
    ///
    /// This uses the term `list` because it refers to property/method lists in gdextension.
    pub struct BoundedPtrList<T> {
        list_lengths: Mutex<HashMap<*mut T, u32>>,
    }

    impl<T> BoundedPtrList<T> {
        pub fn new() -> Self {
            Self {
                list_lengths: Mutex::new(HashMap::new()),
            }
        }

        /// Convert a list into a pointer + length pair. Should be used together with [`list_from_sys`](Self::list_from_sys).
        ///
        /// If `list_from_sys` is not called on this list then that will cause a memory leak.
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

            (ptr.cast_const(), len)
        }

        /// Get a list back from a previous call to [`list_into_sys`](Self::list_into_sys).
        ///
        /// # Safety
        /// - `ptr` must have been returned from a call to `list_into_sys` on `self`.
        /// - `ptr` must not have been used in a call to this function before.
        /// - `ptr` must not have been mutated since the call to `list_into_sys`.
        /// - `ptr` must not be accessed after calling this function.
        #[deny(unsafe_op_in_unsafe_fn)]
        pub unsafe fn list_from_sys(&self, ptr: *const T) -> Box<[T]> {
            let ptr: *mut T = ptr.cast_mut();
            let len = self
                .list_lengths
                .lock()
                .unwrap()
                .remove(&ptr)
                .expect("attempted to free list from wrong collection, this is a bug");
            let len: usize = sys::conv::u32_to_usize(len);

            // SAFETY: `ptr` was created in `list_into_sys` from a slice of length `len`.
            // And has not been mutated since.
            let slice = unsafe { std::slice::from_raw_parts_mut(ptr, len) };

            // SAFETY: This is the first call to this function, and the list will not be accessed again after this function call.
            unsafe { Box::from_raw(slice) }
        }
    }
}

#[cfg(before_api = "4.3")]
use self::bounded_ptr_list::BoundedPtrList;

#[deny(unsafe_op_in_unsafe_fn)]
mod script_instance_info {
    use std::any::type_name;
    use std::ffi::c_void;

    use sys::conv::{bool_to_sys, SYS_FALSE, SYS_TRUE};
    #[cfg(since_api = "4.3")]
    use sys::conv::{ptr_list_from_sys, ptr_list_into_sys};

    use super::{ScriptInstance, ScriptInstanceData, SiMut};
    use crate::builtin::{StringName, Variant};
    use crate::meta::{MethodInfo, PropertyInfo};
    use crate::private::handle_panic;
    use crate::sys;

    /// # Safety
    ///
    /// - `p_instance` must point to a live immutable [`ScriptInstanceData<T>`] for the duration of this function call
    /// - `p_name` must be a valid [`StringName`] pointer.
    /// - `p_value` must be a valid [`Variant`] pointer.
    pub(super) unsafe extern "C" fn set_property_func<T: ScriptInstance>(
        p_instance: sys::GDExtensionScriptInstanceDataPtr,
        p_name: sys::GDExtensionConstStringNamePtr,
        p_value: sys::GDExtensionConstVariantPtr,
    ) -> sys::GDExtensionBool {
        let (name, value);
        // SAFETY: `p_name` and `p_value` are valid pointers to a `StringName` and `Variant`.
        unsafe {
            name = StringName::new_from_string_sys(p_name);
            value = Variant::borrow_var_sys(p_value);
        }
        let ctx = || format!("error when calling {}::set", type_name::<T>());

        let result = handle_panic(ctx, || {
            // SAFETY: `p_instance` points to a live immutable `ScriptInstanceData<T>` for the duration of this call.
            let instance = unsafe { ScriptInstanceData::<T>::borrow_script_sys(p_instance) };
            let mut guard = instance.borrow_mut();

            let instance_guard = SiMut::new(instance.cell_ref(), &mut guard, &instance.base);

            ScriptInstance::set_property(instance_guard, name, value)
        })
        // Unwrapping to a default of false, to indicate that the assignment is not handled by the script.
        .unwrap_or_default();

        bool_to_sys(result)
    }

    /// # Safety
    ///
    /// - `p_instance` must point to a live immutable [`ScriptInstanceData<T>`] for the duration of this function call
    /// - `p_name` must be a valid [`StringName`] pointer.
    /// - It must be safe to move a `Variant` into `r_ret`.
    pub(super) unsafe extern "C" fn get_property_func<T: ScriptInstance>(
        p_instance: sys::GDExtensionScriptInstanceDataPtr,
        p_name: sys::GDExtensionConstStringNamePtr,
        r_ret: sys::GDExtensionVariantPtr,
    ) -> sys::GDExtensionBool {
        // SAFETY: `p_name` is a valid [`StringName`] pointer.
        let name = unsafe { StringName::new_from_string_sys(p_name) };
        let ctx = || format!("error when calling {}::get", type_name::<T>());

        let return_value = handle_panic(ctx, || {
            // SAFETY: `p_instance` points to a live immutable `ScriptInstanceData<T>` for the duration of this call.
            unsafe { ScriptInstanceData::<T>::borrow_script_sys(p_instance) }
                .borrow()
                .get_property(name)
        });

        match return_value {
            Ok(Some(variant)) => {
                // SAFETY: It is safe to move a `Variant` into `r_ret`.
                unsafe { variant.move_into_var_ptr(r_ret) };
                SYS_TRUE
            }
            _ => SYS_FALSE,
        }
    }

    /// # Safety
    ///
    /// - `p_instance` must point to a live immutable [`ScriptInstanceData<T>`] for the duration of this function call
    /// - It must be safe to assign a `u32` to `r_count`.
    pub(super) unsafe extern "C" fn get_property_list_func<T: ScriptInstance>(
        p_instance: sys::GDExtensionScriptInstanceDataPtr,
        r_count: *mut u32,
    ) -> *const sys::GDExtensionPropertyInfo {
        let ctx = || format!("error when calling {}::get_property_list", type_name::<T>());

        // Encapsulate this unsafe block to avoid repeating the safety comment.
        // SAFETY: This closure is only used in this function, and we may dereference `p_instance` to an immutable reference for the duration of
        // this call.
        let borrow_instance =
            move || unsafe { ScriptInstanceData::<T>::borrow_script_sys(p_instance) };

        let property_list = handle_panic(ctx, || {
            let property_list = borrow_instance().borrow().get_property_list();

            property_list
                .into_iter()
                .map(|prop| prop.into_owned_property_sys())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

        #[cfg(before_api = "4.3")]
        let (list_ptr, list_length) = borrow_instance()
            .property_lists
            .list_into_sys(property_list);

        #[cfg(since_api = "4.3")]
        let (list_ptr, list_length) = ptr_list_into_sys(property_list);

        // SAFETY: It is safe to assign a `u32` to `r_count`.
        unsafe {
            *r_count = list_length;
        }

        list_ptr
    }

    /// # Safety
    ///
    /// - `p_instance` must point to a live immutable [`ScriptInstanceData<T>`] for the duration of this function call
    /// - `r_count` is expected to be a valid pointer to an u32.
    pub(super) unsafe extern "C" fn get_method_list_func<T: ScriptInstance>(
        p_instance: sys::GDExtensionScriptInstanceDataPtr,
        r_count: *mut u32,
    ) -> *const sys::GDExtensionMethodInfo {
        let ctx = || format!("error when calling {}::get_method_list", type_name::<T>());

        // Encapsulate this unsafe block to avoid repeating the safety comment.
        // SAFETY: This closure is only used in this function, and we may dereference `p_instance` to an immutable reference for the duration of
        // this call.
        let borrow_instance =
            move || unsafe { ScriptInstanceData::<T>::borrow_script_sys(p_instance) };

        let method_list = handle_panic(ctx, || {
            let method_list = borrow_instance().borrow().get_method_list();

            method_list
                .into_iter()
                .map(|method| method.into_owned_method_sys())
                .collect()
        })
        .unwrap_or_default();

        #[cfg(before_api = "4.3")]
        let (return_pointer, list_length) =
            borrow_instance().method_lists.list_into_sys(method_list);
        #[cfg(since_api = "4.3")]
        let (return_pointer, list_length) = ptr_list_into_sys(method_list);

        unsafe {
            *r_count = list_length;
        }

        return_pointer
    }

    /// Provides the same functionality as the function below, but for Godot 4.2 and lower.
    ///
    /// # Safety
    ///
    /// See latest version below.
    #[cfg(before_api = "4.3")]
    pub(super) unsafe extern "C" fn free_property_list_func<T: ScriptInstance>(
        p_instance: sys::GDExtensionScriptInstanceDataPtr,
        p_prop_info: *const sys::GDExtensionPropertyInfo,
    ) {
        // SAFETY: `p_instance` points to a live immutable `ScriptInstanceData<T>` for the duration of this call.
        let instance = unsafe { ScriptInstanceData::<T>::borrow_script_sys(p_instance) };

        // SAFETY: `p_prop_info` was returned from a call to `list_into_sys`, and has not been mutated since. This is also the first call
        // to `list_from_sys` with this pointer.
        let property_infos = unsafe { instance.property_lists.list_from_sys(p_prop_info) };

        for info in property_infos.iter() {
            // SAFETY: `info` was returned from a call to `into_owned_property_sys` and this is the first and only time this function is called
            // on it.
            unsafe { PropertyInfo::free_owned_property_sys(*info) };
        }
    }

    /// # Safety
    /// - `p_instance` must point to a live immutable [`ScriptInstanceData<T>`] for the duration of this function call
    /// - `p_prop_info` must have been returned from a call to [`get_property_list_func`] called with the same `p_instance` pointer.
    /// - `p_prop_info` must not have been mutated since the call to `get_property_list_func`.
    #[cfg(since_api = "4.3")]
    pub(super) unsafe extern "C" fn free_property_list_func(
        _p_instance: sys::GDExtensionScriptInstanceDataPtr,
        p_prop_info: *const sys::GDExtensionPropertyInfo,
        p_len: u32,
    ) {
        // SAFETY: `p_prop_info` was returned from a call to `list_into_sys`, and has not been mutated since. This is also the first call
        // to `list_from_sys` with this pointer.
        let property_infos = unsafe { ptr_list_from_sys(p_prop_info, p_len) };

        for info in property_infos.iter() {
            // SAFETY: `info` was returned from a call to `into_owned_property_sys` and this is the first and only time this function is called
            // on it.
            unsafe { PropertyInfo::free_owned_property_sys(*info) };
        }
    }

    /// # Safety
    ///
    /// - `p_self` must point to a live immutable [`ScriptInstanceData<T>`] for the duration of this function call
    /// - `p_method` must be a valid [`StringName`] pointer.
    /// - `p_args` has to point to a list of Variant pointers of length `p_argument_count`.
    /// - All the variant pointers in `p_args`, as well the `p_args` pointer itself must be dereferenceable to an immutable reference for the
    ///   duration of this call.
    /// - It must be safe to move a [`Variant`] into `r_return`.
    /// - `r_error` must point to an initialized [`sys::GDExtensionCallError`] which can be written to.
    pub(super) unsafe extern "C" fn call_func<T: ScriptInstance>(
        p_self: sys::GDExtensionScriptInstanceDataPtr,
        p_method: sys::GDExtensionConstStringNamePtr,
        p_args: *const sys::GDExtensionConstVariantPtr,
        p_argument_count: sys::GDExtensionInt,
        r_return: sys::GDExtensionVariantPtr,
        r_error: *mut sys::GDExtensionCallError,
    ) {
        // SAFETY: `p_method` is a valid [`StringName`] pointer.
        let method = unsafe { StringName::new_from_string_sys(p_method) };
        // SAFETY: `p_args` is a valid array of length `p_argument_count`
        let args = unsafe {
            Variant::borrow_ref_slice(
                p_args,
                p_argument_count
                    .try_into()
                    .expect("argument count should be a valid `u32`"),
            )
        };
        let ctx = || format!("error when calling {}::call", type_name::<T>());

        let result = handle_panic(ctx, || {
            // SAFETY: `p_self` points to a live immutable `ScriptInstanceData<T>` for the duration of this call.
            let instance = unsafe { ScriptInstanceData::<T>::borrow_script_sys(p_self) };
            let mut guard = instance.borrow_mut();

            let instance_guard = SiMut::new(instance.cell_ref(), &mut guard, &instance.base);

            ScriptInstance::call(instance_guard, method.clone(), args)
        });

        let error = match result {
            Ok(Ok(variant)) => {
                // SAFETY: It is safe to move a `Variant` into `r_return`.
                unsafe { variant.move_into_var_ptr(r_return) };
                sys::GDEXTENSION_CALL_OK
            }

            Ok(Err(err)) => err,

            Err(_) => sys::GDEXTENSION_CALL_ERROR_INVALID_METHOD,
        };

        // SAFETY: `r_error` is an initialized pointer which we can write to.
        unsafe { (*r_error).error = error };
    }

    /// Ownership of the returned object is not transferred to the caller. The caller is therefore responsible for incrementing the reference
    /// count.
    ///
    /// # Safety
    ///
    /// - `p_instance` must point to a live immutable [`ScriptInstanceData<T>`] for the duration of this function call
    pub(super) unsafe extern "C" fn get_script_func<T: ScriptInstance>(
        p_instance: sys::GDExtensionScriptInstanceDataPtr,
    ) -> sys::GDExtensionObjectPtr {
        let ctx = || format!("error when calling {}::get_script", type_name::<T>());

        let script = handle_panic(ctx, || {
            // SAFETY: `p_instance` points to a live immutable `ScriptInstanceData<T>` for the duration of this call.
            unsafe { ScriptInstanceData::<T>::borrow_script_sys(p_instance) }
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
    /// - `p_instance` must point to a live immutable [`ScriptInstanceData<T>`] for the duration of this function call
    pub(super) unsafe extern "C" fn is_placeholder_func<T: ScriptInstance>(
        p_instance: sys::GDExtensionScriptInstanceDataPtr,
    ) -> sys::GDExtensionBool {
        let ctx = || format!("error when calling {}::is_placeholder", type_name::<T>());

        let is_placeholder = handle_panic(ctx, || {
            // SAFETY: `p_instance` points to a live immutable `ScriptInstanceData<T>` for the duration of this call.
            unsafe { ScriptInstanceData::<T>::borrow_script_sys(p_instance) }
                .borrow()
                .is_placeholder()
        })
        .unwrap_or_default();

        bool_to_sys(is_placeholder)
    }

    /// # Safety
    ///
    /// - `p_instance` must point to a live immutable [`ScriptInstanceData<T>`] for the duration of this function call
    /// - `p_method` has to point to a valid `StringName`.
    pub(super) unsafe extern "C" fn has_method_func<T: ScriptInstance>(
        p_instance: sys::GDExtensionScriptInstanceDataPtr,
        p_method: sys::GDExtensionConstStringNamePtr,
    ) -> sys::GDExtensionBool {
        // SAFETY: `p_method` is a valid [`StringName`] pointer.
        let method = unsafe { StringName::new_from_string_sys(p_method) };
        let ctx = || format!("error when calling {}::has_method", type_name::<T>());

        let has_method = handle_panic(ctx, || {
            // SAFETY: `p_instance` points to a live immutable `ScriptInstanceData<T>` for the duration of this call.
            unsafe { ScriptInstanceData::<T>::borrow_script_sys(p_instance) }
                .borrow()
                .has_method(method)
        })
        .unwrap_or_default();

        bool_to_sys(has_method)
    }

    /// Provides the same functionality as the function below, but for Godot 4.2 and lower.
    ///
    /// # Safety
    ///
    /// See latest version below.
    #[cfg(before_api = "4.3")]
    pub(super) unsafe extern "C" fn free_method_list_func<T: ScriptInstance>(
        p_instance: sys::GDExtensionScriptInstanceDataPtr,
        p_method_info: *const sys::GDExtensionMethodInfo,
    ) {
        // SAFETY: `p_instance` points to a live immutable `ScriptInstanceData<T>` for the duration of this call.
        let instance = unsafe { ScriptInstanceData::<T>::borrow_script_sys(p_instance) };

        // SAFETY: `p_method_info` was returned from a call to `list_into_sys`, and has not been mutated since. This is also the first call
        // to `list_from_sys` with this pointer.
        let method_infos = unsafe { instance.method_lists.list_from_sys(p_method_info) };

        for info in method_infos.iter() {
            // SAFETY: `info` was returned from a call to `into_owned_method_sys`, and this is the first and only time we call this method on
            // it.
            unsafe { MethodInfo::free_owned_method_sys(*info) };
        }
    }

    /// # Safety
    ///
    /// - `p_instance` must point to a live immutable [`ScriptInstanceData<T>`] for the duration of this function call
    /// - `p_method_info` must have been returned from a call to [`get_method_list_func`] called with the same `p_instance` pointer.
    /// - `p_method_info` must not have been mutated since the call to `get_method_list_func`.
    #[cfg(since_api = "4.3")]
    pub(super) unsafe extern "C" fn free_method_list_func(
        _p_instance: sys::GDExtensionScriptInstanceDataPtr,
        p_method_info: *const sys::GDExtensionMethodInfo,
        p_len: u32,
    ) {
        // SAFETY: `p_method_info` was returned from a call to `list_into_sys`, and has not been mutated since. This is also the first call
        // to `list_from_sys` with this pointer.
        let method_infos = unsafe { ptr_list_from_sys(p_method_info, p_len) };

        for info in method_infos.iter() {
            // SAFETY: `info` was returned from a call to `into_owned_method_sys`, and this is the first and only time we call this method on
            // it.
            unsafe { MethodInfo::free_owned_method_sys(*info) };
        }
    }

    /// # Safety
    ///
    /// - `p_instance` must point to a live immutable [`ScriptInstanceData<T>`] for the duration of this function call
    /// - `p_name` must be a valid [`StringName`] pointer.
    /// - `r_is_valid` must be assignable.
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
        // SAFETY: `p_name` is a valid [`StringName`] pointer.
        let name = unsafe { StringName::new_from_string_sys(p_name) };

        let result = handle_panic(ctx, || {
            // SAFETY: `p_instance` points to a live immutable `ScriptInstanceData<T>` for the duration of this call.
            unsafe { ScriptInstanceData::<T>::borrow_script_sys(p_instance) }
                .borrow()
                .get_property_type(name.clone())
        });

        let (is_valid, result) = if let Ok(result) = result {
            (SYS_TRUE, result.sys())
        } else {
            (SYS_FALSE, sys::GDEXTENSION_VARIANT_TYPE_NIL)
        };

        // SAFETY: `r_is_valid` is assignable.
        unsafe { *r_is_valid = is_valid };
        result
    }

    /// # Safety
    ///
    /// - `p_instance` must point to a live immutable [`ScriptInstanceData<T>`] for the duration of this function call
    /// - `r_is_valid` must be assignable.
    /// - It must be safe to move a [`GString`](crate::builtin::GString) into `r_str`.
    pub(super) unsafe extern "C" fn to_string_func<T: ScriptInstance>(
        p_instance: sys::GDExtensionScriptInstanceDataPtr,
        r_is_valid: *mut sys::GDExtensionBool,
        r_str: sys::GDExtensionStringPtr,
    ) {
        let ctx = || format!("error when calling {}::to_string", type_name::<T>());

        let string = handle_panic(ctx, || {
            // SAFETY: `p_instance` points to a live immutable `ScriptInstanceData<T>` for the duration of this call.
            unsafe { ScriptInstanceData::<T>::borrow_script_sys(p_instance) }
                .borrow()
                .to_string()
        })
        .ok();

        let Some(string) = string else {
            return;
        };

        // SAFETY: `r_is_valid` is assignable.
        unsafe { *r_is_valid = SYS_TRUE };
        // SAFETY: It is safe to move a `GString` into `r_str`.
        unsafe { string.move_into_string_ptr(r_str) };
    }

    /// # Safety
    ///
    /// - `p_instance` must point to a live immutable [`ScriptInstanceData<T>`] for the duration of this function call
    ///
    /// If `property_state_add` is non-null, then:
    /// - It is safe to call `property_state_add` using the provided `userdata`.
    /// - `property_state_add` must take ownership of the `StringName` and `Variant` it is called with.
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
            // SAFETY: `p_instance` points to a live immutable `ScriptInstanceData<T>` for the duration of this call.
            unsafe { ScriptInstanceData::<T>::borrow_script_sys(p_instance) }
                .borrow()
                .get_property_state()
        })
        .unwrap_or_default();

        let Some(property_state_add) = property_state_add else {
            return;
        };

        for (name, value) in property_states {
            // SAFETY: `property_state_add` is non-null, therefore we can call the function with the provided `userdata`.
            // Additionally `property_state_add` takes ownership of `name` and `value`.
            unsafe {
                property_state_add(
                    name.into_owned_string_sys(),
                    value.into_owned_var_sys(),
                    userdata,
                )
            }
        }
    }

    /// Ownership of the returned object is not transferred to the caller. The caller must therefore ensure it's not freed when used.
    ///
    /// # Safety
    ///
    /// - `p_instance` must point to a live immutable [`ScriptInstanceData<T>`] for the duration of this function call
    pub(super) unsafe extern "C" fn get_language_func<T: ScriptInstance>(
        p_instance: sys::GDExtensionScriptInstanceDataPtr,
    ) -> sys::GDExtensionScriptLanguagePtr {
        let ctx = || format!("error when calling {}::get_language", type_name::<T>());

        let language = handle_panic(ctx, || {
            // SAFETY: `p_instance` points to a live immutable `ScriptInstanceData<T>` for the duration of this call.
            unsafe { ScriptInstanceData::<T>::borrow_script_sys(p_instance) }
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
    /// - `p_instance` must fulfill the safety preconditions of [`Box::from_raw`] for `Box<ScriptInstanceData<T>>`.
    pub(super) unsafe extern "C" fn free_func<T: ScriptInstance>(
        p_instance: sys::GDExtensionScriptInstanceDataPtr,
    ) {
        unsafe { drop(Box::from_raw(p_instance.cast::<ScriptInstanceData<T>>())) }
    }

    /// # Safety
    ///
    /// - `p_instance` must point to a live immutable [`ScriptInstanceData<T>`] for the duration of this function call
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
            // SAFETY: `p_instance` points to a live immutable `ScriptInstanceData<T>` for the duration of this call.
            unsafe { ScriptInstanceData::<T>::borrow_script_sys(p_instance) }
                .borrow()
                .on_refcount_decremented()
        })
        .unwrap_or(true);

        bool_to_sys(result)
    }

    /// # Safety
    ///
    /// - `p_instance` must point to a live immutable [`ScriptInstanceData<T>`] for the duration of this function call
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
            // SAFETY: `p_instance` points to a live immutable `ScriptInstanceData<T>` for the duration of this call.
            unsafe { ScriptInstanceData::<T>::borrow_script_sys(p_instance) }
                .borrow()
                .on_refcount_incremented();
        })
        .unwrap_or_default();
    }

    /// # Safety
    ///
    /// - `p_instance` must point to a live immutable [`ScriptInstanceData<T>`] for the duration of this function call
    /// - `p_name` must be a valid [`StringName`] pointer.
    /// - It must be safe to move a `Variant` into `r_ret`.
    pub(super) unsafe extern "C" fn get_fallback_func<T: ScriptInstance>(
        p_instance: sys::GDExtensionScriptInstanceDataPtr,
        p_name: sys::GDExtensionConstStringNamePtr,
        r_ret: sys::GDExtensionVariantPtr,
    ) -> sys::GDExtensionBool {
        // SAFETY: `p_name` is a valid `StringName` pointer.
        let name = unsafe { StringName::new_from_string_sys(p_name) };

        let ctx = || {
            format!(
                "error when calling {}::property_get_fallback",
                type_name::<T>()
            )
        };

        let return_value = handle_panic(ctx, || {
            // SAFETY: `p_instance` points to a live immutable `ScriptInstanceData<T>` for the duration of this call.
            unsafe { ScriptInstanceData::<T>::borrow_script_sys(p_instance) }
                .borrow()
                .property_get_fallback(name)
        });

        match return_value {
            Ok(Some(variant)) => {
                // SAFETY: It is safe to move a `Variant` into `r_ret`.
                unsafe { variant.move_into_var_ptr(r_ret) };
                SYS_TRUE
            }
            _ => SYS_FALSE,
        }
    }

    /// # Safety
    ///
    /// - `p_instance` must point to a live immutable [`ScriptInstanceData<T>`] for the duration of this function call
    /// - `p_name` must be a valid [`StringName`] pointer.
    /// - `p_value` must be a valid [`Variant`] pointer.
    pub(super) unsafe extern "C" fn set_fallback_func<T: ScriptInstance>(
        p_instance: sys::GDExtensionScriptInstanceDataPtr,
        p_name: sys::GDExtensionConstStringNamePtr,
        p_value: sys::GDExtensionConstVariantPtr,
    ) -> sys::GDExtensionBool {
        let (name, value);
        // SAFETY: `p_name` and `p_value` are valid `StringName` and `Variant` pointers respectively.
        unsafe {
            name = StringName::new_from_string_sys(p_name);
            value = Variant::borrow_var_sys(p_value);
        };

        let ctx = || {
            format!(
                "error when calling {}::property_set_fallback",
                type_name::<T>()
            )
        };

        let result = handle_panic(ctx, || {
            // SAFETY: `p_instance` points to a live immutable `ScriptInstanceData<T>` for the duration of this call.
            let instance = unsafe { ScriptInstanceData::<T>::borrow_script_sys(p_instance) };
            let mut guard = instance.borrow_mut();

            let instance_guard = SiMut::new(instance.cell_ref(), &mut guard, &instance.base);
            ScriptInstance::property_set_fallback(instance_guard, name, value)
        })
        .unwrap_or_default();

        bool_to_sys(result)
    }

    /// # Safety
    ///
    /// - `p_instance` must point to a live immutable [`ScriptInstanceData<T>`] for the duration of this function call
    /// - `p_method` has to point to a valid [`StringName`].
    /// - `p_value` must be a valid [`sys::GDExtensionBool`] pointer.
    #[cfg(since_api = "4.3")]
    pub(super) unsafe extern "C" fn get_method_argument_count_func<T: ScriptInstance>(
        p_instance: sys::GDExtensionScriptInstanceDataPtr,
        p_method: sys::GDExtensionConstStringNamePtr,
        r_is_valid: *mut sys::GDExtensionBool,
    ) -> sys::GDExtensionInt {
        // SAFETY: `p_method` is a valid [`StringName`] pointer.
        let method = unsafe { StringName::new_from_string_sys(p_method) };
        let ctx = || {
            format!(
                "error when calling {}::get_method_argument_count_func",
                type_name::<T>()
            )
        };

        let method_argument_count = handle_panic(ctx, || {
            // SAFETY: `p_instance` points to a live immutable `ScriptInstanceData<T>` for the duration of this call.
            unsafe { ScriptInstanceData::<T>::borrow_script_sys(p_instance) }
                // Can panic if the GdCell is currently mutably bound.
                .borrow()
                // This is user code and could cause a panic.
                .get_method_argument_count(method)
        })
        // In case of a panic, handle_panic will print an error message. We will recover from the panic by falling back to the default value None.
        .unwrap_or_default();

        let (result, is_valid) = match method_argument_count {
            Some(count) => (count, SYS_TRUE),
            None => (0, SYS_FALSE),
        };

        // SAFETY: `r_is_valid` is assignable.
        unsafe { *r_is_valid = is_valid };

        result.into()
    }
}
