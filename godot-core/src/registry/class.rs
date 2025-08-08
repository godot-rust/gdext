/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::collections::HashMap;
use std::{any, ptr};

use godot_ffi::join_with;
use sys::{interface_fn, out, Global, GlobalGuard, GlobalLockError};

use crate::classes::ClassDb;
use crate::init::InitLevel;
use crate::meta::error::FromGodotError;
use crate::meta::ClassName;
use crate::obj::{cap, DynGd, Gd, GodotClass};
use crate::private::{ClassPlugin, PluginItem};
use crate::registry::callbacks;
use crate::registry::plugin::{DynTraitImpl, ErasedRegisterFn, ITraitImpl, InherentImpl, Struct};
use crate::{classes, godot_error, godot_warn, sys};

/// Returns a lock to a global map of loaded classes, by initialization level.
///
/// Needed for class unregistering. The `static` is populated during class registering. There is no actual concurrency here, because Godot
/// calls register/unregister in the main thread. Mutex is just casual way to ensure safety in this non-performance-critical path.
/// Note that we panic on concurrent access instead of blocking (fail-fast approach). If that happens, most likely something changed on Godot
/// side and analysis required to adopt these changes.
fn global_loaded_classes_by_init_level(
) -> GlobalGuard<'static, HashMap<InitLevel, Vec<LoadedClass>>> {
    static LOADED_CLASSES_BY_INIT_LEVEL: Global<
        HashMap<InitLevel, Vec<LoadedClass>>, //.
    > = Global::default();

    lock_or_panic(&LOADED_CLASSES_BY_INIT_LEVEL, "loaded classes")
}

/// Returns a lock to a global map of loaded classes, by class name.
///
/// Complementary mechanism to the on-registration hooks like `__register_methods()`. This is used for runtime queries about a class, for
/// information which isn't stored in Godot. Example: list related `dyn Trait` implementations.
fn global_loaded_classes_by_name() -> GlobalGuard<'static, HashMap<ClassName, ClassMetadata>> {
    static LOADED_CLASSES_BY_NAME: Global<HashMap<ClassName, ClassMetadata>> = Global::default();

    lock_or_panic(&LOADED_CLASSES_BY_NAME, "loaded classes (by name)")
}

fn global_dyn_traits_by_typeid() -> GlobalGuard<'static, HashMap<any::TypeId, Vec<DynTraitImpl>>> {
    static DYN_TRAITS_BY_TYPEID: Global<HashMap<any::TypeId, Vec<DynTraitImpl>>> =
        Global::default();

    lock_or_panic(&DYN_TRAITS_BY_TYPEID, "dyn traits")
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

/// Represents a class which is currently loaded and retained in memory.
///
/// Besides the name, this type holds information relevant for the deregistration of the class.
pub struct LoadedClass {
    name: ClassName,
    is_editor_plugin: bool,
}

/// Represents a class which is currently loaded and retained in memory -- including metadata.
//
// Currently empty, but should already work for per-class queries.
pub struct ClassMetadata {}

// ----------------------------------------------------------------------------------------------------------------------------------------------

// This works as long as fields are called the same. May still need individual #[cfg]s for newer fields.
#[cfg(before_api = "4.2")]
type GodotCreationInfo = sys::GDExtensionClassCreationInfo;
#[cfg(all(since_api = "4.2", before_api = "4.3"))]
type GodotCreationInfo = sys::GDExtensionClassCreationInfo2;
#[cfg(all(since_api = "4.3", before_api = "4.4"))]
type GodotCreationInfo = sys::GDExtensionClassCreationInfo3;
#[cfg(since_api = "4.4")]
type GodotCreationInfo = sys::GDExtensionClassCreationInfo4;

#[cfg(before_api = "4.4")]
pub(crate) type GodotGetVirtual = <sys::GDExtensionClassGetVirtual as sys::Inner>::FnPtr;
#[cfg(since_api = "4.4")]
pub(crate) type GodotGetVirtual = <sys::GDExtensionClassGetVirtual2 as sys::Inner>::FnPtr;

