/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::collections::HashMap;
use std::ptr;

use crate::init::InitLevel;
use crate::meta::ClassName;
use crate::obj::{cap, GodotClass};
use crate::private::{ClassPlugin, PluginItem};
use crate::registry::callbacks;
use crate::registry::plugin::ErasedRegisterFn;
use crate::{godot_error, sys};
use sys::{interface_fn, out, Global, GlobalGuard, GlobalLockError};

// Needed for class unregistering. The variable is populated during class registering. There is no actual concurrency here, because Godot
// calls register/unregister in the main thread. Mutex is just casual way to ensure safety in this non-performance-critical path.
// Note that we panic on concurrent access instead of blocking (fail-fast approach). If that happens, most likely something changed on Godot
// side and analysis required to adopt these changes.
static LOADED_CLASSES: Global<
    HashMap<InitLevel, Vec<LoadedClass>>, //.
> = Global::default();

// ----------------------------------------------------------------------------------------------------------------------------------------------

/// Represents a class who is currently loaded and retained in memory.
///
/// Besides the name, this type holds information relevant for the deregistration of the class.
pub struct LoadedClass {
    name: ClassName,
    #[cfg_attr(before_api = "4.1", allow(dead_code))]
    is_editor_plugin: bool,
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

#[derive(Debug)]
struct ClassRegistrationInfo {
    class_name: ClassName,
    parent_class_name: Option<ClassName>,
    // Following functions are stored separately, since their order matters.
    register_methods_constants_fn: Option<ErasedRegisterFn>,
    register_properties_fn: Option<ErasedRegisterFn>,
    user_register_fn: Option<ErasedRegisterFn>,
    default_virtual_fn: sys::GDExtensionClassGetVirtual, // Option (set if there is at least one OnReady field)
    user_virtual_fn: sys::GDExtensionClassGetVirtual, // Option (set if there is a `#[godot_api] impl I*`)

    /// Godot low-level class creation parameters.
    #[cfg(before_api = "4.2")]
    godot_params: sys::GDExtensionClassCreationInfo,
    #[cfg(all(since_api = "4.2", before_api = "4.3"))]
    godot_params: sys::GDExtensionClassCreationInfo2,
    #[cfg(since_api = "4.3")]
    godot_params: sys::GDExtensionClassCreationInfo3,

    #[allow(dead_code)] // Currently unused; may be useful for diagnostics in the future.
    init_level: InitLevel,
    is_editor_plugin: bool,

    /// Used to ensure that each component is only filled once.
    component_already_filled: [bool; 3],
}

impl ClassRegistrationInfo {
    fn validate_unique(&mut self, item: &PluginItem) {
        // We could use mem::Discriminant, but match will fail to compile when a new component is added.

        // Note: when changing this match, make sure the array has sufficient size.
        let index = match item {
            PluginItem::Struct { .. } => 0,
            PluginItem::InherentImpl { .. } => 1,
            PluginItem::ITraitImpl { .. } => 2,
        };

        if self.component_already_filled[index] {
            panic!(
                "Godot class `{}` is defined multiple times in Rust; you can rename it with #[class(rename=NewName)]",
                self.class_name,
            )
        }

        self.component_already_filled[index] = true;
    }
}

/// Registers a class with static type information.
// Currently dead code, but will be needed for builder API. Don't remove.
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

    // This works as long as fields are called the same. May still need individual #[cfg]s for newer fields.
    #[cfg(before_api = "4.2")]
    type CreationInfo = sys::GDExtensionClassCreationInfo;
    #[cfg(all(since_api = "4.2", before_api = "4.3"))]
    type CreationInfo = sys::GDExtensionClassCreationInfo2;
    #[cfg(since_api = "4.3")]
    type CreationInfo = sys::GDExtensionClassCreationInfo3;

