/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! Generates a file for each Godot engine + builtin class

use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote, ToTokens};
use std::path::{Path, PathBuf};

use crate::api_parser::*;
use crate::central_generator::{collect_builtin_types, BuiltinTypeInfo};
use crate::util::{
    function_uses_pointers, ident, parse_native_structures_format, safe_ident, to_pascal_case,
    to_rust_type, to_rust_type_abi, to_snake_case, NativeStructuresField,
};
use crate::{
    special_cases, util, Context, GeneratedBuiltin, GeneratedBuiltinModule, GeneratedClass,
    GeneratedClassModule, ModName, RustTy, TyName,
};

pub(crate) fn generate_class_files(
    api: &ExtensionApi,
    ctx: &mut Context,
    _build_config: &str,
    gen_path: &Path,
    out_files: &mut Vec<PathBuf>,
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
        let file_contents = generated_class.code.to_string();

        let out_path = gen_path.join(format!("{}.rs", module_name.rust_mod));
        std::fs::write(&out_path, file_contents).expect("failed to write class file");
        out_files.push(out_path);

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
    let mod_contents = make_module_file(modules).to_string();
    std::fs::write(&out_path, mod_contents).expect("failed to write mod.rs file");
    out_files.push(out_path);
}

pub(crate) fn generate_builtin_class_files(
    api: &ExtensionApi,
    ctx: &mut Context,
    _build_config: &str,
    gen_path: &Path,
    out_files: &mut Vec<PathBuf>,
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
        let file_contents = generated_class.code.to_string();

        let out_path = gen_path.join(format!("{}.rs", module_name.rust_mod));
        std::fs::write(&out_path, file_contents).expect("failed to write class file");
        out_files.push(out_path);

        modules.push(GeneratedBuiltinModule {
            class_name: inner_class_name,
            module_name,
        });
    }

    let out_path = gen_path.join("mod.rs");
    let mod_contents = make_builtin_module_file(modules).to_string();
    std::fs::write(&out_path, mod_contents).expect("failed to write mod.rs file");
    out_files.push(out_path);
}

pub(crate) fn generate_native_structures_files(
    api: &ExtensionApi,
    ctx: &mut Context,
    _build_config: &str,
    gen_path: &Path,
    out_files: &mut Vec<PathBuf>,
) {
    let _ = std::fs::remove_dir_all(gen_path);
    std::fs::create_dir_all(gen_path).expect("create native directory");

    let mut modules = vec![];
    for native_structure in api.native_structures.iter() {
        let module_name = ModName::from_godot(&native_structure.name);
        let class_name = TyName::from_godot(&native_structure.name);

        let generated_class = make_native_structure(native_structure, &class_name, ctx);
        let file_contents = generated_class.code.to_string();

        let out_path = gen_path.join(format!("{}.rs", module_name.rust_mod));
        std::fs::write(&out_path, file_contents).expect("failed to write native structures file");
        out_files.push(out_path);

        modules.push(GeneratedBuiltinModule {
            class_name,
            module_name,
        });
    }

    let out_path = gen_path.join("mod.rs");
    let mod_contents = make_builtin_module_file(modules).to_string();
    std::fs::write(&out_path, mod_contents).expect("failed to write mod.rs file");
    out_files.push(out_path);
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
    let methods = make_methods(&class.methods, class_name, ctx);
    let enums = make_enums(&class.enums, class_name, ctx);
    let constants = make_constants(&class.constants, class_name, ctx);
    let inherits_macro = format_ident!("inherits_transitive_{}", class_name.rust_ty);
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
        use crate::native_structure::*;
        use crate::obj::{AsArg, Gd};
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
            impl crate::obj::GodotClass for #class_name {
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
                }
            }
        }

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
            self.notification(i32::from(what) as i64, false);
        }

        /// ⚠️ Like [`Self::notify()`], but starts at the most-derived class and goes up the hierarchy.
        ///
        /// See docs of that method, including the panics.
        pub fn notify_reversed(&mut self, what: #enum_name) {
            self.notification(i32::from(what) as i64, true);
        }
    }
}

