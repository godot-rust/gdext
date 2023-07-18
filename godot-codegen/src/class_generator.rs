/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! Generates a file for each Godot engine + builtin class

use proc_macro2::{Ident, Literal, TokenStream};
use quote::{format_ident, quote, ToTokens};
use std::path::Path;

use crate::central_generator::{collect_builtin_types, BuiltinTypeInfo};
use crate::util::{
    ident, option_as_slice, parse_native_structures_format, safe_ident, to_pascal_case,
    to_rust_expr, to_rust_type, to_rust_type_abi, to_snake_case, NativeStructuresField,
};
use crate::{api_parser::*, SubmitFn};
use crate::{
    special_cases, util, Context, GeneratedBuiltin, GeneratedBuiltinModule, GeneratedClass,
    GeneratedClassModule, ModName, RustTy, TyName,
};

// ----------------------------------------------------------------------------------------------------------------------------------------------

struct FnReceiver {
    /// `&self`, `&mut self`, (none)
    param: TokenStream,

    /// `ptr::null_mut()`, `self.object_ptr`, `self.sys_ptr`, (none)
    ffi_arg: TokenStream,

    /// `Self::`, `self.`
    #[allow(dead_code)] // TODO remove as soon as used
    self_prefix: TokenStream,
}

impl FnReceiver {
    /// No receiver, not even static `Self`
    fn global_function() -> FnReceiver {
        FnReceiver {
            param: TokenStream::new(),
            ffi_arg: TokenStream::new(),
            self_prefix: TokenStream::new(),
        }
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

struct FnParam {
    name: Ident,
    type_: RustTy,
    default_value: Option<TokenStream>,
}

impl FnParam {
    fn new_range(method_args: &Option<Vec<MethodArg>>, ctx: &mut Context) -> Vec<FnParam> {
        option_as_slice(method_args)
            .iter()
            .map(|arg| Self::new(arg, ctx))
            .collect()
    }

    fn new_range_no_defaults(
        method_args: &Option<Vec<MethodArg>>,
        ctx: &mut Context,
    ) -> Vec<FnParam> {
        option_as_slice(method_args)
            .iter()
            .map(|arg| Self::new_no_defaults(arg, ctx))
            .collect()
    }

    fn new(method_arg: &MethodArg, ctx: &mut Context) -> FnParam {
        let name = safe_ident(&method_arg.name);
        let type_ = to_rust_type(&method_arg.type_, method_arg.meta.as_ref(), ctx);
        let default_value = method_arg
            .default_value
            .as_ref()
            .map(|v| to_rust_expr(v, &type_));

        FnParam {
            name,
            type_,
            default_value,
        }
    }

    fn new_no_defaults(method_arg: &MethodArg, ctx: &mut Context) -> FnParam {
        FnParam {
            name: safe_ident(&method_arg.name),
            type_: to_rust_type(&method_arg.type_, method_arg.meta.as_ref(), ctx),
            //type_: to_rust_type(&method_arg.type_, &method_arg.meta, ctx),
            default_value: None,
        }
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

struct FnReturn {
    decl: TokenStream,
    type_: Option<RustTy>,
}

impl FnReturn {
    fn new(return_value: &Option<MethodReturn>, ctx: &mut Context) -> Self {
        if let Some(ret) = return_value {
            let ty = to_rust_type(&ret.type_, ret.meta.as_ref(), ctx);

            Self {
                decl: ty.return_decl(),
                type_: Some(ty),
            }
        } else {
            Self {
                decl: TokenStream::new(),
                type_: None,
            }
        }
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

enum FnQualifier {
    Mut,    // &mut self
    Const,  // &self
    Static, // Self
    Global, // (nothing)
}

impl FnQualifier {
    fn is_static_or_global(&self) -> bool {
        matches!(self, Self::Static | Self::Global)
    }
}

impl FnQualifier {
    fn for_method(is_const: bool, is_static: bool) -> FnQualifier {
        if is_static {
            FnQualifier::Static
        } else if is_const {
            FnQualifier::Const
        } else {
            FnQualifier::Mut
        }
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

struct FnSignature<'a> {
    function_name: &'a str,
    surrounding_class: Option<&'a TyName>, // None if global function
    is_private: bool,
    is_virtual: bool,
    qualifier: FnQualifier,
    params: Vec<FnParam>,
    return_value: FnReturn,
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

struct FnCode {
    receiver: FnReceiver,
    variant_ffi: Option<VariantFfi>,
    init_code: TokenStream,
    varcall_invocation: TokenStream,
    ptrcall_invocation: TokenStream,
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

struct FnDefinition {
    functions: TokenStream,
    builders: TokenStream,
}

impl FnDefinition {
    fn none() -> FnDefinition {
        FnDefinition {
            functions: TokenStream::new(),
            builders: TokenStream::new(),
        }
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

struct FnDefinitions {
    functions: TokenStream,
    builders: TokenStream,
}

impl FnDefinitions {
    fn expand(definitions: impl Iterator<Item = FnDefinition>) -> FnDefinitions {
        // Collect needed because borrowed by 2 closures
        let definitions: Vec<_> = definitions.collect();
        let functions = definitions.iter().map(|def| &def.functions);
        let structs = definitions.iter().map(|def| &def.builders);

        FnDefinitions {
            functions: quote! { #( #functions )* },
            builders: quote! { #( #structs )* },
        }
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

pub(crate) fn generate_class_files(
    api: &ExtensionApi,
    ctx: &mut Context,
    _build_config: &str,
    gen_path: &Path,
    submit_fn: &mut SubmitFn,
) {
    let _ = std::fs::remove_dir_all(gen_path);
    std::fs::create_dir_all(gen_path).expect("create classes directory");

    let mut modules = vec![];
    for class in api.classes.iter() {
        let class_name = TyName::from_godot(&class.name);
        let module_name = ModName::from_godot(&class.name);

        #[cfg(not(feature = "codegen-full"))]
        if !crate::SELECTED_CLASSES.contains(&class_name.godot_ty.as_str()) {
            continue;
        }

        if special_cases::is_class_deleted(&class_name) {
            continue;
        }

        let generated_class = make_class(class, &class_name, ctx);
        let file_contents = generated_class.code;

        let out_path = gen_path.join(format!("{}.rs", module_name.rust_mod));

        submit_fn(out_path, file_contents);

        modules.push(GeneratedClassModule {
            class_name,
            module_name,
            own_notification_enum_name: generated_class
                .has_own_notification_enum
                .then_some(generated_class.notification_enum_name),
            inherits_macro_ident: generated_class.inherits_macro_ident,
            is_pub_sidecar: generated_class.has_sidecar_module,
        });
    }

    let out_path = gen_path.join("mod.rs");
    let mod_contents = make_module_file(modules);

    submit_fn(out_path, mod_contents);
}

pub(crate) fn generate_builtin_class_files(
    api: &ExtensionApi,
    ctx: &mut Context,
    _build_config: &str,
    gen_path: &Path,
    submit_fn: &mut SubmitFn,
) {
    let _ = std::fs::remove_dir_all(gen_path);
    std::fs::create_dir_all(gen_path).expect("create classes directory");

    let builtin_types_map = collect_builtin_types(api);

    let mut modules = vec![];
    for class in api.builtin_classes.iter() {
        let module_name = ModName::from_godot(&class.name);
        let class_name = TyName::from_godot(&class.name);
        let inner_class_name = TyName::from_godot(&format!("Inner{}", class.name));

        if special_cases::is_builtin_type_deleted(&class_name) {
            continue;
        }

        let type_info = builtin_types_map
            .get(&class.name)
            .unwrap_or_else(|| panic!("builtin type not found: {}", class.name));

        let generated_class =
            make_builtin_class(class, &class_name, &inner_class_name, type_info, ctx);
        let file_contents = generated_class.code;

        let out_path = gen_path.join(format!("{}.rs", module_name.rust_mod));

        submit_fn(out_path, file_contents);

        modules.push(GeneratedBuiltinModule {
            class_name: inner_class_name,
            module_name,
        });
    }

    let out_path = gen_path.join("mod.rs");
    let mod_contents = make_builtin_module_file(modules);

    submit_fn(out_path, mod_contents);
}

pub(crate) fn generate_native_structures_files(
    api: &ExtensionApi,
    ctx: &mut Context,
    _build_config: &str,
    gen_path: &Path,
    submit_fn: &mut SubmitFn,
) {
    let _ = std::fs::remove_dir_all(gen_path);
    std::fs::create_dir_all(gen_path).expect("create native directory");

    let mut modules = vec![];
    for native_structure in api.native_structures.iter() {
        let module_name = ModName::from_godot(&native_structure.name);
        let class_name = TyName::from_godot(&native_structure.name);

        let generated_class = make_native_structure(native_structure, &class_name, ctx);
        let file_contents = generated_class.code;

        let out_path = gen_path.join(format!("{}.rs", module_name.rust_mod));

        submit_fn(out_path, file_contents);

        modules.push(GeneratedBuiltinModule {
            class_name,
            module_name,
        });
    }

    let out_path = gen_path.join("mod.rs");
    let mod_contents = make_builtin_module_file(modules);

    submit_fn(out_path, mod_contents);
}

fn make_class_doc(
    class_name: &TyName,
    base_ident_opt: Option<Ident>,
    has_notification_enum: bool,
    has_sidecar_module: bool,
) -> String {
    let TyName { rust_ty, godot_ty } = class_name;

    let inherits_line = if let Some(base) = base_ident_opt {
        format!("Inherits [`{base}`][crate::engine::{base}].")
    } else {
        "This is the base class for all other classes at the root of the hierarchy. \
        Every instance of `Object` can be stored in a [`Gd`][crate::obj::Gd] smart pointer."
            .to_string()
    };

    let notify_line = if has_notification_enum {
        format!("* [`{rust_ty}Notification`][crate::engine::notify::{rust_ty}Notification]: notification type\n")
    } else {
        String::new()
    };

    let sidecar_line = if has_sidecar_module {
        let module_name = ModName::from_godot(&class_name.godot_ty).rust_mod;
        format!("* [`{module_name}`][crate::engine::{module_name}]: sidecar module with related enum/flag types\n")
    } else {
        String::new()
    };

    let online_link = format!(
        "https://docs.godotengine.org/en/stable/classes/class_{}.html",
        godot_ty.to_ascii_lowercase()
    );

    format!(
        "Godot class `{godot_ty}.`\n\n\
        \
        {inherits_line}\n\n\
        \
        Related symbols:\n\n\
        {sidecar_line}\
        * [`{rust_ty}Virtual`][crate::engine::{rust_ty}Virtual]: virtual methods\n\
        {notify_line}\
        \n\n\
        See also [Godot docs for `{godot_ty}`]({online_link}).\n\n",
    )
}

fn make_virtual_trait_doc(class_name: &TyName) -> String {
    let TyName { rust_ty, godot_ty } = class_name;

    let online_link = format!(
        "https://docs.godotengine.org/en/stable/classes/class_{}.html#methods",
        godot_ty.to_ascii_lowercase()
    );

    format!(
        "Virtual methods for class [`{rust_ty}`][crate::engine::{rust_ty}].\
        \n\n\
        These methods represent constructors (`init`) or callbacks invoked by the engine.\
        \n\n\
        See also [Godot docs for `{godot_ty}` methods]({online_link}).\n\n"
    )
}

fn make_module_doc(class_name: &TyName) -> String {
    let TyName { rust_ty, godot_ty } = class_name;

    let online_link = format!(
        "https://docs.godotengine.org/en/stable/classes/class_{}.html#enumerations",
        godot_ty.to_ascii_lowercase()
    );

    format!(
        "Sidecar module for class [`{rust_ty}`][crate::engine::{rust_ty}].\
        \n\n\
        Defines related flag and enum types. In GDScript, those are nested under the class scope.\
        \n\n\
        See also [Godot docs for `{godot_ty}` enums]({online_link}).\n\n"
    )
}

fn make_constructor(class: &Class, ctx: &Context) -> TokenStream {
    let godot_class_name = &class.name;
    if ctx.is_singleton(godot_class_name) {
        // Note: we cannot return &'static mut Self, as this would be very easy to mutably alias.
        // &'static Self would be possible, but we would lose the whole mutability information (even if that is best-effort and
        // not strict Rust mutability, it makes the API much more usable).
        // As long as the user has multiple Gd smart pointers to the same singletons, only the internal raw pointers are aliased.
        // See also Deref/DerefMut impl for Gd.
        quote! {
            pub fn singleton() -> Gd<Self> {
                unsafe {
                    let __class_name = StringName::from(#godot_class_name);
                    let __object_ptr = sys::interface_fn!(global_get_singleton)(__class_name.string_sys());
                    Gd::from_obj_sys(__object_ptr)
                }
            }
        }
    } else if !class.is_instantiable {
        // Abstract base classes or non-singleton classes without constructor
        TokenStream::new()
    } else if class.is_refcounted {
        // RefCounted, Resource, etc
        quote! {
            pub fn new() -> Gd<Self> {
                unsafe {
                    let __class_name = StringName::from(#godot_class_name);
                    let __object_ptr = sys::interface_fn!(classdb_construct_object)(__class_name.string_sys());
                    //let instance = Self { object_ptr };
                    Gd::from_obj_sys(__object_ptr)
                }
            }
        }
    } else {
        // Manually managed classes: Object, Node etc
        quote! {
            #[must_use]
            pub fn new_alloc() -> Gd<Self> {
                unsafe {
                    let __class_name = StringName::from(#godot_class_name);
                    let __object_ptr = sys::interface_fn!(classdb_construct_object)(__class_name.string_sys());
                    Gd::from_obj_sys(__object_ptr)
                }
            }
        }
    }
}

fn make_class(class: &Class, class_name: &TyName, ctx: &mut Context) -> GeneratedClass {
    // Strings
    let godot_class_str = &class_name.godot_ty;
    let virtual_trait_str = class_name.virtual_trait_name();

    // Idents and tokens
    let (base_ty, base_ident_opt) = match class.inherits.as_ref() {
        Some(base) => {
            let base = ident(&to_pascal_case(base));
            (quote! { crate::engine::#base }, Some(base))
        }
        None => (quote! { () }, None),
    };

    let constructor = make_constructor(class, ctx);
    let FnDefinitions {
        functions: methods,
        builders,
    } = make_methods(option_as_slice(&class.methods), class_name, ctx);

    let enums = make_enums(option_as_slice(&class.enums), class_name, ctx);
    let constants = make_constants(option_as_slice(&class.constants), class_name, ctx);
    let inherits_macro = format_ident!("inherits_transitive_{}", class_name.rust_ty);

    let (exportable_impl, exportable_macro_impl) = if ctx.is_exportable(class_name) {
        (
            quote! {
                impl crate::obj::ExportableObject for #class_name {}
            },
            quote! {
                impl ::godot::obj::ExportableObject for $Class {}
            },
        )
    } else {
        (TokenStream::new(), TokenStream::new())
    };

    let all_bases = ctx.inheritance_tree().collect_all_bases(class_name);
    let (notification_enum, notification_enum_name) =
        make_notification_enum(class_name, &all_bases, ctx);
    let has_sidecar_module = !enums.is_empty();
    let class_doc = make_class_doc(
        class_name,
        base_ident_opt,
        notification_enum.is_some(),
        has_sidecar_module,
    );
    let module_doc = make_module_doc(class_name);
    let virtual_trait = make_virtual_methods_trait(
        class,
        class_name,
        &all_bases,
        &virtual_trait_str,
        &notification_enum_name,
        ctx,
    );
    let notify_method = make_notify_method(class_name, ctx);

    let memory = if class_name.rust_ty == "Object" {
        ident("DynamicRefCount")
    } else if class.is_refcounted {
        ident("StaticRefCount")
    } else {
        ident("ManualMemory")
    };

    // mod re_export needed, because class should not appear inside the file module, and we can't re-export private struct as pub
    let tokens = quote! {
        #![doc = #module_doc]

        use godot_ffi as sys;
        use crate::engine::notify::*;
        use crate::builtin::*;
        use crate::engine::native::*;
        use crate::obj::Gd;
        use sys::GodotFfi as _;
        use std::ffi::c_void;

        pub(super) mod re_export {
            use super::*;

            #[doc = #class_doc]
            #[derive(Debug)]
            #[repr(transparent)]
            pub struct #class_name {
                object_ptr: sys::GDExtensionObjectPtr,
            }
            #virtual_trait
            #notification_enum
            impl #class_name {
                #constructor
                #notify_method
                #methods
                #constants
            }
            unsafe impl crate::obj::GodotClass for #class_name {
                type Base = #base_ty;
                type Declarer = crate::obj::dom::EngineDomain;
                type Mem = crate::obj::mem::#memory;

                const CLASS_NAME: &'static str = #godot_class_str;
            }
            impl crate::obj::EngineClass for #class_name {
                 fn as_object_ptr(&self) -> sys::GDExtensionObjectPtr {
                     self.object_ptr
                 }
                 fn as_type_ptr(&self) -> sys::GDExtensionTypePtr {
                    std::ptr::addr_of!(self.object_ptr) as sys::GDExtensionTypePtr
                 }
            }
            #(
                impl crate::obj::Inherits<crate::engine::#all_bases> for #class_name {}
            )*

            #exportable_impl

            impl std::ops::Deref for #class_name {
                type Target = #base_ty;

                fn deref(&self) -> &Self::Target {
                    // SAFETY: same assumptions as `impl Deref for Gd<T>`, see there for comments
                    unsafe { std::mem::transmute::<&Self, &Self::Target>(self) }
                }
            }
            impl std::ops::DerefMut for #class_name {
                fn deref_mut(&mut self) -> &mut Self::Target {
                    // SAFETY: see above
                    unsafe { std::mem::transmute::<&mut Self, &mut Self::Target>(self) }
                }
            }

            #[macro_export]
            #[allow(non_snake_case)]
            macro_rules! #inherits_macro {
                ($Class:ident) => {
                    impl ::godot::obj::Inherits<::godot::engine::#class_name> for $Class {}
                    #(
                        impl ::godot::obj::Inherits<::godot::engine::#all_bases> for $Class {}
                    )*
                    #exportable_macro_impl
                }
            }
        }

        #builders
        #enums
    };
    // note: TypePtr -> ObjectPtr conversion OK?

    GeneratedClass {
        code: tokens,
        notification_enum_name,
        has_own_notification_enum: notification_enum.is_some(),
        inherits_macro_ident: inherits_macro,
        has_sidecar_module,
    }
}

fn make_notify_method(class_name: &TyName, ctx: &mut Context) -> TokenStream {
    let enum_name = ctx.notification_enum_name(class_name);

    quote! {
        /// ⚠️ Sends a Godot notification to all classes inherited by the object.
        ///
        /// Triggers calls to `on_notification()`, and depending on the notification, also to Godot's lifecycle callbacks such as `ready()`.
        ///
        /// Starts from the highest ancestor (the `Object` class) and goes down the hierarchy.
        /// See also [Godot docs for `Object::notification()`](https://docs.godotengine.org/en/latest/classes/class_object.html#id3).
        ///
        /// # Panics
        ///
        /// If you call this method on a user-defined object while holding a `GdRef` or `GdMut` guard on the instance, you will encounter
        /// a panic. The reason is that the receiving virtual method `on_notification()` acquires a `GdMut` lock dynamically, which must
        /// be exclusive.
        pub fn notify(&mut self, what: #enum_name) {
            self.notification(i32::from(what), false);
        }

        /// ⚠️ Like [`Self::notify()`], but starts at the most-derived class and goes up the hierarchy.
        ///
        /// See docs of that method, including the panics.
        pub fn notify_reversed(&mut self, what: #enum_name) {
            self.notification(i32::from(what), true);
        }
    }
}

fn make_notification_enum(
    class_name: &TyName,
    all_bases: &Vec<TyName>,
    ctx: &mut Context,
) -> (Option<TokenStream>, Ident) {
    let Some(all_constants) = ctx.notification_constants(class_name) else {
        // Class has no notification constants: reuse (direct/indirect) base enum
        return (None, ctx.notification_enum_name(class_name));
    };

    // Collect all notification constants from current and base classes
    let mut all_constants = all_constants.clone();
    for base_name in all_bases {
        if let Some(constants) = ctx.notification_constants(base_name) {
            all_constants.extend(constants.iter().cloned());
        }
    }

    workaround_constant_collision(&mut all_constants);

    let enum_name = ctx.notification_enum_name(class_name);
    let doc_str = format!(
        "Notification type for class [`{c}`][crate::engine::{c}].",
        c = class_name.rust_ty
    );

    let mut notification_enumerators_pascal = Vec::new();
    let mut notification_enumerators_ord = Vec::new();
    for (constant_ident, constant_value) in all_constants {
        notification_enumerators_pascal.push(constant_ident);
        notification_enumerators_ord.push(util::make_enumerator_ord(constant_value));
    }

    let code = quote! {
        #[doc = #doc_str]
        ///
        /// Makes it easier to keep an overview all possible notification variants for a given class, including
        /// notifications defined in base classes.
        #[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
        #[repr(i32)]
        pub enum #enum_name {
            #(
                #notification_enumerators_pascal = #notification_enumerators_ord,
            )*

            /// Since Godot represents notifications as integers, it's always possible that a notification outside the known types
            /// is received. For example, the user can manually issue notifications through `Object.notification()`.
            Unknown(i32),
        }

        impl From<i32> for #enum_name {
            /// Always succeeds, mapping unknown integers to the `Unknown` variant.
            fn from(enumerator: i32) -> Self {
                match enumerator {
                    #(
                        #notification_enumerators_ord => Self::#notification_enumerators_pascal,
                    )*
                    other_int => Self::Unknown(other_int),
                }
            }
        }

        impl From<#enum_name> for i32 {
            fn from(notification: #enum_name) -> i32 {
                match notification {
                    #(
                        #enum_name::#notification_enumerators_pascal => #notification_enumerators_ord,
                    )*
                    #enum_name::Unknown(int) => int,
                }
            }
        }
    };

    (Some(code), enum_name)
}

/// Workaround for Godot bug https://github.com/godotengine/godot/issues/75839
///
/// Godot has a collision for two notification constants (DRAW, NODE_CACHE_REQUESTED) in the same inheritance branch (as of 4.0.2).
/// This cannot be represented in a Rust enum, so we merge the two constants into a single enumerator.
fn workaround_constant_collision(all_constants: &mut Vec<(Ident, i32)>) {
    for first in ["Draw", "VisibilityChanged"] {
        if let Some(index_of_draw) = all_constants
            .iter()
            .position(|(constant_name, _)| constant_name == first)
        {
            all_constants[index_of_draw].0 = format_ident!("{first}OrNodeRecacheRequested");
            all_constants.retain(|(constant_name, _)| constant_name != "NodeRecacheRequested");
        }
    }
}

fn make_builtin_class(
    class: &BuiltinClass,
    class_name: &TyName,
    inner_class_name: &TyName,
    type_info: &BuiltinTypeInfo,
    ctx: &mut Context,
) -> GeneratedBuiltin {
    let outer_class = if let RustTy::BuiltinIdent(ident) = to_rust_type(&class.name, None, ctx) {
        ident
    } else {
        panic!("Rust type `{}` categorized wrong", class.name)
    };
    let inner_class = &inner_class_name.rust_ty;

    let class_enums = class.enums.as_ref().map_or(Vec::new(), |class_enums| {
        class_enums
            .iter()
            .map(BuiltinClassEnum::to_enum)
            .collect::<Vec<Enum>>()
    });

    let FnDefinitions {
        functions: methods,
        builders,
    } = make_builtin_methods(
        option_as_slice(&class.methods),
        class_name,
        inner_class_name,
        type_info,
        ctx,
    );

    let enums = make_enums(&class_enums, class_name, ctx);
    let special_constructors = make_special_builtin_methods(class_name, ctx);

    // mod re_export needed, because class should not appear inside the file module, and we can't re-export private struct as pub
    let code = quote! {
        use godot_ffi as sys;
        use crate::builtin::*;
        use crate::engine::native::*;
        use crate::obj::Gd;
        use crate::sys::GodotFfi as _;
        use crate::engine::Object;

        #[repr(transparent)]
        pub struct #inner_class<'a> {
            _outer_lifetime: std::marker::PhantomData<&'a ()>,
            sys_ptr: sys::GDExtensionTypePtr,
        }
        impl<'a> #inner_class<'a> {
            pub fn from_outer(outer: &#outer_class) -> Self {
                Self {
                    _outer_lifetime: std::marker::PhantomData,
                    sys_ptr: outer.sys(),
                }
            }
            #special_constructors
            #methods
        }

        #builders
        #enums
    };
    // note: TypePtr -> ObjectPtr conversion OK?

    GeneratedBuiltin { code }
}

fn make_native_structure(
    structure: &NativeStructure,
    class_name: &TyName,
    ctx: &mut Context,
) -> GeneratedBuiltin {
    let class_name = &class_name.rust_ty;

    let fields = make_native_structure_fields(&structure.format, ctx);

    // mod re_export needed, because class should not appear inside the file module, and we can't re-export private struct as pub
    let tokens = quote! {
        use godot_ffi as sys;
        use crate::builtin::*;
        use crate::engine::native::*;
        use crate::obj::Gd;
        use crate::sys::GodotFfi as _;
        use crate::engine::Object;

        #[repr(C)]
        pub struct #class_name {
            #fields
        }
    };
    // note: TypePtr -> ObjectPtr conversion OK?

    GeneratedBuiltin { code: tokens }
}

fn make_native_structure_fields(format_str: &str, ctx: &mut Context) -> TokenStream {
    let fields = parse_native_structures_format(format_str)
        .expect("Could not parse native_structures format field");
    let field_definitions = fields
        .into_iter()
        .map(|field| make_native_structure_field_definition(field, ctx));
    quote! {
        #( #field_definitions )*
    }
}

fn make_native_structure_field_definition(
    field: NativeStructuresField,
    ctx: &mut Context,
) -> TokenStream {
    let field_type = normalize_native_structure_field_type(&field.field_type);
    let field_type = to_rust_type_abi(&field_type, ctx);
    let field_name = ident(&to_snake_case(&field.field_name));
    quote! {
        pub #field_name: #field_type,
    }
}

fn normalize_native_structure_field_type(field_type: &str) -> String {
    // native_structures uses a different format for enums than the
    // rest of the JSON file. If we detect a scoped field, convert it
    // to the enum format expected by to_rust_type.
    if field_type.contains("::") {
        let with_dot = field_type.replace("::", ".");
        format!("enum::{}", with_dot)
    } else {
        field_type.to_string()
    }
}

fn make_module_file(classes_and_modules: Vec<GeneratedClassModule>) -> TokenStream {
    let mut class_decls = Vec::new();
    let mut notify_decls = Vec::new();

    for m in classes_and_modules.iter() {
        let GeneratedClassModule {
            module_name,
            class_name,
            own_notification_enum_name,
            is_pub_sidecar: is_pub,
            ..
        } = m;
        let virtual_trait_name = ident(&class_name.virtual_trait_name());

        let vis = is_pub.then_some(quote! { pub });

        let class_decl = quote! {
            #vis mod #module_name;
            pub use #module_name::re_export::#class_name;
            pub use #module_name::re_export::#virtual_trait_name;
        };
        class_decls.push(class_decl);

        if let Some(enum_name) = own_notification_enum_name {
            let notify_decl = quote! {
                pub use super::#module_name::re_export::#enum_name;
            };

            notify_decls.push(notify_decl);
        }
    }

    let macros = classes_and_modules.iter().map(|m| {
        let GeneratedClassModule {
            inherits_macro_ident,
            ..
        } = m;

        // We cannot re-export the following, because macro is in the crate root
        // pub use #module_ident::re_export::#inherits_macro_ident;
        quote! {
            pub use #inherits_macro_ident;
        }
    });

    quote! {
        #( #class_decls )*

        pub mod notify {
            #( #notify_decls )*
        }

        #[doc(hidden)]
        pub mod class_macros {
            pub use crate::*;
            #( #macros )*
        }
    }
}

fn make_builtin_module_file(classes_and_modules: Vec<GeneratedBuiltinModule>) -> TokenStream {
    let decls = classes_and_modules.iter().map(|m| {
        let GeneratedBuiltinModule {
            module_name,
            class_name,
            ..
        } = m;

        quote! {
            mod #module_name;
            pub use #module_name::#class_name;
        }
    });

    quote! {
        #( #decls )*
    }
}

fn make_methods(methods: &[ClassMethod], class_name: &TyName, ctx: &mut Context) -> FnDefinitions {
    let definitions = methods
        .iter()
        .map(|method| make_method_definition(method, class_name, ctx));

    FnDefinitions::expand(definitions)
}

fn make_builtin_methods(
    methods: &[BuiltinClassMethod],
    class_name: &TyName,
    inner_class_name: &TyName,
    type_info: &BuiltinTypeInfo,
    ctx: &mut Context,
) -> FnDefinitions {
    let definitions = methods.iter().map(|method| {
        make_builtin_method_definition(method, class_name, inner_class_name, type_info, ctx)
    });

    FnDefinitions::expand(definitions)
}

fn make_enums(enums: &[Enum], _class_name: &TyName, _ctx: &Context) -> TokenStream {
    let definitions = enums.iter().map(util::make_enum_definition);

    quote! {
        #( #definitions )*
    }
}

fn make_constants(
    constants: &[ClassConstant],
    _class_name: &TyName,
    _ctx: &Context,
) -> TokenStream {
    let definitions = constants.iter().map(util::make_constant_definition);

    quote! {
        #( #definitions )*
    }
}

/// Depending on the built-in class, adds custom constructors and methods.
fn make_special_builtin_methods(class_name: &TyName, _ctx: &Context) -> TokenStream {
    if class_name.godot_ty == "Array" {
        quote! {
            pub fn from_outer_typed<T>(outer: &Array<T>) -> Self
                where T: crate::builtin::meta::VariantMetadata
            {
                Self {
                    _outer_lifetime: std::marker::PhantomData,
                    sys_ptr: outer.sys(),
                }
            }
        }
    } else {
        TokenStream::new()
    }
}

#[cfg(not(feature = "codegen-full"))]
fn is_type_excluded(ty: &str, ctx: &mut Context) -> bool {
    fn is_class_excluded(class: &str) -> bool {
        !crate::SELECTED_CLASSES.contains(&class)
    }

    fn is_rust_type_excluded(ty: &RustTy) -> bool {
        match ty {
            RustTy::BuiltinIdent(_) => false,
            RustTy::BuiltinArray(_) => false,
            RustTy::RawPointer { inner, .. } => is_rust_type_excluded(&inner),
            RustTy::EngineArray { elem_class, .. } => is_class_excluded(elem_class.as_str()),
            RustTy::EngineEnum {
                surrounding_class, ..
            } => match surrounding_class.as_ref() {
                None => false,
                Some(class) => is_class_excluded(class.as_str()),
            },
            RustTy::EngineClass { class, .. } => is_class_excluded(&class),
        }
    }
    is_rust_type_excluded(&to_rust_type(ty, None, ctx))
}

fn is_method_excluded(
    method: &ClassMethod,
    is_virtual_impl: bool,
    #[allow(unused_variables)] ctx: &mut Context,
) -> bool {
    // Currently excluded:
    //
    // * Private virtual methods are only included in a virtual
    //   implementation.

    // -- FIXME remove when impl complete
    #[cfg(not(feature = "codegen-full"))]
    if method
        .return_value
        .as_ref()
        .map_or(false, |ret| is_type_excluded(ret.type_.as_str(), ctx))
        || method.arguments.as_ref().map_or(false, |args| {
            args.iter()
                .any(|arg| is_type_excluded(arg.type_.as_str(), ctx))
        })
    {
        return true;
    }
    // -- end.

    if method.name.starts_with('_') && !is_virtual_impl {
        return true;
    }

    false
}

#[cfg(feature = "codegen-full")]
fn is_function_excluded(_function: &UtilityFunction, _ctx: &mut Context) -> bool {
    false
}

#[cfg(not(feature = "codegen-full"))]
fn is_function_excluded(function: &UtilityFunction, ctx: &mut Context) -> bool {
    function
        .return_type
        .as_ref()
        .map_or(false, |ret| is_type_excluded(ret.as_str(), ctx))
        || function.arguments.as_ref().map_or(false, |args| {
            args.iter()
                .any(|arg| is_type_excluded(arg.type_.as_str(), ctx))
        })
}

fn make_method_definition(
    method: &ClassMethod,
    class_name: &TyName,
    ctx: &mut Context,
) -> FnDefinition {
    if is_method_excluded(method, false, ctx) || special_cases::is_deleted(class_name, &method.name)
    {
        return FnDefinition::none();
    }
    /*if method.map_args(|args| args.is_empty()) {
        // Getters (i.e. 0 arguments) will be stripped of their `get_` prefix, to conform to Rust convention
        if let Some(remainder) = method_name.strip_prefix("get_") {
            // TODO Do not apply for FileAccess::get_16, StreamPeer::get_u16, etc
            if !remainder.chars().nth(0).unwrap().is_ascii_digit() {
                method_name = remainder;
            }
        }
    }*/

    let method_name_str = special_cases::maybe_renamed(class_name, &method.name);

    let receiver = make_receiver(
        method.is_static,
        method.is_const,
        quote! { self.object_ptr },
    );

    let hash = method.hash;
    let is_varcall = method.is_vararg;

    let variant_ffi = is_varcall.then(VariantFfi::variant_ptr);
    let function_provider = if is_varcall {
        ident("object_method_bind_call")
    } else {
        ident("object_method_bind_ptrcall")
    };

    let class_name_str = &class_name.godot_ty;
    let init_code = quote! {
        let __class_name = StringName::from(#class_name_str);
        let __method_name = StringName::from(#method_name_str);
        let __method_bind = sys::interface_fn!(classdb_get_method_bind)(
            __class_name.string_sys(),
            __method_name.string_sys(),
            #hash
        );
        assert!(
            !__method_bind.is_null(),
            "failed to load method {}::{} (hash {}) -- possible Godot/gdext version mismatch",
            #class_name_str,
            #method_name_str,
            #hash
        );
        let __call_fn = sys::interface_fn!(#function_provider);
    };

    let receiver_ffi_arg = &receiver.ffi_arg;
    let varcall_invocation = quote! {
        __call_fn(__method_bind, #receiver_ffi_arg, __args_ptr, __args.len() as i64, return_ptr, std::ptr::addr_of_mut!(__err));
    };
    let ptrcall_invocation = quote! {
        __call_fn(__method_bind, #receiver_ffi_arg, __args_ptr, return_ptr);
    };

    make_function_definition(
        &FnSignature {
            function_name: method_name_str,
            surrounding_class: Some(class_name),
            is_private: special_cases::is_private(class_name, &method.name),
            is_virtual: false,
            qualifier: FnQualifier::for_method(method.is_const, method.is_static),
            params: FnParam::new_range(&method.arguments, ctx),
            return_value: FnReturn::new(&method.return_value, ctx),
        },
        &FnCode {
            receiver,
            variant_ffi,
            init_code,
            varcall_invocation,
            ptrcall_invocation,
        },
    )
}

fn make_builtin_method_definition(
    method: &BuiltinClassMethod,
    class_name: &TyName,
    inner_class_name: &TyName,
    type_info: &BuiltinTypeInfo,
    ctx: &mut Context,
) -> FnDefinition {
    let method_name_str = &method.name;

    let return_value = method
        .return_type
        .as_deref()
        .map(MethodReturn::from_type_no_meta);
    let hash = method.hash.expect("missing hash for builtin method");
    let is_varcall = method.is_vararg;
    let variant_ffi = is_varcall.then(VariantFfi::type_ptr);

    let variant_type = &type_info.type_names.sys_variant_type;
    let init_code = quote! {
        let __variant_type = sys::#variant_type;
        let __method_name = StringName::from(#method_name_str);
        let __call_fn = sys::interface_fn!(variant_get_ptr_builtin_method)(
            __variant_type,
            __method_name.string_sys(),
            #hash
        );
        let __call_fn = __call_fn.unwrap_unchecked();
    };

    let receiver = make_receiver(method.is_static, method.is_const, quote! { self.sys_ptr });
    let receiver_ffi_arg = &receiver.ffi_arg;
    let ptrcall_invocation = quote! {
        __call_fn(#receiver_ffi_arg, __args_ptr, return_ptr, __args.len() as i32);
    };

    make_function_definition(
        &FnSignature {
            function_name: method_name_str,
            surrounding_class: Some(inner_class_name),
            is_private: special_cases::is_private(class_name, &method.name),
            is_virtual: false,
            qualifier: FnQualifier::for_method(method.is_const, method.is_static),

            // Disable default parameters for builtin classes.
            // They are not public-facing and need more involved implementation (lifetimes etc). Also reduces number of symbols in API.
            params: FnParam::new_range_no_defaults(&method.arguments, ctx),
            return_value: FnReturn::new(&return_value, ctx),
        },
        &FnCode {
            receiver,
            variant_ffi,
            init_code,
            varcall_invocation: ptrcall_invocation.clone(),
            ptrcall_invocation,
        },
    )
}

pub(crate) fn make_utility_function_definition(
    function: &UtilityFunction,
    ctx: &mut Context,
) -> TokenStream {
    if is_function_excluded(function, ctx) {
        return TokenStream::new();
    }

    let function_name_str = &function.name;
    let return_value = function
        .return_type
        .as_deref()
        .map(MethodReturn::from_type_no_meta);
    let hash = function.hash;
    let variant_ffi = function.is_vararg.then_some(VariantFfi::type_ptr());
    let init_code = quote! {
        let __function_name = StringName::from(#function_name_str);
        let __call_fn = sys::interface_fn!(variant_get_ptr_utility_function)(__function_name.string_sys(), #hash);
        let __call_fn = __call_fn.unwrap_unchecked();
    };
    let invocation = quote! {
        __call_fn(return_ptr, __args_ptr, __args.len() as i32);
    };

    let definition = make_function_definition(
        &FnSignature {
            function_name: function_name_str,
            surrounding_class: None,
            is_private: false,
            is_virtual: false,
            qualifier: FnQualifier::Global,
            params: FnParam::new_range(&function.arguments, ctx),
            return_value: FnReturn::new(&return_value, ctx),
        },
        &FnCode {
            receiver: FnReceiver::global_function(),
            variant_ffi,
            init_code,
            varcall_invocation: invocation.clone(),
            ptrcall_invocation: invocation,
        },
    );

    assert!(
        definition.builders.is_empty(),
        "utility functions should not have builders"
    );
    definition.functions
}

/// Defines which methods to use to convert between `Variant` and FFI (either variant ptr or type ptr)
struct VariantFfi {
    sys_method: Ident,
    from_sys_init_method: Ident,
}
impl VariantFfi {
    fn variant_ptr() -> Self {
        Self {
            sys_method: ident("var_sys_const"),
            from_sys_init_method: ident("from_var_sys_init"),
        }
    }
    fn type_ptr() -> Self {
        Self {
            sys_method: ident("sys_const"),
            from_sys_init_method: ident("from_sys_init_default"),
        }
    }
}

fn make_vis(is_private: bool) -> TokenStream {
    if is_private {
        quote! { pub(crate) }
    } else {
        quote! { pub }
    }
}

fn make_function_definition(sig: &FnSignature, code: &FnCode) -> FnDefinition {
    let has_default_params = function_uses_default_params(sig);
    let vis = if has_default_params {
        // Public API mapped by separate function.
        // Needs to be crate-public because default-arg builder lives outside of the module.
        quote! { pub(crate) }
    } else {
        make_vis(sig.is_private)
    };

    let (maybe_unsafe, safety_doc) = if function_uses_pointers(sig) {
        (
            quote! { unsafe },
            quote! {
                #[doc = "# Safety"]
                #[doc = ""]
                #[doc = "Godot currently does not document safety requirements on this method. Make sure you understand the underlying semantics."]
            },
        )
    } else {
        (TokenStream::new(), TokenStream::new())
    };

    let is_varcall = code.variant_ffi.is_some();
    let [params, variant_types, arg_exprs, arg_names] =
        make_params_and_impl(&sig.params, is_varcall, false);

    let primary_fn_name = if has_default_params {
        format_ident!("{}_full", safe_ident(sig.function_name))
    } else {
        safe_ident(sig.function_name)
    };

    let (default_fn_code, default_structs_code) = if has_default_params {
        make_function_definition_with_defaults(sig, code, &primary_fn_name)
    } else {
        (TokenStream::new(), TokenStream::new())
    };

    let args_indices = (0..arg_exprs.len()).map(Literal::usize_unsuffixed);

    let (prepare_arg_types, error_fn_context);
    if code.variant_ffi.is_some() {
        // varcall (using varargs)
        prepare_arg_types = quote! {
            let mut __arg_types = Vec::with_capacity(__explicit_args.len() + varargs.len());
            // __arg_types.extend(__explicit_args.iter().map(Variant::get_type));
            __arg_types.extend(varargs.iter().map(Variant::get_type));
            let __vararg_str = varargs.iter().map(|v| format!("{v}")).collect::<Vec<_>>().join(", ");
        };

        let joined = arg_names
            .iter()
            .map(|n| format!("{{{n}:?}}"))
            .collect::<Vec<_>>()
            .join(", ");

        let fmt = format!("{f}({joined}; {{__vararg_str}})", f = sig.function_name);
        error_fn_context = quote! { &format!(#fmt) };
    } else {
        // ptrcall
        prepare_arg_types = quote! {
            let __arg_types = [
                #( #variant_types ),*
            ];
        };
        error_fn_context = sig.function_name.to_token_stream();
    };

    let return_decl = &sig.return_value.decl;
    let call_code = make_return_and_impl(
        &sig.return_value,
        code,
        prepare_arg_types,
        error_fn_context,
        sig.is_virtual,
    );

    let receiver_param = &code.receiver.param;
    let primary_function = if sig.is_virtual {
        quote! {
            #safety_doc
            #maybe_unsafe fn #primary_fn_name(
                #receiver_param
                #( #params, )*
            ) #return_decl {
                #call_code
            }
        }
    } else if let Some(variant_ffi) = code.variant_ffi.as_ref() {
        // varcall (using varargs)
        let sys_method = &variant_ffi.sys_method;
        let init_code = &code.init_code;
        quote! {
            #safety_doc
            #vis #maybe_unsafe fn #primary_fn_name(
                #receiver_param
                #( #params, )*
                varargs: &[Variant]
            ) #return_decl {
                unsafe {
                    #init_code

                    let __explicit_args = [
                        #( #arg_exprs ),*
                    ];

                    let mut __args = Vec::with_capacity(__explicit_args.len() + varargs.len());
                    __args.extend(__explicit_args.iter().map(Variant::#sys_method));
                    __args.extend(varargs.iter().map(Variant::#sys_method));

                    let __args_ptr = __args.as_ptr();

                    #call_code
                }
            }
        }
    } else {
        // ptrcall
        let init_code = &code.init_code;
        quote! {
            #safety_doc
            #vis #maybe_unsafe fn #primary_fn_name(
                #receiver_param
                #( #params, )*
            ) #return_decl {
                unsafe {
                    #init_code

                    #[allow(clippy::let_unit_value)]
                    let __args = (
                        #( #arg_exprs, )*
                    );

                    let __args = [
                        #( sys::GodotFfi::as_arg_ptr(&__args.#args_indices) ),*
                    ];

                    let __args_ptr = __args.as_ptr();

                    #call_code
                }
            }
        }
    };

    FnDefinition {
        functions: quote! {
            #primary_function
            #default_fn_code
        },
        builders: default_structs_code,
    }
}

fn make_function_definition_with_defaults(
    sig: &FnSignature,
    code: &FnCode,
    full_fn_name: &Ident,
) -> (TokenStream, TokenStream) {
    let (default_fn_params, required_fn_params): (Vec<_>, Vec<_>) = sig
        .params
        .iter()
        .partition(|arg| arg.default_value.is_some());

    let simple_fn_name = safe_ident(sig.function_name);
    let extended_fn_name = format_ident!("{}_ex", simple_fn_name);
    let vis = make_vis(sig.is_private);

    let (builder_doc, surround_class_prefix) = make_extender_doc(sig, &extended_fn_name);

    let ExtenderReceiver {
        object_fn_param,
        object_param,
        object_arg,
    } = make_extender_receiver(sig);

    let Extender {
        builder_ty,
        builder_lifetime,
        builder_anon_lifetime,
        builder_methods,
        builder_fields,
        builder_args,
        builder_inits,
    } = make_extender(sig, object_fn_param, default_fn_params);

    let receiver_param = &code.receiver.param;
    let receiver_self = &code.receiver.self_prefix;
    let (required_params, required_args) = make_params_and_args(&required_fn_params);
    let return_decl = &sig.return_value.decl;

    // Technically, the builder would not need a lifetime -- it could just maintain an `object_ptr` copy.
    // However, this increases the risk that it is used out of place (not immediately for a default-param call).
    // Ideally we would require &mut, but then we would need `mut Gd<T>` objects everywhere.

    // #[allow] exceptions:
    // - wrong_self_convention:     to_*() and from_*() are taken from Godot
    // - redundant_field_names:     'value: value' is a possible initialization pattern
    // - needless-update:           '..self' has nothing left to change
    let builders = quote! {
        #[doc = #builder_doc]
        #[must_use]
        pub struct #builder_ty #builder_lifetime {
            // #builder_surround_ref
            #( #builder_fields, )*
        }

        #[allow(clippy::wrong_self_convention, clippy::redundant_field_names, clippy::needless_update)]
        impl #builder_lifetime #builder_ty #builder_lifetime {
            fn new(
                #object_param
                #( #required_params, )*
            ) -> Self {
                Self {
                    #( #builder_inits, )*
                }
            }

            #( #builder_methods )*

            #[inline]
            pub fn done(self) #return_decl {
                #surround_class_prefix #full_fn_name(
                    #( #builder_args, )*
                )
            }
        }
    };

    let functions = quote! {
        #[inline]
        #vis fn #simple_fn_name(
            #receiver_param
            #( #required_params, )*
        ) #return_decl {
            #receiver_self #extended_fn_name(
                #( #required_args, )*
            ).done()
        }

        #[inline]
        #vis fn #extended_fn_name(
            #receiver_param
            #( #required_params, )*
        ) -> #builder_ty #builder_anon_lifetime {
            #builder_ty::new(
                #object_arg
                #( #required_args, )*
            )
        }
    };

    (functions, builders)
}

fn make_extender_doc(sig: &FnSignature, extended_fn_name: &Ident) -> (String, TokenStream) {
    // Not in the above match, because this is true for both static/instance methods.
    // Static/instance is determined by first argument (always use fully qualified function call syntax).
    let surround_class_prefix;
    let builder_doc;

    match sig.surrounding_class {
        Some(TyName { rust_ty, .. }) => {
            surround_class_prefix = quote! { re_export::#rust_ty:: };
            builder_doc = format!(
                "Default-param extender for [`{class}::{method}`][super::{class}::{method}].",
                class = rust_ty,
                method = extended_fn_name,
            );
        }
        None => {
            // There are currently no default parameters for utility functions
            // -> this is currently dead code, but _should_ work if Godot ever adds them.
            surround_class_prefix = TokenStream::new();
            builder_doc = format!(
                "Default-param extender for [`{function}`][super::{function}].",
                function = extended_fn_name
            );
        }
    };

    (builder_doc, surround_class_prefix)
}

fn make_extender_receiver(sig: &FnSignature) -> ExtenderReceiver {
    let builder_mut = if matches!(sig.qualifier, FnQualifier::Const) {
        quote! {}
    } else {
        quote! { mut }
    };

    // Treat the object parameter like other parameters, as first in list.
    // Only add it if the method is not global or static.
    match &sig.surrounding_class {
        Some(surrounding_class) if !sig.qualifier.is_static_or_global() => {
            let class = &surrounding_class.rust_ty;

            ExtenderReceiver {
                object_fn_param: Some(FnParam {
                    name: ident("surround_object"),
                    // Not exactly EngineClass, but close enough
                    type_: RustTy::EngineClass {
                        tokens: quote! { &'a #builder_mut re_export::#class },
                        class: String::new(),
                    },
                    default_value: None,
                }),
                object_param: quote! { surround_object: &'a #builder_mut re_export::#class, },
                object_arg: quote! { self, },
            }
        }
        _ => ExtenderReceiver {
            object_fn_param: None,
            object_param: TokenStream::new(),
            object_arg: TokenStream::new(),
        },
    }
}

struct ExtenderReceiver {
    object_fn_param: Option<FnParam>,
    object_param: TokenStream,
    object_arg: TokenStream,
}

struct Extender {
    builder_ty: Ident,
    builder_lifetime: TokenStream,
    builder_anon_lifetime: TokenStream,
    builder_methods: Vec<TokenStream>,
    builder_fields: Vec<TokenStream>,
    builder_args: Vec<TokenStream>,
    builder_inits: Vec<TokenStream>,
}

fn make_extender(
    sig: &FnSignature,
    object_fn_param: Option<FnParam>,
    default_fn_params: Vec<&FnParam>,
) -> Extender {
    // Note: could build a documentation string with default values here, but the Rust tokens are not very readable,
    // and often not helpful, such as Enum::from_ord(13). Maybe one day those could be resolved and curated.

    let (lifetime, anon_lifetime) = if sig.qualifier.is_static_or_global() {
        (TokenStream::new(), TokenStream::new())
    } else {
        (quote! { <'a> }, quote! { <'_> })
    };

    let all_fn_params = object_fn_param.iter().chain(&sig.params);
    let len = all_fn_params.size_hint().0;

    let mut result = Extender {
        builder_ty: format_ident!("Ex{}", to_pascal_case(sig.function_name)),
        builder_lifetime: lifetime,
        builder_anon_lifetime: anon_lifetime,
        builder_methods: Vec::with_capacity(default_fn_params.len()),
        builder_fields: Vec::with_capacity(len),
        builder_args: Vec::with_capacity(len),
        builder_inits: Vec::with_capacity(len),
    };

    for param in all_fn_params {
        let FnParam {
            name,
            type_,
            default_value,
        } = param;

        // Initialize with default parameters where available, forward constructor args otherwise
        let init = if let Some(value) = default_value {
            quote! { #name: #value }
        } else {
            quote! { #name }
        };

        result.builder_fields.push(quote! { #name: #type_ });
        result.builder_args.push(quote! { self.#name });
        result.builder_inits.push(init);
    }

    for param in default_fn_params {
        let FnParam { name, type_, .. } = param;

        let method = quote! {
            #[inline]
            pub fn #name(self, value: #type_) -> Self {
                // Currently not testing whether the parameter was already set
                Self {
                    #name: value,
                    ..self
                }
            }
        };

        result.builder_methods.push(method);
    }

    result
}

fn make_receiver(is_static: bool, is_const: bool, ffi_arg: TokenStream) -> FnReceiver {
    // could reuse FnQualifier as parameter

    let param = if is_static {
        quote! {}
    } else if is_const {
        quote! { &self, }
    } else {
        quote! { &mut self, }
    };

    let ffi_arg = if is_static {
        quote! { std::ptr::null_mut() }
    } else {
        ffi_arg
    };

    let self_prefix = if is_static {
        quote! { Self:: }
    } else {
        quote! { self. }
    };

    FnReceiver {
        param,
        ffi_arg,
        self_prefix,
    }
}

fn make_params_and_impl(
    method_args: &[FnParam],
    is_varcall: bool,
    skip_defaults: bool,
) -> [Vec<TokenStream>; 4] {
    let mut params = vec![];
    let mut variant_types = vec![];
    let mut arg_exprs = vec![];
    let mut arg_names = vec![];

    for param in method_args.iter() {
        if skip_defaults && param.default_value.is_some() {
            continue;
        }

        let param_name = &param.name;
        let param_ty = &param.type_;

        let arg_expr = if is_varcall {
            quote! { <#param_ty as ToVariant>::to_variant(&#param_name) }
        } else if let RustTy::EngineClass { tokens: path, .. } = &param_ty {
            quote! { <#path as sys::GodotFuncMarshal>::try_into_via(#param_name).unwrap() }
        } else {
            quote! { <#param_ty as sys::GodotFuncMarshal>::try_into_via(#param_name).unwrap() }
        };

        params.push(quote! { #param_name: #param_ty });
        variant_types.push(quote! { <#param_ty as VariantMetadata>::variant_type() });
        arg_exprs.push(arg_expr);
        arg_names.push(quote! { #param_name });
    }

    [params, variant_types, arg_exprs, arg_names]
}

fn make_params_and_args(method_args: &[&FnParam]) -> (Vec<TokenStream>, Vec<TokenStream>) {
    method_args
        .iter()
        .map(|param| {
            let param_name = &param.name;
            let param_ty = &param.type_;

            (quote! { #param_name: #param_ty }, quote! { #param_name })
        })
        .unzip()
}

fn make_return_and_impl(
    return_value: &FnReturn,
    code: &FnCode,
    prepare_arg_types: TokenStream,
    error_fn_context: TokenStream, // only for panic message
    is_virtual: bool,
) -> TokenStream {
    let return_ty = &return_value.type_;

    // Virtual methods
    if is_virtual {
        return quote! { unimplemented!() };
    }

    // Varcall
    if let Some(variant_ffi) = &code.variant_ffi {
        // If the return type is not Variant, then convert to concrete target type
        let return_expr = match return_ty {
            None => TokenStream::new(), // unit return
            Some(RustTy::BuiltinIdent(ident)) if ident == "Variant" => quote! { __variant },
            Some(_) => quote! { __variant.to() },
        };
        let from_sys_init_method = &variant_ffi.from_sys_init_method;
        let varcall_invocation = &code.varcall_invocation;

        // Note: __err may remain unused if the #call does not handle errors (e.g. utility fn, ptrcall, ...).
        //       __variant remains unused if the function returns unit.
        // TODO use Result instead of panic on error
        return quote! {
            let __variant = Variant::#from_sys_init_method(|return_ptr| {
                let mut __err = sys::default_call_error();
                #varcall_invocation
                if __err.error != sys::GDEXTENSION_CALL_OK {
                    #prepare_arg_types
                    sys::panic_call_error(&__err, #error_fn_context, &__arg_types);
                }
            });
            #return_expr
        };
    }

    // Ptrcall
    let ptrcall_invocation = &code.ptrcall_invocation;
    match return_ty {
        Some(RustTy::EngineClass { tokens, .. }) => {
            let return_ty = tokens;
            quote! {
                <#return_ty>::from_sys_init_opt(|return_ptr| {
                    #ptrcall_invocation
                })
            }
        }
        Some(return_ty) => {
            quote! {
                let via = <<#return_ty as sys::GodotFuncMarshal>::Via as sys::GodotFfi>::from_sys_init_default(|return_ptr| {
                    #ptrcall_invocation
                });
                <#return_ty as sys::GodotFuncMarshal>::try_from_via(via).unwrap()
            }
        }
        None => {
            quote! {
                let return_ptr = std::ptr::null_mut();
                #ptrcall_invocation
            }
        }
    }
}

fn make_virtual_methods_trait(
    class: &Class,
    class_name: &TyName,
    all_base_names: &[TyName],
    trait_name: &str,
    notification_enum_name: &Ident,
    ctx: &mut Context,
) -> TokenStream {
    let trait_name = ident(trait_name);

    let virtual_method_fns = make_all_virtual_methods(class, all_base_names, ctx);
    let special_virtual_methods = special_virtual_methods(notification_enum_name);

    let trait_doc = make_virtual_trait_doc(class_name);

    quote! {
        #[doc = #trait_doc]
        #[allow(unused_variables)]
        #[allow(clippy::unimplemented)]
        pub trait #trait_name: crate::obj::GodotClass + crate::private::You_forgot_the_attribute__godot_api {
            #special_virtual_methods
            #( #virtual_method_fns )*
        }
    }
}

fn special_virtual_methods(notification_enum_name: &Ident) -> TokenStream {
    quote! {
        #[doc(hidden)]
        fn register_class(builder: &mut crate::builder::ClassBuilder<Self>) {
            unimplemented!()
        }

        /// Godot constructor, accepting an injected `base` object.
        ///
        /// `base` refers to the base instance of the class, which can either be stored in a `#[base]` field or discarded.
        /// This method returns a fully-constructed instance, which will then be moved into a [`Gd<T>`][crate::obj::Gd] pointer.
        ///
        /// If the class has a `#[class(init)]` attribute, this method will be auto-generated and must not be overridden.
        fn init(base: crate::obj::Base<Self::Base>) -> Self {
            unimplemented!()
        }

        /// String representation of the Godot instance.
        ///
        /// Override this method to define how the instance is represented as a string.
        /// Used by `impl Display for Gd<T>`, as well as `str()` and `print()` in GDScript.
        fn to_string(&self) -> crate::builtin::GodotString {
            unimplemented!()
        }

        /// Called when the object receives a Godot notification.
        ///
        /// The type of notification can be identified through `what`. The enum is designed to hold all possible `NOTIFICATION_*`
        /// constants that the current class can handle. However, this is not validated in Godot, so an enum variant `Unknown` exists
        /// to represent integers out of known constants (mistakes or future additions).
        ///
        /// This method is named `_notification` in Godot, but `on_notification` in Rust. To _send_ notifications, use the
        /// [`Object::notify`][crate::engine::Object::notify] method.
        ///
        /// See also in Godot docs:
        /// * [`Object::_notification`](https://docs.godotengine.org/en/stable/classes/class_object.html#class-object-method-notification).
        /// * [Notifications tutorial](https://docs.godotengine.org/en/stable/tutorials/best_practices/godot_notifications.html).
        fn on_notification(&mut self, what: #notification_enum_name) {
            unimplemented!()
        }
    }
}

fn make_virtual_method(method: &ClassMethod, ctx: &mut Context) -> TokenStream {
    let method_name = virtual_method_name(method);

    // Virtual methods are never static.
    assert!(!method.is_static);

    let definition = make_function_definition(
        &FnSignature {
            function_name: method_name,
            surrounding_class: None, // no default parameters needed for virtual methods
            is_private: false,
            is_virtual: true,
            qualifier: FnQualifier::for_method(method.is_const, method.is_static),
            params: FnParam::new_range(&method.arguments, ctx),
            return_value: FnReturn::new(&method.return_value, ctx),
        },
        &FnCode {
            receiver: make_receiver(false, method.is_const, TokenStream::new()),
            // make_return() requests following args, but they are not used for virtual methods. We can provide empty streams.
            variant_ffi: None,
            init_code: TokenStream::new(),
            varcall_invocation: TokenStream::new(),
            ptrcall_invocation: TokenStream::new(),
        },
    );

    // Virtual methods have no builders.
    definition.functions
}

fn make_all_virtual_methods(
    class: &Class,
    all_base_names: &[TyName],
    ctx: &mut Context,
) -> Vec<TokenStream> {
    let mut all_virtuals = vec![];
    let mut extend_virtuals = |class| {
        all_virtuals.extend(
            get_methods_in_class(class)
                .iter()
                .cloned()
                .filter(|m| m.is_virtual),
        );
    };

    // Get virtuals defined on the current class.
    extend_virtuals(class);

    // Add virtuals from superclasses.
    for base in all_base_names {
        let superclass = ctx.get_engine_class(base);
        extend_virtuals(superclass);
    }

    all_virtuals
        .into_iter()
        .filter_map(|method| {
            if is_method_excluded(&method, true, ctx) {
                None
            } else {
                Some(make_virtual_method(&method, ctx))
            }
        })
        .collect()
}

fn get_methods_in_class(class: &Class) -> &[ClassMethod] {
    match &class.methods {
        None => &[],
        Some(methods) => methods,
    }
}

fn virtual_method_name(class_method: &ClassMethod) -> &str {
    // Matching the C++ convention, we remove the leading underscore
    // from virtual method names.
    let method_name = class_method
        .name
        .strip_prefix('_')
        .unwrap_or(&class_method.name);

    // As a special exception, a few classes define a virtual method
    // called "_init" (distinct from the constructor), so we rename
    // those to avoid a name conflict in our trait.
    if method_name == "init" {
        "init_ext"
    } else {
        method_name
    }
}

fn function_uses_pointers(sig: &FnSignature) -> bool {
    let has_pointer_params = sig
        .params
        .iter()
        .any(|param| matches!(param.type_, RustTy::RawPointer { .. }));

    let has_pointer_return = matches!(sig.return_value.type_, Some(RustTy::RawPointer { .. }));

    // No short-circuiting due to variable decls, but that's fine.
    has_pointer_params || has_pointer_return
}

fn function_uses_default_params(sig: &FnSignature) -> bool {
    sig.params.iter().any(|arg| arg.default_value.is_some())
        && !special_cases::is_excluded_from_default_params(sig.surrounding_class, sig.function_name)
}
