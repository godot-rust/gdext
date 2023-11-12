/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

#![allow(dead_code)] // FIXME

use crate::init::InitLevel;
use crate::log;
use crate::obj::*;
use crate::private::as_storage;
use crate::storage::InstanceStorage;
use godot_ffi as sys;

use sys::interface_fn;

use crate::builtin::meta::ClassName;
use crate::builtin::StringName;
use crate::out;
use std::any::Any;
use std::collections::HashMap;
use std::sync::{Mutex, MutexGuard, TryLockError};
use std::{fmt, ptr};

// For now, that variable is needed for class unregistering. It's populated during class
// registering. There is no actual concurrency here, because Godot call register/unregister in main
// thread - Mutex is just casual way to ensure safety in this performance non-critical path.
// Note that we panic on concurrent access instead of blocking - that's fail fast approach. If that
// happen, most likely something changed on Godot side and analysis required to adopt these changes.
static LOADED_CLASSES: Mutex<Option<HashMap<InitLevel, Vec<ClassName>>>> = Mutex::new(None);

// TODO(bromeon): some information coming from the proc-macro API is deferred through PluginComponent, while others is directly
// translated to code. Consider moving more code to the PluginComponent, which allows for more dynamic registration and will
// be easier for a future builder API.

/// Piece of information that is gathered by the self-registration ("plugin") system.
#[derive(Debug)]
pub struct ClassPlugin {
    pub class_name: ClassName,
    pub component: PluginComponent,
    pub init_level: Option<InitLevel>,
}

/// Type-erased function object, holding a `register_class` function.
#[derive(Copy, Clone)]
pub struct ErasedRegisterFn {
    // Wrapper needed because Debug can't be derived on function pointers with reference parameters, so this won't work:
    // pub type ErasedRegisterFn = fn(&mut dyn std::any::Any);
    // (see https://stackoverflow.com/q/53380040)
    pub raw: fn(&mut dyn Any),
}

impl fmt::Debug for ErasedRegisterFn {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "0x{:0>16x}", self.raw as usize)
    }
}

/// Represents the data part of a [`ClassPlugin`] instance.
#[derive(Clone, Debug)]
pub enum PluginComponent {
    /// Class definition itself, must always be available
    ClassDef {
        base_class_name: ClassName,

        /// Godot low-level`create` function, wired up to library-generated `init`
        generated_create_fn: Option<
            unsafe extern "C" fn(
                _class_userdata: *mut std::ffi::c_void, //
            ) -> sys::GDExtensionObjectPtr,
        >,

        generated_recreate_fn: Option<
            unsafe extern "C" fn(
                p_class_userdata: *mut std::ffi::c_void,
                p_object: sys::GDExtensionObjectPtr,
            ) -> sys::GDExtensionClassInstancePtr,
        >,

        free_fn: unsafe extern "C" fn(
            _class_user_data: *mut std::ffi::c_void,
            instance: sys::GDExtensionClassInstancePtr,
        ),
    },

    /// Collected from `#[godot_api] impl MyClass`
    UserMethodBinds {
        /// Callback to library-generated function which registers functions in the `impl`
        ///
        /// Always present since that's the entire point of this `impl` block.
        generated_register_fn: ErasedRegisterFn,
    },