fn make_notification_enum(
    class_name: &TyName,
    all_bases: &Vec<TyName>,
    ctx: &mut Context,
) -> (Option<TokenStream>, Ident) {
    let Some(all_constants) = ctx.notification_constants(class_name) else  {
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
        notification_enumerators_ord.push(constant_value);
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
    let outer_class = if let RustTy::BuiltinIdent(ident) = to_rust_type(&class.name, ctx) {
        ident
    } else {
        panic!("Rust type `{}` categorized wrong", class.name)
    };
    let inner_class = &inner_class_name.rust_ty;

    let class_enums = class.enums.as_ref().map(|class_enums| {
        class_enums
            .iter()
            .map(BuiltinClassEnum::to_enum)
            .collect::<Vec<Enum>>()
    });

    let methods = make_builtin_methods(&class.methods, class_name, type_info, ctx);
    let enums = make_enums(&class_enums, class_name, ctx);
    let special_constructors = make_special_builtin_methods(class_name, ctx);

    // mod re_export needed, because class should not appear inside the file module, and we can't re-export private struct as pub
    let tokens = quote! {
        use godot_ffi as sys;
        use crate::builtin::*;
        use crate::native_structure::*;
        use crate::obj::{AsArg, Gd};
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

        #enums
    };
    // note: TypePtr -> ObjectPtr conversion OK?

    GeneratedBuiltin { code: tokens }
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
        use crate::native_structure::*;
        use crate::obj::{AsArg, Gd};
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

fn make_methods(
    methods: &Option<Vec<ClassMethod>>,
    class_name: &TyName,
    ctx: &mut Context,
) -> TokenStream {
    let methods = match methods {
        Some(m) => m,
        None => return TokenStream::new(),
    };

    let definitions = methods
        .iter()
        .map(|method| make_method_definition(method, class_name, ctx));

    quote! {
        #( #definitions )*
    }
}

fn make_builtin_methods(
    methods: &Option<Vec<BuiltinClassMethod>>,
    class_name: &TyName,
    type_info: &BuiltinTypeInfo,
    ctx: &mut Context,
) -> TokenStream {
    let methods = match methods {
        Some(m) => m,
        None => return TokenStream::new(),
    };

    let definitions = methods
        .iter()
        .map(|method| make_builtin_method_definition(method, class_name, type_info, ctx));

    quote! {
        #( #definitions )*
    }
}

fn make_enums(enums: &Option<Vec<Enum>>, _class_name: &TyName, _ctx: &Context) -> TokenStream {
    let Some(enums) = enums else {
        return TokenStream::new();
    };

    let definitions = enums.iter().map(util::make_enum_definition);

    quote! {
        #( #definitions )*
    }
}

fn make_constants(
    constants: &Option<Vec<ClassConstant>>,
    _class_name: &TyName,
    _ctx: &Context,
) -> TokenStream {
    let Some(constants) = constants else {
        return TokenStream::new();
    };

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
    is_rust_type_excluded(&to_rust_type(ty, ctx))
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
) -> TokenStream {
    if is_method_excluded(method, false, ctx) || special_cases::is_deleted(class_name, &method.name)
    {
        return TokenStream::new();
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

    let (receiver, receiver_arg) = make_receiver(
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
    let varcall_invocation = quote! {
        __call_fn(__method_bind, #receiver_arg, __args_ptr, __args.len() as i64, return_ptr, std::ptr::addr_of_mut!(__err));
    };
    let ptrcall_invocation = quote! {
        __call_fn(__method_bind, #receiver_arg, __args_ptr, return_ptr);
    };

    let is_virtual = false;
    make_function_definition(
        method_name_str,
        special_cases::is_private(class_name, &method.name),
        receiver,
        &method.arguments,
        method.return_value.as_ref(),
        variant_ffi,
        init_code,
        &varcall_invocation,
        &ptrcall_invocation,
        is_virtual,
        ctx,
    )
}

fn make_builtin_method_definition(
    method: &BuiltinClassMethod,
    class_name: &TyName,
    type_info: &BuiltinTypeInfo,
    ctx: &mut Context,
) -> TokenStream {
    let method_name_str = &method.name;

    let (receiver, receiver_arg) =
        make_receiver(method.is_static, method.is_const, quote! { self.sys_ptr });

    let return_value = method.return_type.as_deref().map(MethodReturn::from_type);
    let hash = method.hash;
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
    let ptrcall_invocation = quote! {
        __call_fn(#receiver_arg, __args_ptr, return_ptr, __args.len() as i32);
    };

    let is_virtual = false;
    make_function_definition(
        method_name_str,
        special_cases::is_private(class_name, &method.name),
        receiver,
        &method.arguments,
        return_value.as_ref(),
        variant_ffi,
        init_code,
        &ptrcall_invocation,
        &ptrcall_invocation,
        is_virtual,
        ctx,
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
    let return_value = function.return_type.as_deref().map(MethodReturn::from_type);
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

    let is_virtual = false;
    make_function_definition(
        function_name_str,
        false,
        TokenStream::new(),
        &function.arguments,
        return_value.as_ref(),
        variant_ffi,
        init_code,
        &invocation,
        &invocation,
        is_virtual,
        ctx,
    )
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

#[allow(clippy::too_many_arguments)] // adding a struct/trait that's used only here, one time, reduces complexity by precisely 0%
fn make_function_definition(
    function_name: &str,
    is_private: bool,
    receiver: TokenStream,
    method_args: &Option<Vec<MethodArg>>,
    return_value: Option<&MethodReturn>,
    variant_ffi: Option<VariantFfi>,
    init_code: TokenStream,
    varcall_invocation: &TokenStream,
    ptrcall_invocation: &TokenStream,
    is_virtual: bool,
    ctx: &mut Context,
) -> TokenStream {
    let vis = if is_private {
        quote! { pub(crate) }
    } else {
        quote! { pub }
    };
    let (safety, doc) = if function_uses_pointers(method_args, &return_value) {
        (
            quote! { unsafe },
            quote! {
                #[doc = "# Safety"]
                #[doc = ""]
                #[doc = "Godot currently does not document safety requirements on this method. Make sure you understand the underlying semantics."]
            },
        )
    } else {
        (quote! {}, quote! {})
    };

    let is_varcall = variant_ffi.is_some();
    let fn_name = safe_ident(function_name);
    let [params, variant_types, arg_exprs, arg_names] = make_params(method_args, is_varcall, ctx);

    let (prepare_arg_types, error_fn_context);
    if variant_ffi.is_some() {
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

        let fmt = format!("{function_name}({joined}; {{__vararg_str}})");
        error_fn_context = quote! { &format!(#fmt) };
    } else {
        // ptrcall
        prepare_arg_types = quote! {
            let __arg_types = [
                #( #variant_types ),*
            ];
        };
        error_fn_context = function_name.to_token_stream();
    };

    let (return_decl, call_code) = make_return(
        return_value,
        variant_ffi.as_ref(),
        varcall_invocation,
        ptrcall_invocation,
        prepare_arg_types,
        error_fn_context,
        is_virtual,
        ctx,
    );

    if is_virtual {
        quote! {
            #doc
            #safety fn #fn_name( #receiver #( #params, )* ) #return_decl {
                #call_code
            }
        }
    } else if let Some(variant_ffi) = variant_ffi.as_ref() {
        // varcall (using varargs)
        let sys_method = &variant_ffi.sys_method;
        quote! {
            #doc
            #vis #safety fn #fn_name( #receiver #( #params, )* varargs: &[Variant]) #return_decl {
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
        quote! {
            #doc
            #vis #safety fn #fn_name( #receiver #( #params, )* ) #return_decl {
                unsafe {
                    #init_code

                    let __args = [
                        #( #arg_exprs ),*
                    ];

                    let __args_ptr = __args.as_ptr();

                    #call_code
                }
            }
        }
    }
}

fn make_receiver(
    is_static: bool,
    is_const: bool,
    receiver_arg: TokenStream,
) -> (TokenStream, TokenStream) {
    let receiver = make_receiver_self_param(is_static, is_const);

    let receiver_arg = if is_static {
        quote! { std::ptr::null_mut() }
    } else {
        receiver_arg
    };

    (receiver, receiver_arg)
}

fn make_receiver_self_param(is_static: bool, is_const: bool) -> TokenStream {
    if is_static {
        quote! {}
    } else if is_const {
        quote! { &self, }
    } else {
        quote! { &mut self, }
    }
}

fn make_params(
    method_args: &Option<Vec<MethodArg>>,
    is_varcall: bool,
    ctx: &mut Context,
) -> [Vec<TokenStream>; 4] {
    let empty = vec![];
    let method_args = method_args.as_ref().unwrap_or(&empty);

    let mut params = vec![];
    let mut variant_types = vec![];
    let mut arg_exprs = vec![];
    let mut arg_names = vec![];
    for arg in method_args.iter() {
        let param_name = safe_ident(&arg.name);
        let param_ty = to_rust_type(&arg.type_, ctx);

        let arg_expr = if is_varcall {
            quote! { <#param_ty as ToVariant>::to_variant(&#param_name) }
        } else if let RustTy::EngineClass { tokens: path, .. } = &param_ty {
            quote! { <#path as AsArg>::as_arg_ptr(&#param_name) }
        } else {
            quote! { <#param_ty as sys::GodotFfi>::sys_const(&#param_name) }
        };

        params.push(quote! { #param_name: #param_ty });
        variant_types.push(quote! { <#param_ty as VariantMetadata>::variant_type() });
        arg_exprs.push(arg_expr);
        arg_names.push(quote! { #param_name });
    }
    [params, variant_types, arg_exprs, arg_names]
}

#[allow(clippy::too_many_arguments)]
fn make_return(
    return_value: Option<&MethodReturn>,
    variant_ffi: Option<&VariantFfi>,
    varcall_invocation: &TokenStream,
    ptrcall_invocation: &TokenStream,
    prepare_arg_types: TokenStream,
    error_fn_context: TokenStream, // only for panic message
    is_virtual: bool,
    ctx: &mut Context,
) -> (TokenStream, TokenStream) {
    let return_decl: TokenStream;
    let return_ty: Option<RustTy>;

    if let Some(ret) = return_value {
        let ty = to_rust_type(&ret.type_, ctx);
        return_decl = ty.return_decl();
        return_ty = Some(ty);
    } else {
        return_decl = TokenStream::new();
        return_ty = None;
    }

    let call = match (is_virtual, variant_ffi, return_ty) {
        (true, _, _) => {
            quote! {
                unimplemented!()
            }
        }
        (false, Some(variant_ffi), Some(return_ty)) => {
            // If the return type is not Variant, then convert to concrete target type
            let return_expr = match return_ty {
                RustTy::BuiltinIdent(ident) if ident == "Variant" => quote! { variant },
                _ => quote! { variant.to() },
            };
            let from_sys_init_method = &variant_ffi.from_sys_init_method;

            // Note: __err may remain unused if the #call does not handle errors (e.g. utility fn, ptrcall, ...)
            // TODO use Result instead of panic on error
            quote! {
                let variant = Variant::#from_sys_init_method(|return_ptr| {
                    let mut __err = sys::default_call_error();
                    #varcall_invocation
                    if __err.error != sys::GDEXTENSION_CALL_OK {
                        #prepare_arg_types
                        sys::panic_call_error(&__err, #error_fn_context, &__arg_types);
                    }
                });
                #return_expr
            }
        }
        (false, Some(_), None) => {
            // Note: __err may remain unused if the #call does not handle errors (e.g. utility fn, ptrcall, ...)
            // TODO use Result instead of panic on error
            quote! {
                let mut __err = sys::default_call_error();
                let return_ptr = std::ptr::null_mut();
                #varcall_invocation
                if __err.error != sys::GDEXTENSION_CALL_OK {
                    #prepare_arg_types
                    sys::panic_call_error(&__err, #error_fn_context, &__arg_types);
                }
            }
        }
        (false, None, Some(RustTy::EngineClass { tokens, .. })) => {
            let return_ty = tokens;
            quote! {
                <#return_ty>::from_sys_init_opt(|return_ptr| {
                    #ptrcall_invocation
                })
            }
        }
        (false, None, Some(return_ty)) => {
            quote! {
                <#return_ty as sys::GodotFfi>::from_sys_init_default(|return_ptr| {
                    #ptrcall_invocation
                })
            }
        }
        (false, None, None) => {
            quote! {
                let return_ptr = std::ptr::null_mut();
                #ptrcall_invocation
            }
        }
    };

    (return_decl, call)
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

fn make_virtual_method(class_method: &ClassMethod, ctx: &mut Context) -> TokenStream {
    let method_name = virtual_method_name(class_method);

    // Virtual methods are never static.
    assert!(!class_method.is_static);

    let receiver = make_receiver_self_param(false, class_method.is_const);

    // make_return requests these token streams, but they won't be used for
    // virtual methods. We can provide empty streams.
    let varcall_invocation = TokenStream::new();
    let ptrcall_invocation = TokenStream::new();
    let init_code = TokenStream::new();
    let variant_ffi = None;

    let is_virtual = true;
    let is_private = false;
    make_function_definition(
        method_name,
        is_private,
        receiver,
        &class_method.arguments,
        class_method.return_value.as_ref(),
        variant_ffi,
        init_code,
        &varcall_invocation,
        &ptrcall_invocation,
        is_virtual,
        ctx,
    )
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
