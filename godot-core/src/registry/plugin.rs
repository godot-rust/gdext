/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::any::Any;
use std::{any, fmt};

use crate::init::InitLevel;
use crate::meta::ClassId;
use crate::obj::{bounds, cap, Bounds, DynGd, Gd, GodotClass, Inherits, UserClass};
use crate::registry::callbacks;
use crate::registry::class::GodotGetVirtual;
use crate::{classes, sys};

// TODO(bromeon): some information coming from the proc-macro API is deferred through PluginItem, while others is directly
// translated to code. Consider moving more code to the PluginItem, which allows for more dynamic registration and will
// be easier for a future builder API.

// ----------------------------------------------------------------------------------------------------------------------------------------------

/// Piece of information that is gathered by the self-registration ("plugin") system.
///
/// You should not manually construct this struct, but rather use [`ClassPlugin::new()`].
#[derive(Debug)]
pub struct ClassPlugin {
    /// The name of the class to register plugins for.
    ///
    /// This is used to group plugins so that all class properties for a single class can be registered at the same time.
    /// Incorrectly setting this value should not cause any UB but will likely cause errors during registration time.
    pub(crate) class_name: ClassId,

    /// Which [`InitLevel`] this plugin should be registered at.
    ///
    /// Incorrectly setting this value should not cause any UB but will likely cause errors during registration time.
    // Init-level is per ClassPlugin and not per PluginItem, because all components of all classes are mixed together in one
    // huge linker list. There is no per-class aggregation going on, so this allows to easily filter relevant classes.
    pub(crate) init_level: InitLevel,

    /// The actual item being registered.
    pub(crate) item: PluginItem,
}

impl ClassPlugin {
    /// Creates a new `ClassPlugin`, automatically setting the `class_name` and `init_level` to the values defined in [`GodotClass`].
    pub fn new<T: GodotClass>(item: PluginItem) -> Self {
        Self {
            class_name: T::class_id(),
            init_level: T::INIT_LEVEL,
            item,
        }
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Type-erased values

/// Type-erased function object, holding a function which should called during class registration.
#[derive(Copy, Clone)]
pub struct ErasedRegisterFn {
    // A wrapper is needed here because Debug can't be derived on function pointers with reference parameters, so this won't work:
    // pub type ErasedRegisterFn = fn(&mut dyn std::any::Any);
    // (see https://stackoverflow.com/q/53380040)
    /// The actual function to be called during class registration.
    pub raw: fn(&mut dyn Any),
}

impl fmt::Debug for ErasedRegisterFn {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "0x{:0>16x}", self.raw as usize)
    }
}

/// Type-erased function object, holding a function which should be called during RPC function registration.
#[derive(Copy, Clone)]
pub struct ErasedRegisterRpcsFn {
    // A wrapper is needed here because Debug can't be derived on function pointers with reference parameters, so this won't work:
    // pub type ErasedRegisterFn = fn(&mut dyn std::any::Any);
    // (see https://stackoverflow.com/q/53380040)
    /// The actual function to be called during RPC function registration.
    ///
    /// This should be called with a reference to the object that we want to register RPC functions for.
    pub raw: fn(&mut dyn Any),
}

impl fmt::Debug for ErasedRegisterRpcsFn {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "0x{:0>16x}", self.raw as usize)
    }
}

/// Type-erased function which converts a `Gd<Object>` into a `DynGd<Object, D>` for some trait object `D`.
///
/// See [`DynTraitImpl`] for usage.
pub type ErasedDynifyFn = unsafe fn(Gd<classes::Object>) -> ErasedDynGd;

/// Type-erased `DynGd<Object, D>` for some trait object `D`.
///
/// See [`DynTraitImpl`] for usage.
pub struct ErasedDynGd {
    pub boxed: Box<dyn Any>,
}

type GodotCreateFn = unsafe extern "C" fn(
    _class_userdata: *mut std::ffi::c_void,
    #[cfg(since_api = "4.4")] _notify_postinitialize: sys::GDExtensionBool,
) -> sys::GDExtensionObjectPtr;

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Plugin items

/// Represents the data part of a [`ClassPlugin`] instance.
///
/// Each enumerator represents a different item in Rust code, which is processed by an independent proc macro (for example,
/// `#[derive(GodotClass)]` on structs, or `#[godot_api]` on impl blocks).
#[derive(Clone, Debug)]
pub enum PluginItem {
    /// Class definition itself, must always be available -- created by `#[derive(GodotClass)]`.
    Struct(Struct),