    /// Collected from `#[godot_api] impl GodotExt for MyClass`
    UserVirtuals {
        /// Callback to user-defined `register_class` function
        user_register_fn: Option<ErasedRegisterFn>,

        /// Godot low-level`create` function, wired up to the user's `init`
        user_create_fn: Option<
            unsafe extern "C" fn(
                _class_userdata: *mut std::ffi::c_void, //
            ) -> sys::GDExtensionObjectPtr,
        >,

        user_recreate_fn: Option<
            unsafe extern "C" fn(
                p_class_userdata: *mut ::std::os::raw::c_void,
                p_object: sys::GDExtensionObjectPtr,
            ) -> sys::GDExtensionClassInstancePtr,
        >,

        /// User-defined `to_string` function
        user_to_string_fn: Option<
            unsafe extern "C" fn(
                p_instance: sys::GDExtensionClassInstancePtr,
                r_is_valid: *mut sys::GDExtensionBool,
                r_out: sys::GDExtensionStringPtr,
            ),
        >,

        /// User-defined `on_notification` function
        #[cfg(before_api = "4.2")]
        user_on_notification_fn: Option<
            unsafe extern "C" fn(
                p_instance: sys::GDExtensionClassInstancePtr, //
                p_what: i32,
            ),
        >,
        #[cfg(since_api = "4.2")]
        user_on_notification_fn: Option<
            unsafe extern "C" fn(
                p_instance: sys::GDExtensionClassInstancePtr, //
                p_what: i32,
                p_reversed: sys::GDExtensionBool,
            ),
        >,

        /// Callback for other virtuals
        get_virtual_fn: unsafe extern "C" fn(
            p_userdata: *mut std::os::raw::c_void,
            p_name: sys::GDExtensionConstStringNamePtr,
        ) -> sys::GDExtensionClassCallVirtual,
    },