    let godot_params = CreationInfo {
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

    assert!(
        !T::class_name().is_empty(),
        "cannot register () or unnamed class"
    );

    register_class_raw(ClassRegistrationInfo {
        class_name: T::class_name(),
        parent_class_name: Some(T::Base::class_name()),
        register_methods_constants_fn: None,
        register_properties_fn: None,
        user_register_fn: Some(ErasedRegisterFn {
            raw: callbacks::register_class_by_builder::<T>,
        }),
        user_virtual_fn: None,
        default_virtual_fn: None,
        godot_params,
        init_level: T::INIT_LEVEL,
        is_editor_plugin: false,
        component_already_filled: Default::default(), // [false; N]
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
        // Filter per ClassPlugin and not PluginItem, because all components of all classes are mixed together in one huge list.
        if elem.init_level != init_level {
            return;
        }

        //out!("* Plugin: {elem:#?}");

        let name = elem.class_name;
        let class_info = map
            .entry(name)
            .or_insert_with(|| default_registration_info(name));

        fill_class_info(elem.item.clone(), class_info);
    });

    let mut loaded_classes_by_level = global_loaded_classes();
    for info in map.into_values() {
        let class_name = info.class_name;
        out!("Register class:   {class_name} at level `{init_level:?}`");
        let loaded_class = LoadedClass {
            name: class_name,
            is_editor_plugin: info.is_editor_plugin,
        };
        loaded_classes_by_level
            .entry(init_level)
            .or_default()
            .push(loaded_class);

        register_class_raw(info);
        out!("Class {class_name} loaded");
    }

    out!("All classes for level `{init_level:?}` auto-registered.");
}

pub fn unregister_classes(init_level: InitLevel) {
    let mut loaded_classes_by_level = global_loaded_classes();
    let loaded_classes_current_level = loaded_classes_by_level
        .remove(&init_level)
        .unwrap_or_default();
    out!("Unregistering classes of level {init_level:?}...");
    for class_name in loaded_classes_current_level.into_iter().rev() {
        unregister_class_raw(class_name);
    }
}

fn global_loaded_classes() -> GlobalGuard<'static, HashMap<InitLevel, Vec<LoadedClass>>> {
    match LOADED_CLASSES.try_lock() {
        Ok(it) => it,
        Err(err) => match err {
            GlobalLockError::Poisoned {..} => panic!(
                "global lock for loaded classes poisoned; class registration or deregistration may have panicked"
            ),
            GlobalLockError::WouldBlock => panic!("unexpected concurrent access to global lock for loaded classes"),
            GlobalLockError::InitFailed => unreachable!("global lock for loaded classes not initialized"),
        },
    }
}