    /// Collected from `#[godot_api] impl MyClass`.
    InherentImpl(InherentImpl),

    /// Collected from `#[godot_api] impl I... for MyClass`.
    ITraitImpl(ITraitImpl),

    /// Collected from `#[godot_dyn]` macro invocations.
    DynTraitImpl(DynTraitImpl),
}

/// Helper function which checks that the field has not been set before.
fn set<T>(field: &mut Option<T>, value: T) {
    assert!(field.is_none(), "attempted to set field more than once",);
    *field = Some(value);
}

/// The data for a class definition.
#[derive(Clone, Debug)]
pub struct Struct {
    /// The name of the base class in Godot.
    ///
    /// This must match [`GodotClass::Base`]'s class name.
    pub(crate) base_class_name: ClassId,

    /// Godot low-level `create` function, wired up to library-generated `init`.
    ///
    /// For `#[class(no_init)]`, behavior depends on Godot version:
    /// - 4.5 and later: `None`
    /// - until 4.4: a dummy function that fails, to not break hot reloading.
    ///
    /// This is mutually exclusive with [`ITraitImpl::user_create_fn`].
    pub(crate) generated_create_fn: Option<GodotCreateFn>,

    /// Godot low-level `recreate` function, used when hot-reloading a user class.
    ///
    /// This is mutually exclusive with [`ITraitImpl::user_recreate_fn`].
    pub(crate) generated_recreate_fn: Option<
        unsafe extern "C" fn(
            p_class_userdata: *mut std::ffi::c_void,
            p_object: sys::GDExtensionObjectPtr,
        ) -> sys::GDExtensionClassInstancePtr,
    >,

    /// Callback to library-generated function which registers properties in the `struct` definition.
    pub(crate) register_properties_fn: ErasedRegisterFn,

    /// Callback on refc-increment. Only for `RefCounted` classes.
    pub(crate) reference_fn: sys::GDExtensionClassReference,

    /// Callback on refc-decrement. Only for `RefCounted` classes.
    pub(crate) unreference_fn: sys::GDExtensionClassUnreference,

    /// Function called by Godot when an object of this class is freed.
    ///
    /// Always implemented as [`callbacks::free`].
    pub(crate) free_fn: unsafe extern "C" fn(
        _class_user_data: *mut std::ffi::c_void,
        instance: sys::GDExtensionClassInstancePtr,
    ),

    /// Calls `__before_ready()`, if there is at least one `OnReady` field. Used if there is no `#[godot_api] impl` block
    /// overriding ready.
    pub(crate) default_get_virtual_fn: Option<GodotGetVirtual>,

    /// Whether `#[class(tool)]` was used.
    pub(crate) is_tool: bool,

    /// Whether the base class is an `EditorPlugin`.
    pub(crate) is_editor_plugin: bool,

    /// Whether `#[class(internal)]` was used.
    pub(crate) is_internal: bool,

    /// Whether the class has a default constructor.
    pub(crate) is_instantiable: bool,
}

impl Struct {
    pub fn new<T: GodotClass + cap::ImplementsGodotExports>() -> Self {
        let refcounted = <T::Memory as bounds::Memory>::IS_REF_COUNTED;

        Self {
            base_class_name: T::Base::class_id(),
            generated_create_fn: None,
            generated_recreate_fn: None,
            register_properties_fn: ErasedRegisterFn {
                raw: callbacks::register_user_properties::<T>,
            },
            free_fn: callbacks::free::<T>,
            default_get_virtual_fn: None,
            is_tool: false,
            is_editor_plugin: false,
            is_internal: false,
            is_instantiable: false,
            // While Godot doesn't do anything with these callbacks for non-RefCounted classes, we can avoid instantiating them in Rust.
            reference_fn: refcounted.then_some(callbacks::reference::<T>),
            unreference_fn: refcounted.then_some(callbacks::unreference::<T>),
        }
    }

    pub fn with_generated<T: GodotClass + cap::GodotDefault>(mut self) -> Self {
        set(&mut self.generated_create_fn, callbacks::create::<T>);

        set(&mut self.generated_recreate_fn, callbacks::recreate::<T>);
        self
    }

    // Workaround for https://github.com/godot-rust/gdext/issues/874, before https://github.com/godotengine/godot/pull/99133 is merged in 4.5.
    #[cfg(before_api = "4.5")]
    pub fn with_generated_no_default<T: GodotClass>(mut self) -> Self {
        set(&mut self.generated_create_fn, callbacks::create_null::<T>);

        set(
            &mut self.generated_recreate_fn,
            callbacks::recreate_null::<T>,
        );
        self
    }