    #[cfg(since_api = "4.1")]
    EditorPlugin,
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

#[derive(Debug)]
struct ClassRegistrationInfo {
    class_name: ClassName,
    parent_class_name: Option<ClassName>,
    generated_register_fn: Option<ErasedRegisterFn>,
    user_register_fn: Option<ErasedRegisterFn>,
    #[cfg(before_api = "4.2")]
    godot_params: sys::GDExtensionClassCreationInfo,
    #[cfg(since_api = "4.2")]
    godot_params: sys::GDExtensionClassCreationInfo2,
    init_level: InitLevel,
    is_editor_plugin: bool,
}

/// Registers a class with static type information.
pub fn register_class<
    T: cap::GodotDefault
        + cap::ImplementsGodotVirtual
        + cap::GodotToString
        + cap::GodotNotification
        + cap::GodotRegisterClass
        + GodotClass,
>() {
    // TODO: provide overloads with only some trait impls

    out!("Manually register class {}", std::any::type_name::<T>());

    #[cfg(before_api = "4.2")]
    let godot_params = sys::GDExtensionClassCreationInfo {
        to_string_func: Some(callbacks::to_string::<T>),
        notification_func: Some(callbacks::on_notification::<T>),
        reference_func: Some(callbacks::reference::<T>),
        unreference_func: Some(callbacks::unreference::<T>),
        create_instance_func: Some(callbacks::create::<T>),
        free_instance_func: Some(callbacks::free::<T>),
        get_virtual_func: Some(callbacks::get_virtual::<T>),
        get_rid_func: None,
        class_userdata: ptr::null_mut(), // will be passed to create fn, but global per class
        ..default_creation_info()
    };
    #[cfg(since_api = "4.2")]
    let godot_params = sys::GDExtensionClassCreationInfo2 {
        to_string_func: Some(callbacks::to_string::<T>),
        notification_func: Some(callbacks::on_notification::<T>),
        reference_func: Some(callbacks::reference::<T>),
        unreference_func: Some(callbacks::unreference::<T>),
        create_instance_func: Some(callbacks::create::<T>),
        recreate_instance_func: Some(callbacks::recreate::<T>),
        free_instance_func: Some(callbacks::free::<T>),
        get_virtual_func: Some(callbacks::get_virtual::<T>),
        get_rid_func: None,
        class_userdata: ptr::null_mut(), // will be passed to create fn, but global per class
        ..default_creation_info()
    };

    register_class_raw(ClassRegistrationInfo {
        class_name: T::class_name(),
        parent_class_name: Some(T::Base::class_name()),
        generated_register_fn: None,
        user_register_fn: Some(ErasedRegisterFn {
            raw: callbacks::register_class_by_builder::<T>,
        }),
        godot_params,
        init_level: T::INIT_LEVEL.unwrap_or_else(|| {
            panic!("Unknown initialization level for class {}", T::class_name())
        }),
        is_editor_plugin: false,
    });
}

/// Lets Godot know about all classes that have self-registered through the plugin system.
pub fn auto_register_classes(init_level: InitLevel) {
    out!("Auto-register classes at level `{init_level:?}`...");

    // Note: many errors are already caught by the compiler, before this runtime validation even takes place:
    // * missing #[derive(GodotClass)] or impl GodotClass for T
    // * duplicate impl GodotDefault for T
    //
    let mut map = HashMap::<ClassName, ClassRegistrationInfo>::new();

    crate::private::iterate_plugins(|elem: &ClassPlugin| {
        //out!("* Plugin: {elem:#?}");
        match elem.init_level {
            None => {
                log::godot_error!("Unknown initialization level for class {}", elem.class_name);
                return;
            }
            Some(elem_init_level) if elem_init_level != init_level => return,
            _ => (),
        }

        let name = elem.class_name;
        let class_info = map
            .entry(name)
            .or_insert_with(|| default_registration_info(name));

        fill_class_info(elem.component.clone(), class_info);
    });

    let mut loaded_classes_guard = get_loaded_classes_with_mutex();
    let loaded_classes_by_level = loaded_classes_guard.get_or_insert_with(HashMap::default);

    for info in map.into_values() {
        out!(
            "Register class:   {} at level `{init_level:?}`",
            info.class_name
        );
        let class_name = info.class_name;
        loaded_classes_by_level
            .entry(init_level)
            .or_default()
            .push(info.class_name);
        register_class_raw(info);

        out!("Class {} loaded", class_name);
    }

    out!("All classes for level `{init_level:?}` auto-registered.");
}

pub fn unregister_classes(init_level: InitLevel) {
    let mut loaded_classes_guard = get_loaded_classes_with_mutex();
    let loaded_classes_by_level = loaded_classes_guard.get_or_insert_with(HashMap::default);
    let loaded_classes_current_level = loaded_classes_by_level
        .remove(&init_level)
        .unwrap_or_default();
    out!("Unregistering classes of level {init_level:?}...");
    for class_name in loaded_classes_current_level.iter().rev() {
        unregister_class_raw(class_name);
    }
}

fn get_loaded_classes_with_mutex() -> MutexGuard<'static, Option<HashMap<InitLevel, Vec<ClassName>>>>
{
    match LOADED_CLASSES.try_lock() {
        Ok(it) => it,
        Err(err) => match err {
            TryLockError::Poisoned(_err) => panic!(
                "LOADED_CLASSES poisoned. seems like class registration or deregistration panicked."
            ),
            TryLockError::WouldBlock => panic!("unexpected concurrent access detected to CLASSES"),
        },
    }
}

/// Populate `c` with all the relevant data from `component` (depending on component type).
fn fill_class_info(component: PluginComponent, c: &mut ClassRegistrationInfo) {
    // out!("|   reg (before):    {c:?}");
    // out!("|   comp:            {component:?}");
    match component {
        PluginComponent::ClassDef {
            base_class_name,
            generated_create_fn,
            generated_recreate_fn,
            free_fn,
        } => {
            c.parent_class_name = Some(base_class_name);

            fill_into(
                &mut c.godot_params.create_instance_func,
                generated_create_fn,
            )
            .unwrap_or_else(|_|
                panic!(
                    "Godot class `{}` is defined multiple times in Rust; you can rename them with #[class(rename=NewName)]",
                    c.class_name,
                )
            );

            #[cfg(since_api = "4.2")]
            fill_into(
                &mut c.godot_params.recreate_instance_func,
                generated_recreate_fn,
            )
            .unwrap_or_else(|_|
                panic!(
                    "Godot class `{}` is defined multiple times in Rust; you can rename them with #[class(rename=NewName)]",
                    c.class_name,
                )
            );

            #[cfg(before_api = "4.2")]
            assert!(generated_recreate_fn.is_none()); // not used

            c.godot_params.free_instance_func = Some(free_fn);
        }

        PluginComponent::UserMethodBinds {
            generated_register_fn,
        } => {
            c.generated_register_fn = Some(generated_register_fn);
        }

        PluginComponent::UserVirtuals {
            user_register_fn,
            user_create_fn,
            user_recreate_fn,
            user_to_string_fn,
            user_on_notification_fn,
            get_virtual_fn,
        } => {
            c.user_register_fn = user_register_fn;

            // The following unwraps of fill_into() shouldn't panic, since rustc will error if there are
            // multiple `impl I{Class} for Thing` definitions.

            fill_into(&mut c.godot_params.create_instance_func, user_create_fn).unwrap();

            #[cfg(since_api = "4.2")]
            fill_into(&mut c.godot_params.recreate_instance_func, user_recreate_fn).unwrap();

            #[cfg(before_api = "4.2")]
            assert!(user_recreate_fn.is_none()); // not used

            c.godot_params.to_string_func = user_to_string_fn;
            c.godot_params.notification_func = user_on_notification_fn;
            c.godot_params.get_virtual_func = Some(get_virtual_fn);
        }
        #[cfg(since_api = "4.1")]
        PluginComponent::EditorPlugin => {
            c.is_editor_plugin = true;
        }
    }
    // out!("|   reg (after):     {c:?}");
    // out!();
}

/// If `src` is occupied, it moves the value into `dst`, while ensuring that no previous value is present in `dst`.
fn fill_into<T>(dst: &mut Option<T>, src: Option<T>) -> Result<(), ()> {
    match (dst, src) {
        (dst @ None, src) => *dst = src,
        (Some(_), Some(_)) => return Err(()),
        (Some(_), None) => { /* do nothing */ }
    }
    Ok(())
}

/// Registers a class with given the dynamic type information `info`.
fn register_class_raw(info: ClassRegistrationInfo) {
    // First register class...

    let class_name = info.class_name;
    let parent_class_name = info
        .parent_class_name
        .expect("class defined (parent_class_name)");

    unsafe {
        // Try to register class...

        #[cfg(before_api = "4.2")]
        #[allow(clippy::let_unit_value)]
        // notifies us if Godot API ever adds a return type.
        let _: () = interface_fn!(classdb_register_extension_class)(
            sys::get_library(),
            class_name.string_sys(),
            parent_class_name.string_sys(),
            ptr::addr_of!(info.godot_params),
        );

        #[cfg(since_api = "4.2")]
        #[allow(clippy::let_unit_value)]
        // notifies us if Godot API ever adds a return type.
        let _: () = interface_fn!(classdb_register_extension_class2)(
            sys::get_library(),
            class_name.string_sys(),
            parent_class_name.string_sys(),
            ptr::addr_of!(info.godot_params),
        );

        // ...then see if it worked.
        // This is necessary because the above registration does not report errors (apart from console output).
        let tag = interface_fn!(classdb_get_class_tag)(class_name.string_sys());
        assert!(
            !tag.is_null(),
            "failed to register class `{class_name}`; check preceding Godot stderr messages",
        );
    }

    // ...then custom symbols

    //let mut class_builder = crate::builder::ClassBuilder::<?>::new();
    let mut class_builder = 0; // TODO dummy argument; see callbacks

    // First call generated (proc-macro) registration function, then user-defined one.
    // This mimics the intuition that proc-macros are running "before" normal runtime code.
    if let Some(register_fn) = info.generated_register_fn {
        (register_fn.raw)(&mut class_builder);
    }
    if let Some(register_fn) = info.user_register_fn {
        (register_fn.raw)(&mut class_builder);
    }

    #[cfg(since_api = "4.1")]
    if info.is_editor_plugin {
        unsafe { interface_fn!(editor_add_plugin)(class_name.string_sys()) };
    }
    #[cfg(before_api = "4.1")]
    assert!(!info.is_editor_plugin);
}

fn unregister_class_raw(class_name: &ClassName) {
    out!("Unregister class: {class_name}");
    unsafe {
        #[allow(clippy::let_unit_value)]
        let _: () = interface_fn!(classdb_unregister_extension_class)(
            sys::get_library(),
            class_name.string_sys(),
        );
    }
    out!("Class {class_name} unloaded");
}

/// Callbacks that are passed as function pointers to Godot upon class registration.
///
/// Re-exported to `crate::private`
#[allow(clippy::missing_safety_doc)]
pub mod callbacks {
    use super::*;
    use crate::builder::ClassBuilder;
    use crate::obj::Base;

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