#[derive(Debug)]
struct ClassRegistrationInfo {
    class_name: ClassName,
    parent_class_name: Option<ClassName>,
    // Following functions are stored separately, since their order matters.
    register_methods_constants_fn: Option<ErasedRegisterFn>,
    register_properties_fn: Option<ErasedRegisterFn>,
    user_register_fn: Option<ErasedRegisterFn>,
    default_virtual_fn: Option<GodotGetVirtual>, // Optional (set if there is at least one OnReady field)
    user_virtual_fn: Option<GodotGetVirtual>, // Optional (set if there is a `#[godot_api] impl I*`)

    /// Godot low-level class creation parameters.
    godot_params: GodotCreationInfo,

    #[allow(dead_code)] // Currently unused; may be useful for diagnostics in the future.
    init_level: InitLevel,
    is_editor_plugin: bool,

    /// One entry for each `dyn Trait` implemented (and registered) for this class.
    dynify_fns_by_trait: HashMap<any::TypeId, DynTraitImpl>,

    /// Used to ensure that each component is only filled once.
    component_already_filled: [bool; 4],
}

impl ClassRegistrationInfo {
    fn validate_unique(&mut self, item: &PluginItem) {
        // We could use mem::Discriminant, but match will fail to compile when a new component is added.

        // Note: when changing this match, make sure the array has sufficient size.
        let index = match item {
            PluginItem::Struct { .. } => 0,
            PluginItem::InherentImpl(_) => 1,
            PluginItem::ITraitImpl { .. } => 2,

            // Multiple dyn traits can be registered, thus don't validate for uniqueness.
            // (Still keep array size, so future additions don't have to regard this).
            PluginItem::DynTraitImpl { .. } => return,
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
#[expect(dead_code)] // Will be needed for builder API. Don't remove.
pub(crate) fn register_class<
    T: cap::GodotDefault
        + cap::ImplementsGodotVirtual
        + cap::GodotToString
        + cap::GodotNotification
        + cap::GodotRegisterClass
        + GodotClass,
>() {
    // TODO: provide overloads with only some trait impls

    out!("Manually register class {}", std::any::type_name::<T>());

    let godot_params = GodotCreationInfo {
        to_string_func: Some(callbacks::to_string::<T>),
        notification_func: Some(callbacks::on_notification::<T>),
        reference_func: Some(callbacks::reference::<T>),
        unreference_func: Some(callbacks::unreference::<T>),
        create_instance_func: Some(callbacks::create::<T>),
        free_instance_func: Some(callbacks::free::<T>),
        get_virtual_func: Some(callbacks::get_virtual::<T>),
        class_userdata: ptr::null_mut(), // will be passed to create fn, but global per class
        ..default_creation_info()
    };

    assert!(
        !T::class_name().is_none(),
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
        dynify_fns_by_trait: HashMap::new(),
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

    // First register all the loaded classes and dyn traits.
    // We need all the dyn classes in the registry to properly register DynGd properties;
    // one can do it directly inside the loop – by locking and unlocking the mutex –
    // but it is much slower and doesn't guarantee that all the dependent classes will be already loaded in most cases.
    register_classes_and_dyn_traits(&mut map, init_level);

    // Editor plugins should be added to the editor AFTER all the classes has been registered.
    // Adding EditorPlugin to the Editor before registering all the classes it depends on might result in crash.
    let mut editor_plugins: Vec<ClassName> = Vec::new();

    // Actually register all the classes.
    for info in map.into_values() {
        #[cfg(feature = "debug-log")]
        let class_name = info.class_name;

        if info.is_editor_plugin {
            editor_plugins.push(info.class_name);
        }

        register_class_raw(info);

        out!("Class {class_name} loaded.");
    }

    // Will imminently add given class to the editor.
    // It is expected and beneficial behaviour while we load library for the first time
    // but (for now) might lead to some issues during hot reload.
    // See also: (https://github.com/godot-rust/gdext/issues/1132)
    for editor_plugin_class_name in editor_plugins {
        unsafe { interface_fn!(editor_add_plugin)(editor_plugin_class_name.string_sys()) };
    }

    out!("All classes for level `{init_level:?}` auto-registered.");
}

fn register_classes_and_dyn_traits(
    map: &mut HashMap<ClassName, ClassRegistrationInfo>,
    init_level: InitLevel,
) {
    let mut loaded_classes_by_level = global_loaded_classes_by_init_level();
    let mut loaded_classes_by_name = global_loaded_classes_by_name();
    let mut dyn_traits_by_typeid = global_dyn_traits_by_typeid();

    for info in map.values_mut() {
        let class_name = info.class_name;
        out!("Register class:   {class_name} at level `{init_level:?}`");

        let loaded_class = LoadedClass {
            name: class_name,
            is_editor_plugin: info.is_editor_plugin,
        };
        let metadata = ClassMetadata {};

        // Transpose Class->Trait relations to Trait->Class relations.
        for (trait_type_id, mut dyn_trait_impl) in info.dynify_fns_by_trait.drain() {
            // Note: Must be done after filling out the class info since plugins are being iterated in unspecified order.
            dyn_trait_impl.parent_class_name = info.parent_class_name;

            dyn_traits_by_typeid
                .entry(trait_type_id)
                .or_default()
                .push(dyn_trait_impl);
        }

        loaded_classes_by_level
            .entry(init_level)
            .or_default()
            .push(loaded_class);

        loaded_classes_by_name.insert(class_name, metadata);
    }
}

pub fn unregister_classes(init_level: InitLevel) {
    let mut loaded_classes_by_level = global_loaded_classes_by_init_level();
    let mut loaded_classes_by_name = global_loaded_classes_by_name();
    // TODO clean up dyn traits

    let loaded_classes_current_level = loaded_classes_by_level
        .remove(&init_level)
        .unwrap_or_default();

    out!("Unregister classes of level {init_level:?}...");
    for class in loaded_classes_current_level.into_iter().rev() {
        // Remove from other map.
        loaded_classes_by_name.remove(&class.name);

        // Unregister from Godot.
        unregister_class_raw(class);
    }
}

#[cfg(feature = "codegen-full")]
pub fn auto_register_rpcs<T: GodotClass>(object: &mut T) {
    // Find the element that matches our class, and call the closure if it exists.
    if let Some(InherentImpl {
        register_rpcs_fn: Some(closure),
        ..
    }) = crate::private::find_inherent_impl(T::class_name())
    {
        (closure.raw)(object);
    }
}

/// Tries to convert a `Gd<T>` to a `DynGd<T, D>` for some class `T` and trait object `D`, where the trait may only be implemented for
/// some subclass of `T`.
///
/// This works even when `T` doesn't implement `AsDyn<D>`, as long as the dynamic class of `object` implements `AsDyn<D>`.
///
/// This only looks for an `AsDyn<D>` implementation in the dynamic class; the conversion will fail if the dynamic class doesn't
/// implement `AsDyn<D>`, even if there exists some superclass that does implement `AsDyn<D>`. This restriction could in theory be
/// lifted, but would need quite a bit of extra machinery to work.
pub(crate) fn try_dynify_object<T: GodotClass, D: ?Sized + 'static>(
    mut object: Gd<T>,
) -> Result<DynGd<T, D>, (FromGodotError, Gd<T>)> {
    let typeid = any::TypeId::of::<D>();
    let trait_name = sys::short_type_name::<D>();

    // Iterate all classes that implement the trait.
    let dyn_traits_by_typeid = global_dyn_traits_by_typeid();
    let Some(relations) = dyn_traits_by_typeid.get(&typeid) else {
        return Err((FromGodotError::UnregisteredDynTrait { trait_name }, object));
    };

    // TODO maybe use 2nd hashmap instead of linear search.
    // (probably not pair of typeid/classname, as that wouldn't allow the above check).
    for relation in relations {
        match relation.get_dyn_gd(object) {
            Ok(dyn_gd) => return Ok(dyn_gd),
            Err(obj) => object = obj,
        }
    }

    let error = FromGodotError::UnimplementedDynTrait {
        trait_name,
        class_name: object.dynamic_class_string().to_string(),
    };

    Err((error, object))
}

/// Responsible for creating hint_string for [`DynGd<T, D>`][crate::obj::DynGd] properties which works with [`PropertyHint::NODE_TYPE`][crate::global::PropertyHint::NODE_TYPE] or [`PropertyHint::RESOURCE_TYPE`][crate::global::PropertyHint::RESOURCE_TYPE].
///
/// Godot offers very limited capabilities when it comes to validating properties in the editor if given class isn't a tool.
/// Proper hint string combined with `PropertyHint::RESOURCE_TYPE` allows to limit selection only to valid classes - those registered as implementors of given `DynGd<T, D>`'s `D` trait.
/// Godot editor allows to export only one node type with `PropertyHint::NODE_TYPE` – therefore we are returning only the base class.
///
/// See also [Godot docs for PropertyHint](https://docs.godotengine.org/en/stable/classes/class_@globalscope.html#enum-globalscope-propertyhint).
pub(crate) fn get_dyn_property_hint_string<T, D>() -> String
where
    T: GodotClass,
    D: ?Sized + 'static,
{
    // Exporting multiple node types is not supported.
    if T::inherits::<classes::Node>() {
        return T::class_name().to_string();
    }

    let typeid = any::TypeId::of::<D>();
    let dyn_traits_by_typeid = global_dyn_traits_by_typeid();

    let Some(relations) = dyn_traits_by_typeid.get(&typeid) else {
        let trait_name = sys::short_type_name::<D>();
        godot_warn!(
            "godot-rust: No class has been linked to trait {trait_name} with #[godot_dyn]."
        );
        return String::new();
    };
    assert!(
        !relations.is_empty(),
        "Trait {trait_name} has been registered as DynGd Trait \
        despite no class being related to it \n\
        **this is a bug, please report it**",
        trait_name = sys::short_type_name::<D>()
    );

    // Include only implementors inheriting given T.
    // For example – don't include Nodes or Objects while creating hint_string for Resource.
    let relations_iter = relations.iter().filter_map(|implementor| {
        // TODO – check if caching it (using is_derived_base_cached) yields any benefits.
        if implementor.parent_class_name? == T::class_name()
            || ClassDb::singleton().is_parent_class(
                &implementor.parent_class_name?.to_string_name(),
                &T::class_name().to_string_name(),
            )
        {
            Some(implementor)
        } else {
            None
        }
    });

    join_with(relations_iter, ", ", |dyn_trait| {
        dyn_trait.class_name().to_cow_str()
    })
}

/// Populate `c` with all the relevant data from `component` (depending on component type).
fn fill_class_info(item: PluginItem, c: &mut ClassRegistrationInfo) {
    c.validate_unique(&item);

    // out!("|   reg (before):    {c:?}");
    // out!("|   comp:            {component:?}");
    match item {
        PluginItem::Struct(Struct {
            base_class_name,
            generated_create_fn,
            generated_recreate_fn,
            register_properties_fn,
            free_fn,
            default_get_virtual_fn,
            is_tool,
            is_editor_plugin,
            is_internal,
            is_instantiable,
            #[cfg(all(since_api = "4.3", feature = "register-docs"))]
                docs: _,
            reference_fn,
            unreference_fn,
        }) => {
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
            c.godot_params.reference_func = reference_fn;
            c.godot_params.unreference_func = unreference_fn;

            fill_into(
                &mut c.godot_params.create_instance_func,
                generated_create_fn,
            )
            .expect("duplicate: create_instance_func (def)");

            #[cfg(before_api = "4.2")]
            let _ = is_internal; // mark used
            #[cfg(since_api = "4.2")]
            {
                fill_into(
                    &mut c.godot_params.recreate_instance_func,
                    generated_recreate_fn,
                )
                .expect("duplicate: recreate_instance_func (def)");

                c.godot_params.is_exposed = sys::conv::bool_to_sys(!is_internal);
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

        PluginItem::InherentImpl(InherentImpl {
            register_methods_constants_fn,
            register_rpcs_fn: _,
            #[cfg(all(since_api = "4.3", feature = "register-docs"))]
                docs: _,
        }) => {
            c.register_methods_constants_fn = Some(register_methods_constants_fn);
        }

        PluginItem::ITraitImpl(ITraitImpl {
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
            #[cfg(all(since_api = "4.3", feature = "register-docs"))]
                virtual_method_docs: _,
            #[cfg(since_api = "4.2")]
            validate_property_fn,
        }) => {
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
            c.user_virtual_fn = get_virtual_fn;
            #[cfg(since_api = "4.2")]
            {
                c.godot_params.validate_property_func = validate_property_fn;
            }
        }
        PluginItem::DynTraitImpl(dyn_trait_impl) => {
            let type_id = dyn_trait_impl.dyn_trait_typeid();

            let prev = c.dynify_fns_by_trait.insert(type_id, dyn_trait_impl);

            assert!(
                prev.is_none(),
                "Duplicate registration of {:?} for class {}",
                type_id,
                c.class_name
            );
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
    // Some metadata like dynify fns are already emptied at this point. Only consider registrations for Godot.

    // First register class...
    validate_class_constraints(&info);

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
        let register_fn = interface_fn!(classdb_register_extension_class);

        #[cfg(all(since_api = "4.2", before_api = "4.3"))]
        let register_fn = interface_fn!(classdb_register_extension_class2);

        #[cfg(all(since_api = "4.3", before_api = "4.4"))]
        let register_fn = interface_fn!(classdb_register_extension_class3);

        #[cfg(since_api = "4.4")]
        let register_fn = interface_fn!(classdb_register_extension_class4);

        let _: () = register_fn(
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
            "Failed to register class `{class_name}`; check preceding Godot stderr messages."
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
}

fn validate_class_constraints(_class: &ClassRegistrationInfo) {
    // TODO: if we add builder API, the proc-macro checks in parse_struct_attributes() etc. should be duplicated here.
}

fn unregister_class_raw(class: LoadedClass) {
    let class_name = class.name;
    out!("Unregister class: {class_name}");

    // If class is an editor plugin, unregister that first.
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

fn lock_or_panic<T>(global: &'static Global<T>, ctx: &str) -> GlobalGuard<'static, T> {
    match global.try_lock() {
        Ok(it) => it,
        Err(err) => match err {
            GlobalLockError::Poisoned { .. } => panic!(
                "global lock for {ctx} poisoned; class registration or deregistration may have panicked"
            ),
            GlobalLockError::WouldBlock => panic!("unexpected concurrent access to global lock for {ctx}"),
            GlobalLockError::InitFailed => unreachable!("global lock for {ctx} not initialized"),
        },
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
        register_methods_constants_fn: None,
        register_properties_fn: None,
        user_register_fn: None,
        default_virtual_fn: None,
        user_virtual_fn: None,
        godot_params: default_creation_info(),
        init_level: InitLevel::Scene,
        is_editor_plugin: false,
        dynify_fns_by_trait: HashMap::new(),
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

#[cfg(all(since_api = "4.3", before_api = "4.4"))]
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

#[cfg(since_api = "4.4")]
fn default_creation_info() -> sys::GDExtensionClassCreationInfo4 {
    sys::GDExtensionClassCreationInfo4 {
        is_virtual: false as u8,
        is_abstract: false as u8,
        is_exposed: sys::conv::SYS_TRUE,
        is_runtime: sys::conv::SYS_TRUE,
        icon_path: ptr::null(),
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
        class_userdata: ptr::null_mut(),
    }
}