    pub fn with_default_get_virtual_fn<T: GodotClass + UserClass>(mut self) -> Self {
        set(
            &mut self.default_get_virtual_fn,
            callbacks::default_get_virtual::<T>,
        );
        self
    }

    pub fn with_tool(mut self) -> Self {
        self.is_tool = true;
        self
    }

    pub fn with_editor_plugin(mut self) -> Self {
        self.is_editor_plugin = true;
        self
    }

    pub fn with_internal(mut self) -> Self {
        self.is_internal = true;
        self
    }

    pub fn with_instantiable(mut self) -> Self {
        self.is_instantiable = true;
        self
    }
}

/// Stores registration functions for methods, constants, and documentation from inherent `#[godot_api]` impl blocks.
#[derive(Clone, Debug)]
pub struct InherentImpl {
    /// Callback to library-generated function which registers functions and constants in the `impl` block.
    ///
    /// Always present since that's the entire point of this `impl` block.
    pub(crate) register_methods_constants_fn: ErasedRegisterFn,

    /// Callback to library-generated function which calls [`Node::rpc_config`](crate::classes::Node::rpc_config) for each function annotated
    /// with `#[rpc]` on the `impl` block.
    ///
    /// This function is called in [`UserClass::__before_ready()`](crate::obj::UserClass::__before_ready) definitions generated by the
    /// `#[derive(GodotClass)]` macro.
    // This field is only used during codegen-full.
    #[cfg_attr(not(feature = "codegen-full"), expect(dead_code))]
    pub(crate) register_rpcs_fn: Option<ErasedRegisterRpcsFn>,
}

impl InherentImpl {
    pub fn new<T: cap::ImplementsGodotApi>() -> Self {
        Self {
            register_methods_constants_fn: ErasedRegisterFn {
                raw: callbacks::register_user_methods_constants::<T>,
            },
            register_rpcs_fn: Some(ErasedRegisterRpcsFn {
                raw: callbacks::register_user_rpcs::<T>,
            }),
        }
    }
}

#[derive(Default, Clone, Debug)]
pub struct ITraitImpl {
    /// Callback to user-defined `register_class` function.
    pub(crate) user_register_fn: Option<ErasedRegisterFn>,

    /// Godot low-level `create` function, wired up to the user's `init`.
    ///
    /// This is mutually exclusive with [`Struct::generated_create_fn`].
    pub(crate) user_create_fn: Option<GodotCreateFn>,

    /// Godot low-level `recreate` function, used when hot-reloading a user class.
    ///
    /// This is mutually exclusive with [`Struct::generated_recreate_fn`].
    pub(crate) user_recreate_fn: Option<
        unsafe extern "C" fn(
            p_class_userdata: *mut ::std::os::raw::c_void,
            p_object: sys::GDExtensionObjectPtr,
        ) -> sys::GDExtensionClassInstancePtr,
    >,

    /// User-defined `to_string` function.
    pub(crate) user_to_string_fn: Option<
        unsafe extern "C" fn(
            p_instance: sys::GDExtensionClassInstancePtr,
            r_is_valid: *mut sys::GDExtensionBool,
            r_out: sys::GDExtensionStringPtr,
        ),
    >,

    /// User-defined `on_notification` function.
    pub(crate) user_on_notification_fn: Option<
        unsafe extern "C" fn(
            p_instance: sys::GDExtensionClassInstancePtr,
            p_what: i32,
            p_reversed: sys::GDExtensionBool,
        ),
    >,

    /// User-defined `set_property` function.
    pub(crate) user_set_fn: Option<
        unsafe extern "C" fn(
            p_instance: sys::GDExtensionClassInstancePtr,
            p_name: sys::GDExtensionConstStringNamePtr,
            p_value: sys::GDExtensionConstVariantPtr,
        ) -> sys::GDExtensionBool,
    >,

    /// User-defined `get_property` function.
    pub(crate) user_get_fn: Option<
        unsafe extern "C" fn(
            p_instance: sys::GDExtensionClassInstancePtr,
            p_name: sys::GDExtensionConstStringNamePtr,
            r_ret: sys::GDExtensionVariantPtr,
        ) -> sys::GDExtensionBool,
    >,

    /// Callback for other virtual methods specific to each class.
    pub(crate) get_virtual_fn: Option<GodotGetVirtual>,