        let base_ptr =
            unsafe { interface_fn!(classdb_construct_object)(base_class_name.string_sys()) };

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

    pub unsafe extern "C" fn reference<T: GodotClass>(instance: sys::GDExtensionClassInstancePtr) {
        let storage = as_storage::<T>(instance);
        storage.on_inc_ref();
    }

    pub unsafe extern "C" fn unreference<T: GodotClass>(
        instance: sys::GDExtensionClassInstancePtr,
    ) {
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

    pub fn register_user_binds<T: cap::ImplementsGodotApi + cap::ImplementsGodotExports>(
        _class_builder: &mut dyn Any,
    ) {
        // let class_builder = class_builder
        //     .downcast_mut::<ClassBuilder<T>>()
        //     .expect("bad type erasure");

        //T::register_methods(class_builder);
        T::__register_methods();
        T::__register_constants();
        T::__register_exports();
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Substitutes for Default impl

// Yes, bindgen can implement Default, but only for _all_ types (with single exceptions).
// For FFI types, it's better to have explicit initialization in the general case though.
fn default_registration_info(class_name: ClassName) -> ClassRegistrationInfo {
    ClassRegistrationInfo {
        class_name,
        parent_class_name: None,
        generated_register_fn: None,
        user_register_fn: None,
        godot_params: default_creation_info(),
        init_level: InitLevel::Scene,
        is_editor_plugin: false,
    }
}

#[cfg(before_api = "4.2")]
fn default_creation_info() -> sys::GDExtensionClassCreationInfo {
    sys::GDExtensionClassCreationInfo {
        is_abstract: false as u8,
        is_virtual: false as u8,
        set_func: None,
        get_func: None,
        get_property_list_func: None,
        free_property_list_func: None,
        property_can_revert_func: None,
        property_get_revert_func: None,
        notification_func: None,
        to_string_func: None,
        reference_func: None,
        unreference_func: None,
        create_instance_func: None,
        free_instance_func: None,
        get_virtual_func: None,
        get_rid_func: None,
        class_userdata: ptr::null_mut(),
    }
}

#[cfg(since_api = "4.2")]
fn default_creation_info() -> sys::GDExtensionClassCreationInfo2 {
    sys::GDExtensionClassCreationInfo2 {
        is_abstract: false as u8,
        is_virtual: false as u8,
        set_func: None,
        get_func: None,
        get_property_list_func: None,
        free_property_list_func: None,
        property_can_revert_func: None,
        property_get_revert_func: None,
        validate_property_func: None,
        notification_func: None,
        to_string_func: None,
        reference_func: None,
        unreference_func: None,
        create_instance_func: None,
        free_instance_func: None,
        recreate_instance_func: None,
        get_virtual_func: None,
        get_virtual_call_data_func: None,
        call_virtual_with_data_func: None,
        get_rid_func: None,
        class_userdata: ptr::null_mut(),
        is_exposed: true as u8,
    }
}