/// Populate `c` with all the relevant data from `component` (depending on component type).
fn fill_class_info(item: PluginItem, c: &mut ClassRegistrationInfo) {
    c.validate_unique(&item);

    // out!("|   reg (before):    {c:?}");
    // out!("|   comp:            {component:?}");
    match item {
        PluginItem::Struct {
            base_class_name,
            generated_create_fn,
            generated_recreate_fn,
            register_properties_fn,
            free_fn,
            default_get_virtual_fn,
            is_tool,
            is_editor_plugin,
            is_hidden,
            is_instantiable,
        } => {
            c.parent_class_name = Some(base_class_name);
            c.default_virtual_fn = default_get_virtual_fn;
            c.register_properties_fn = Some(register_properties_fn);
            c.is_editor_plugin = is_editor_plugin;

            // Classes marked #[class(no_init)] are translated to "abstract" in Godot. This disables their default constructor.
            // "Abstract" is a misnomer -- it's not an abstract base class, but rather a "utility/static class" (although it can have instance
            // methods). Examples are Input, IP, FileAccess, DisplayServer.
            //
            // Abstract base classes on the other hand are called "virtual" in Godot. Examples are Mesh, Material, Texture.
            // For some reason, certain ABCs like PhysicsBody2D are not marked "virtual" but "abstract".
            //
            // See also: https://github.com/godotengine/godot/pull/58972
            c.godot_params.is_abstract = sys::conv::bool_to_sys(!is_instantiable);
            c.godot_params.free_instance_func = Some(free_fn);

            fill_into(
                &mut c.godot_params.create_instance_func,
                generated_create_fn,
            )
            .expect("duplicate: create_instance_func (def)");

            #[cfg(before_api = "4.2")]
            let _ = is_hidden; // mark used
            #[cfg(since_api = "4.2")]
            {
                fill_into(
                    &mut c.godot_params.recreate_instance_func,
                    generated_recreate_fn,
                )
                .expect("duplicate: recreate_instance_func (def)");

                c.godot_params.is_exposed = sys::conv::bool_to_sys(!is_hidden);
            }

            #[cfg(before_api = "4.2")]
            assert!(generated_recreate_fn.is_none()); // not used

            #[cfg(before_api = "4.3")]
            let _ = is_tool; // mark used
            #[cfg(since_api = "4.3")]
            {
                c.godot_params.is_runtime =
                    sys::conv::bool_to_sys(crate::private::is_class_runtime(is_tool));
            }
        }

        PluginItem::InherentImpl {
            register_methods_constants_fn,
        } => {
            c.register_methods_constants_fn = Some(register_methods_constants_fn);
        }

        PluginItem::ITraitImpl {
            user_register_fn,
            user_create_fn,
            user_recreate_fn,
            user_to_string_fn,
            user_on_notification_fn,
            user_set_fn,
            user_get_fn,
            get_virtual_fn,
            user_get_property_list_fn,
            user_free_property_list_fn,
            user_property_can_revert_fn,
            user_property_get_revert_fn,
        } => {
            c.user_register_fn = user_register_fn;

            // The following unwraps of fill_into() shouldn't panic, since rustc will error if there are
            // multiple `impl I{Class} for Thing` definitions.

            fill_into(&mut c.godot_params.create_instance_func, user_create_fn)
                .expect("duplicate: create_instance_func (i)");

            #[cfg(since_api = "4.2")]
            fill_into(&mut c.godot_params.recreate_instance_func, user_recreate_fn)
                .expect("duplicate: recreate_instance_func (i)");

            #[cfg(before_api = "4.2")]
            assert!(user_recreate_fn.is_none()); // not used

            c.godot_params.to_string_func = user_to_string_fn;
            c.godot_params.notification_func = user_on_notification_fn;
            c.godot_params.set_func = user_set_fn;
            c.godot_params.get_func = user_get_fn;
            c.godot_params.get_property_list_func = user_get_property_list_fn;
            c.godot_params.free_property_list_func = user_free_property_list_fn;
            c.godot_params.property_can_revert_func = user_property_can_revert_fn;
            c.godot_params.property_get_revert_func = user_property_get_revert_fn;
            c.user_virtual_fn = Some(get_virtual_fn);
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
fn register_class_raw(mut info: ClassRegistrationInfo) {
    // First register class...

    let class_name = info.class_name;
    let parent_class_name = info
        .parent_class_name
        .expect("class defined (parent_class_name)");

    // Register virtual functions -- if the user provided some via #[godot_api], take those; otherwise, use the
    // ones generated alongside #[derive(GodotClass)]. The latter can also be null, if no OnReady is provided.
    if info.godot_params.get_virtual_func.is_none() {
        info.godot_params.get_virtual_func = info.user_virtual_fn.or(info.default_virtual_fn);
    }

    // The explicit () type notifies us if Godot API ever adds a return type.
    let registration_failed = unsafe {
        // Try to register class...

        #[cfg(before_api = "4.2")]
        let _: () = interface_fn!(classdb_register_extension_class)(
            sys::get_library(),
            class_name.string_sys(),
            parent_class_name.string_sys(),
            ptr::addr_of!(info.godot_params),
        );

        #[cfg(all(since_api = "4.2", before_api = "4.3"))]
        let _: () = interface_fn!(classdb_register_extension_class2)(
            sys::get_library(),
            class_name.string_sys(),
            parent_class_name.string_sys(),
            ptr::addr_of!(info.godot_params),
        );

        #[cfg(since_api = "4.3")]
        let _: () = interface_fn!(classdb_register_extension_class3)(
            sys::get_library(),
            class_name.string_sys(),
            parent_class_name.string_sys(),
            ptr::addr_of!(info.godot_params),
        );

        // ...then see if it worked.
        // This is necessary because the above registration does not report errors (apart from console output).
        let tag = interface_fn!(classdb_get_class_tag)(class_name.string_sys());
        tag.is_null()
    };

    // Do not panic here; otherwise lock is poisoned and the whole extension becomes unusable.
    // This can happen during hot reload if a class changes base type in an incompatible way (e.g. RefCounted -> Node).
    if registration_failed {
        godot_error!(
            "Failed to register class `{class_name}`; check preceding Godot stderr messages"
        );
    }

    // ...then custom symbols

    //let mut class_builder = crate::builder::ClassBuilder::<?>::new();
    let mut class_builder = 0; // TODO dummy argument; see callbacks

    // Order of the following registrations is crucial:
    // 1. Methods and constants.
    // 2. Properties (they may depend on get/set methods).
    // 3. User-defined registration function (intuitively, user expects their own code to run after proc-macro generated code).
    if let Some(register_fn) = info.register_methods_constants_fn {
        (register_fn.raw)(&mut class_builder);
    }

    if let Some(register_fn) = info.register_properties_fn {
        (register_fn.raw)(&mut class_builder);
    }

    if let Some(register_fn) = info.user_register_fn {
        (register_fn.raw)(&mut class_builder);
    }

    #[cfg(since_api = "4.1")]
    if info.is_editor_plugin {
        unsafe { interface_fn!(editor_add_plugin)(class_name.string_sys()) };
    }
}

fn unregister_class_raw(class: LoadedClass) {
    let class_name = class.name;
    out!("Unregister class: {class_name}");

    // If class is an editor plugin, unregister that first.
    #[cfg(since_api = "4.1")]
    if class.is_editor_plugin {
        unsafe {
            interface_fn!(editor_remove_plugin)(class_name.string_sys());
        }

        out!("> Editor plugin removed");
    }

    #[allow(clippy::let_unit_value)]
    let _: () = unsafe {
        interface_fn!(classdb_unregister_extension_class)(
            sys::get_library(),
            class_name.string_sys(),
        )
    };

    out!("Class {class_name} unloaded");
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Substitutes for Default impl

// Yes, bindgen can implement Default, but only for _all_ types (with single exceptions).
// For FFI types, it's better to have explicit initialization in the general case though.
fn default_registration_info(class_name: ClassName) -> ClassRegistrationInfo {
    ClassRegistrationInfo {
        class_name,
        parent_class_name: None,
        register_methods_constants_fn: None,
        register_properties_fn: None,
        user_register_fn: None,
        default_virtual_fn: None,
        user_virtual_fn: None,
        godot_params: default_creation_info(),
        init_level: InitLevel::Scene,
        is_editor_plugin: false,
        component_already_filled: Default::default(), // [false; N]
    }
}

#[cfg(before_api = "4.2")]
fn default_creation_info() -> sys::GDExtensionClassCreationInfo {
    sys::GDExtensionClassCreationInfo {
        is_virtual: false as u8,
        is_abstract: false as u8,
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

#[cfg(all(since_api = "4.2", before_api = "4.3"))]
fn default_creation_info() -> sys::GDExtensionClassCreationInfo2 {
    sys::GDExtensionClassCreationInfo2 {
        is_virtual: false as u8,
        is_abstract: false as u8,
        is_exposed: sys::conv::SYS_TRUE,
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
    }
}

#[cfg(since_api = "4.3")]
fn default_creation_info() -> sys::GDExtensionClassCreationInfo3 {
    sys::GDExtensionClassCreationInfo3 {
        is_virtual: false as u8,
        is_abstract: false as u8,
        is_exposed: sys::conv::SYS_TRUE,
        is_runtime: sys::conv::SYS_TRUE,
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
    }
}