    /// User-defined `get_property_list` function.
    pub(crate) user_get_property_list_fn: Option<
        unsafe extern "C" fn(
            p_instance: sys::GDExtensionClassInstancePtr,
            r_count: *mut u32,
        ) -> *const sys::GDExtensionPropertyInfo,
    >,

    // We do not support using this in Godot < 4.3, however it's easier to leave this in and fail elsewhere when attempting to use
    // this in Godot < 4.3.
    #[cfg(before_api = "4.3")]
    pub(crate) user_free_property_list_fn: Option<
        unsafe extern "C" fn(
            p_instance: sys::GDExtensionClassInstancePtr,
            p_list: *const sys::GDExtensionPropertyInfo,
        ),
    >,
    /// Frees the property list created in the user-defined `get_property_list` function.
    #[cfg(since_api = "4.3")]
    pub(crate) user_free_property_list_fn: Option<
        unsafe extern "C" fn(
            p_instance: sys::GDExtensionClassInstancePtr,
            p_list: *const sys::GDExtensionPropertyInfo,
            p_count: u32,
        ),
    >,

    /// Part of user-defined `property_get_revert` function.
    ///
    /// This effectively just calls [`Option::is_some`] on the return value of the `property_get_revert` function.
    pub(crate) user_property_can_revert_fn: Option<
        unsafe extern "C" fn(
            p_instance: sys::GDExtensionClassInstancePtr,
            p_name: sys::GDExtensionConstStringNamePtr,
        ) -> sys::GDExtensionBool,
    >,

    /// Part of user-defined `property_get_revert` function.
    ///
    /// This returns null when the return value of `property_get_revert` is `None`, and otherwise returns the value contained
    /// within the `Some`.
    pub(crate) user_property_get_revert_fn: Option<
        unsafe extern "C" fn(
            p_instance: sys::GDExtensionClassInstancePtr,
            p_name: sys::GDExtensionConstStringNamePtr,
            r_ret: sys::GDExtensionVariantPtr,
        ) -> sys::GDExtensionBool,
    >,
    pub(crate) validate_property_fn: Option<
        unsafe extern "C" fn(
            p_instance: sys::GDExtensionClassInstancePtr,
            p_property: *mut sys::GDExtensionPropertyInfo,
        ) -> sys::GDExtensionBool,
    >,
}

impl ITraitImpl {
    pub fn new<T: GodotClass + cap::ImplementsGodotVirtual>() -> Self {
        Self {
            get_virtual_fn: Some(callbacks::get_virtual::<T>),
            ..Default::default()
        }
    }

    pub fn with_register<T: GodotClass + cap::GodotRegisterClass>(mut self) -> Self {
        set(
            &mut self.user_register_fn,
            ErasedRegisterFn {
                raw: callbacks::register_class_by_builder::<T>,
            },
        );

        self
    }

    pub fn with_create<T: GodotClass + cap::GodotDefault>(mut self) -> Self {
        set(&mut self.user_create_fn, callbacks::create::<T>);

        #[cfg(since_api = "4.3")]
        set(&mut self.user_recreate_fn, callbacks::recreate::<T>);
        self
    }

    pub fn with_string<T: GodotClass + cap::GodotToString>(mut self) -> Self {
        set(&mut self.user_to_string_fn, callbacks::to_string::<T>);
        self
    }

    pub fn with_on_notification<T: GodotClass + cap::GodotNotification>(mut self) -> Self {
        set(
            &mut self.user_on_notification_fn,
            callbacks::on_notification::<T>,
        );
        self
    }

    pub fn with_get_property<T: GodotClass + cap::GodotGet>(mut self) -> Self {
        set(&mut self.user_get_fn, callbacks::get_property::<T>);
        self
    }

    pub fn with_set_property<T: GodotClass + cap::GodotSet>(mut self) -> Self {
        set(&mut self.user_set_fn, callbacks::set_property::<T>);
        self
    }

    pub fn with_get_property_list<T: GodotClass + cap::GodotGetPropertyList>(mut self) -> Self {
        set(
            &mut self.user_get_property_list_fn,
            callbacks::get_property_list::<T>,
        );

        #[cfg(since_api = "4.3")]
        set(
            &mut self.user_free_property_list_fn,
            callbacks::free_property_list::<T>,
        );
        self
    }

    pub fn with_property_get_revert<T: GodotClass + cap::GodotPropertyGetRevert>(mut self) -> Self {
        set(
            &mut self.user_property_get_revert_fn,
            callbacks::property_get_revert::<T>,
        );
        set(
            &mut self.user_property_can_revert_fn,
            callbacks::property_can_revert::<T>,
        );
        self
    }

    pub fn with_validate_property<T: GodotClass + cap::GodotValidateProperty>(mut self) -> Self {
        set(
            &mut self.validate_property_fn,
            callbacks::validate_property::<T>,
        );
        self
    }
}

/// Representation of a `#[godot_dyn]` invocation.
///
/// Stores all the information needed for `DynGd` re-enrichment.
#[derive(Clone, Debug)]
pub struct DynTraitImpl {
    /// The class that this `dyn Trait` implementation corresponds to.
    class_name: ClassId,

    /// Base inherited class required for `DynGd<T, D>` exports (i.e. one specified in `#[class(base = ...)]`).
    ///
    /// Godot doesn't guarantee availability of all the GDExtension classes through the ClassDb while generating `PropertyHintInfo` for our exports.
    /// Therefore, we rely on the built-in inherited base class in such cases.
    /// Only [`class_name`][DynTraitImpl::class_name] is available at the time of adding given `DynTraitImpl` to plugin registry with `#[godot_dyn]`;
    /// It is important to fill this information before registration.
    ///
    /// See also [`get_dyn_property_hint_string`][crate::registry::class::get_dyn_property_hint_string].
    pub(crate) parent_class_name: Option<ClassId>,

    /// TypeId of the `dyn Trait` object.
    dyn_trait_typeid: any::TypeId,

    /// Function used to get a `DynGd<T,D>` from a `Gd<Object>`. This is used in the [`FromGodot`](crate::meta::FromGodot) implementation
    /// of [`DynGd`]. This function is always implemented as [`callbacks::dynify_fn::<T, D>`] where `T` is the class represented by `class_name`
    /// and `D` is the trait object corresponding to `dyn_trait_typeid`.
    ///
    /// Function that converts a `Gd<Object>` to a type-erased `DynGd<Object, dyn Trait>` (with the latter erased for common storage).
    erased_dynify_fn: ErasedDynifyFn,
}

impl DynTraitImpl {
    pub fn new<T, D>() -> Self
    where
        T: GodotClass
            + Inherits<classes::Object>
            + crate::obj::AsDyn<D>
            + Bounds<Declarer = bounds::DeclUser>,
        D: ?Sized + 'static,
    {
        Self {
            class_name: T::class_id(),
            parent_class_name: None,
            dyn_trait_typeid: std::any::TypeId::of::<D>(),
            erased_dynify_fn: callbacks::dynify_fn::<T, D>,
        }
    }

    /// The class that this `dyn Trait` implementation corresponds to.
    pub fn class_name(&self) -> &ClassId {
        &self.class_name
    }

    /// The type id of the trait object this was registered with.
    pub fn dyn_trait_typeid(&self) -> any::TypeId {
        self.dyn_trait_typeid
    }

    /// Convert a [`Gd<T>`] to a [`DynGd<T, D>`] using `self`.
    ///
    /// This will fail with `Err(object)` if the dynamic class of `object` does not match the [`ClassId`] stored in `self`.
    pub fn get_dyn_gd<T: GodotClass, D: ?Sized + 'static>(
        &self,
        object: Gd<T>,
    ) -> Result<DynGd<T, D>, Gd<T>> {
        let dynamic_class = object.dynamic_class_string();

        if dynamic_class != self.class_name.to_string_name() {
            return Err(object);
        }

        let object = object.upcast_object();

        // SAFETY: `DynTraitImpl::new` ensures that this function is safe to call when `object` is castable to `self.class_name`.
        // Since the dynamic class of `object` is `self.class_name`, it must be castable to `self.class_name`.
        let erased_dyn = unsafe { (self.erased_dynify_fn)(object) };

        let dyn_gd_object = erased_dyn.boxed.downcast::<DynGd<classes::Object, D>>();

        // SAFETY: `callbacks::dynify_fn` returns a `DynGd<Object, D>` which has been type erased. So this downcast will always succeed.
        let dyn_gd_object = unsafe { dyn_gd_object.unwrap_unchecked() };

        // SAFETY: This is effectively upcasting a value which has class equal to `self.class_name` to a `DynGd<T, D>`. Since the class of
        // `object` is `T` and its dynamic class is `self.class_name`, this means that `T` must be a superclass of `self.class_name`. Thus
        // this upcast is safe.
        let dyn_gd_t = unsafe { dyn_gd_object.cast_unchecked::<T>() };

        Ok(dyn_gd_t)
    }
}
