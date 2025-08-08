/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::path::Path;

use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};

use crate::context::{Context, NotificationEnum};
use crate::generator::functions_common::{FnCode, FnDefinition, FnDefinitions};
use crate::generator::method_tables::MethodTableKey;
use crate::generator::{
    constants, docs, enums, functions_common, notifications, signals, virtual_traits,
};
use crate::models::domain::{
    ApiView, Class, ClassLike, ClassMethod, ExtensionApi, FnDirection, FnQualifier, Function,
    ModName, TyName,
};
use crate::util::{ident, make_string_name};
use crate::{util, SubmitFn};

pub fn generate_class_files(
    api: &ExtensionApi,
    ctx: &mut Context,
    view: &ApiView,
    gen_path: &Path,
    submit_fn: &mut SubmitFn,
) {
    let _ = std::fs::remove_dir_all(gen_path);
    std::fs::create_dir_all(gen_path).expect("create classes directory");

    let mut modules = vec![];
    for class in api.classes.iter() {
        let generated_class = make_class(class, ctx, view);
        let file_contents = generated_class.code;

        let out_path = gen_path.join(format!("{}.rs", class.mod_name().rust_mod));

        submit_fn(out_path, file_contents);

        modules.push(GeneratedClassModule {
            class_name: class.name().clone(),
            module_name: class.mod_name().clone(),
            own_notification_enum_name: generated_class.notification_enum.try_to_own_name(),
            inherits_macro_ident: generated_class.inherits_macro_ident,
            is_pub_sidecar: generated_class.has_sidecar_module,
            has_interface_trait: generated_class.has_interface_trait,
        });
    }

    let out_path = gen_path.join("mod.rs");
    let mod_contents = make_class_module_file(modules);

    submit_fn(out_path, mod_contents);
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Implementation

struct GeneratedClass {
    code: TokenStream,
    notification_enum: NotificationEnum,
    inherits_macro_ident: Option<Ident>,
    /// Sidecars are the associated modules with related enum/flag types, such as `node_3d` for `Node3D` class.
    has_sidecar_module: bool,
    has_interface_trait: bool,
}

struct GeneratedClassModule {
    class_name: TyName,
    module_name: ModName,
    own_notification_enum_name: Option<Ident>,
    inherits_macro_ident: Option<Ident>,
    is_pub_sidecar: bool,
    has_interface_trait: bool,
}

struct Construction {
    constructor: TokenStream,
    construct_doc: &'static str,
    final_doc: Option<&'static str>,
    godot_default_impl: TokenStream,
}

fn make_class(class: &Class, ctx: &mut Context, view: &ApiView) -> GeneratedClass {
    let class_name = class.name();

    // Strings
    let godot_class_str = &class_name.godot_ty;
    let class_name_cstr = util::c_str(godot_class_str);

    // Idents and tokens
    let (base_ty, base_ident_opt) = match class.base_class.as_ref() {
        Some(TyName { rust_ty, .. }) => {
            (quote! { crate::classes::#rust_ty }, Some(rust_ty.clone()))
        }
        None => (quote! { crate::obj::NoBase }, None),
    };

    let Construction {
        constructor,
        construct_doc,
        final_doc,
        godot_default_impl,
    } = make_constructor_and_default(class, ctx);

    let mut extended_class_doc = construct_doc.replace("Self", &class_name.rust_ty.to_string());
    extended_class_doc.push_str(final_doc.unwrap_or_default());

    let api_level = class.api_level;
    let init_level = api_level.to_init_level();

    // These attributes are for our nightly docs pipeline, which enables "only available in ..." labels in the HTML output. The website CI sets
    // RUSTFLAGS="--cfg published_docs" during the `cargo +nightly doc` invocation. They are applied to classes, interface traits, sidecar modules,
    // the notification enum, other enums and default-parameter extender structs.
    //
    // In other parts of the codebase, #[cfg] statements are replaced with #[doc(cfg)] using sed/sd. However, that doesn't work here, because
    // generated files are output in ./target/build/debug. Upon doing sed/sd replacements on these files, cargo doc will either treat them as
    // unchanged (doing nothing), or rebuild the generated files into a _different_ folder. Therefore, the generator itself must already provide
    // the correct attributes from the start.
    let (cfg_attributes, cfg_inner_attributes);
    if class.is_experimental {
        cfg_attributes = quote! {
            // #[cfg(feature = "experimental-godot-api")]
            #[cfg_attr(published_docs, doc(cfg(feature = "experimental-godot-api")))]
        };
        cfg_inner_attributes = quote! {
            // #![cfg(feature = "experimental-godot-api")]
            #![cfg_attr(published_docs, doc(cfg(feature = "experimental-godot-api")))]
        };
    } else {
        cfg_attributes = TokenStream::new();
        cfg_inner_attributes = TokenStream::new();
    };

    let FnDefinitions {
        functions: methods,
        builders,
    } = make_class_methods(class, &class.methods, &cfg_attributes, ctx);

    let signals::SignalCodegen {
        signal_code,
        has_own_signals,
    } = signals::make_class_signals(class, &class.signals, ctx);

    let enums = enums::make_enums(&class.enums, &cfg_attributes);
    let constants = constants::make_constants(&class.constants);
    let deref_impl = make_deref_impl(class_name, &base_ty);

    let all_bases = ctx.inheritance_tree().collect_all_bases(class_name);
    let (inherits_macro_ident, inherits_macro_code) = make_inherits_macro(class, &all_bases);
    let (notification_enum, notification_enum_name) =
        notifications::make_notification_enum(class_name, &all_bases, &cfg_attributes, ctx);

    // Associated "sidecar" module is made public if there are other symbols related to the class, which are not
    // in top-level godot::classes module (notification enums are not in the sidecar, but in godot::classes::notify).
    // This checks if token streams (i.e. code) is empty.
    let has_sidecar_module = !enums.is_empty() || !builders.is_empty() || has_own_signals;

    let module_doc = docs::make_module_doc(class_name);

    // Classes that can't be inherited from don't need to provide an interface with overridable virtual methods.
    let has_interface_trait = !class.is_final;
    let interface_trait = if has_interface_trait {
        virtual_traits::make_virtual_methods_trait(
            class,
            &all_bases,
            &notification_enum_name,
            &cfg_attributes,
            view,
            ctx,
        )
    } else {
        TokenStream::new()
    };

    let class_doc = docs::make_class_doc(
        class_name,
        base_ident_opt,
        notification_enum.is_some(),
        has_sidecar_module,
        has_interface_trait,
        has_own_signals,
    );

    // notify() and notify_reversed() are added after other methods, to list others first in docs.
    let notify_methods = notifications::make_notify_methods(class_name, ctx);

    let (assoc_memory, assoc_dyn_memory, is_exportable) = make_bounds(class, ctx);

    let internal_methods = quote! {
        fn __checked_id(&self) -> Option<crate::obj::InstanceId> {
            // SAFETY: only Option due to layout-compatibility with RawGd<T>; it is always Some because stored in Gd<T> which is non-null.
            let rtti = unsafe { self.rtti.as_ref().unwrap_unchecked() };
            let instance_id = rtti.check_type::<Self>();
            Some(instance_id)
        }

        #[doc(hidden)]
        pub fn __object_ptr(&self) -> sys::GDExtensionObjectPtr {
            self.object_ptr
        }
    };

    // mod re_export needed, because class should not appear inside the file module, and we can't re-export private struct as pub.
    let imports = util::make_imports();
    let tokens = quote! {
        #![doc = #module_doc]
        #cfg_inner_attributes

        #imports
        use crate::classes::notify::*;
        use std::ffi::c_void;

        pub(super) mod re_export {
            use super::*;

            #[doc = #class_doc]
            #[doc = #extended_class_doc]
            #cfg_attributes
            #[derive(Debug)]
            #[repr(C)]
            pub struct #class_name {
                object_ptr: sys::GDExtensionObjectPtr,

                // This field should never be None. Type Option<T> is chosen to be layout-compatible with Gd<T>, which uses RawGd<T> inside.
                // The RawGd<T>'s identity field can be None because of generality (it can represent null pointers, as opposed to Gd<T>).
                rtti: Option<crate::private::ObjectRtti>,
            }
            #interface_trait
            #notification_enum
            impl #class_name {
                #constructor
                #methods
                #notify_methods
                #internal_methods
                #constants
            }
            impl crate::obj::GodotClass for #class_name {
                type Base = #base_ty;

                // Code duplicated in godot-macros.
                fn class_name() -> ClassName {
                    // Optimization note: instead of lazy init, could use separate static which is manually initialized during registration.
                    static CLASS_NAME: std::sync::OnceLock<ClassName> = std::sync::OnceLock::new();

                    let name: &'static ClassName = CLASS_NAME.get_or_init(|| ClassName::alloc_next_ascii(#class_name_cstr));
                    *name
                }

                const INIT_LEVEL: crate::init::InitLevel = #init_level;
            }
            unsafe impl crate::obj::Bounds for #class_name {
                type Memory = crate::obj::bounds::#assoc_memory;
                type DynMemory = crate::obj::bounds::#assoc_dyn_memory;
                type Declarer = crate::obj::bounds::DeclEngine;
                type Exportable = crate::obj::bounds::#is_exportable;
            }

            #(
                // SAFETY: #all_bases is a list of classes provided by Godot such that #class_name is guaranteed a subclass of all of them.
                unsafe impl crate::obj::Inherits<crate::classes::#all_bases> for #class_name {}
            )*

            #godot_default_impl
            #deref_impl
            #inherits_macro_code
        }

        #builders
        #enums
        #signal_code
    };
    // note: TypePtr -> ObjectPtr conversion OK?

    GeneratedClass {
        code: tokens,
        notification_enum: NotificationEnum {
            name: notification_enum_name,
            declared_by_own_class: notification_enum.is_some(),
        },
        inherits_macro_ident,
        has_sidecar_module,
        has_interface_trait,
    }
}

/// If the class can be inherited from (non-final), create a macro that can be accessed in subclasses to implement the `Inherits` trait.
///
/// Returns empty tokens if the class is final.
fn make_inherits_macro(class: &Class, all_bases: &[TyName]) -> (Option<Ident>, TokenStream) {
    let class_name = class.name();

    // Create a macro that can be accessed in subclasses to implement the Inherits trait.
    // Use this name because when typing a non-existent class, users will be met with the following error:
    //    could not find `inherit_from_OS__ensure_class_exists` in `class_macros`
    //
    // Former macro name was `unsafe_inherits_transitive_*`.
    let inherits_macro_ident =
        format_ident!("inherit_from_{}__ensure_class_exists", class_name.rust_ty);

    // For final classes, we can directly create a meaningful compile error.
    if class.is_final {
        let error_msg = format!(
            "Class `{}` is final, meaning it cannot be inherited in GDExtension or GDScript.",
            class_name.rust_ty
        );

        let code = quote! {
            #[macro_export]
            #[allow(non_snake_case)]
            macro_rules! #inherits_macro_ident {
                ($Class:ident) => {
                    compile_error!(#error_msg);
                }
            }
        };

        return (None, code);
    }

    let inherits_macro_safety_doc = format!(
        "The provided class must be a subclass of all the superclasses of [`{}`]",
        class_name.rust_ty
    );

    let code = quote! {
        /// # Safety
        ///
        #[doc = #inherits_macro_safety_doc]
        #[macro_export]
        #[allow(non_snake_case)]
        macro_rules! #inherits_macro_ident {
            ($Class:ident) => {
                unsafe impl ::godot::obj::Inherits<::godot::classes::#class_name> for $Class {}
                #(
                    unsafe impl ::godot::obj::Inherits<::godot::classes::#all_bases> for $Class {}
                )*
            }
        }
    };

    (Some(inherits_macro_ident), code)
}

fn make_class_module_file(classes_and_modules: Vec<GeneratedClassModule>) -> TokenStream {
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

        let interface_reexport = m.has_interface_trait.then(|| {
            quote! { pub use #module_name::re_export::#virtual_trait_name; }
        });

        let class_decl = quote! {
            #vis mod #module_name;
            pub use #module_name::re_export::#class_name;
            #interface_reexport
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

        // For final classes, do nothing.
        let Some(inherits_macro_ident) = inherits_macro_ident else {
            return TokenStream::new();
        };

        // We cannot re-export the following, because macro is in the crate root
        // pub use #module_ident::re_export::#inherits_macro_ident;
        quote! {
            pub use #inherits_macro_ident;
        }
    });

    quote! {
        #( #class_decls )*

        /// Notification enums for all classes.
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

fn make_constructor_and_default(class: &Class, ctx: &Context) -> Construction {
    let class_name = class.name();

    let godot_class_stringname = make_string_name(&class_name.godot_ty);
    // Note: this could use class_name() but is not yet done due to potential future lazy-load refactoring.
    //let class_name_obj = quote! { <Self as crate::obj::GodotClass>::class_name() };

    let (constructor, construct_doc, has_godot_default_impl);
    if ctx.is_singleton(class_name) {
        // Note: we cannot return &'static mut Self, as this would be very easy to mutably alias.
        // &'static Self would be possible, but we would lose the whole mutability information (even if that is best-effort and
        // not strict Rust mutability, it makes the API much more usable).
        // As long as the user has multiple Gd smart pointers to the same singletons, only the internal raw pointers are aliased.
        // See also Deref/DerefMut impl for Gd.
        constructor = quote! {
            pub fn singleton() -> Gd<Self> {
                unsafe {
                    let __class_name = #godot_class_stringname;
                    let __object_ptr = sys::interface_fn!(global_get_singleton)(__class_name.string_sys());
                    Gd::from_obj_sys(__object_ptr)
                }
            }
        };
        construct_doc = "# Singleton\n\n\
            This class is a singleton. You can get the one instance using [`Self::singleton()`][Self::singleton].";
        has_godot_default_impl = false;
    } else if !class.is_instantiable {
        // Abstract base classes or non-singleton classes without constructor.
        constructor = TokenStream::new();
        construct_doc = "# Not instantiable\n\nThis class cannot be constructed. Obtain `Gd<Self>` instances via Godot APIs.";
        has_godot_default_impl = false;
    } else {
        // Manually managed classes (Object, Node, ...) as well as ref-counted ones (RefCounted, Resource, ...).
        // The constructors are provided as associated methods in NewGd::new_gd() and NewAlloc::new_alloc().
        constructor = TokenStream::new();

        if class.is_refcounted {
            construct_doc = "# Construction\n\n\
                This class is reference-counted. You can create a new instance using [`Self::new_gd()`][crate::obj::NewGd::new_gd]."
        } else {
            construct_doc = "# Construction\n\n\
                This class is manually managed. You can create a new instance using [`Self::new_alloc()`][crate::obj::NewAlloc::new_alloc].\n\n\
                Do not forget to call [`free()`][crate::obj::Gd::free] or hand over ownership to Godot."
        }

        has_godot_default_impl = true;
    }

    let final_doc = if class.is_final {
        Some(
            "\n\n# Final class\n\n\
            This class is _final_, meaning you cannot inherit from it, and it comes without `I*` interface trait. \
            It is still possible that other Godot classes inherit from it, but that is limited to the engine itself.",
        )
    } else {
        None
    };

    let godot_default_impl = if has_godot_default_impl {
        let class_name = &class.name().rust_ty;
        quote! {
            impl crate::obj::cap::GodotDefault for #class_name {
                fn __godot_default() -> crate::obj::Gd<Self> {
                    crate::classes::construct_engine_object::<Self>()
                }
            }
        }
    } else {
        TokenStream::new()
    };

    Construction {
        constructor,
        construct_doc,
        final_doc,
        godot_default_impl,
    }
}

fn make_deref_impl(class_name: &TyName, base_ty: &TokenStream) -> TokenStream {
    // The base_ty of `Object` is `NoBase`, and we don't want every engine class to deref to `NoBase`.
    if class_name.rust_ty == "Object" {
        return TokenStream::new();
    }

    quote! {
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
    }
}

fn make_bounds(class: &Class, ctx: &mut Context) -> (Ident, Ident, Ident) {
    let c = class.name();

    let assoc_dyn_memory = if c.rust_ty == "Object" {
        ident("MemDynamic")
    } else if class.is_refcounted {
        ident("MemRefCounted")
    } else {
        ident("MemManual")
    };

    let assoc_memory = if class.is_refcounted {
        ident("MemRefCounted")
    } else {
        ident("MemManual")
    };

    let tree = ctx.inheritance_tree();
    let is_exportable = if tree.inherits(c, "Node") || tree.inherits(c, "Resource") {
        ident("Yes")
    } else {
        ident("No")
    };

    (assoc_memory, assoc_dyn_memory, is_exportable)
}

fn make_class_methods(
    class: &Class,
    methods: &[ClassMethod],
    cfg_attributes: &TokenStream,
    ctx: &mut Context,
) -> FnDefinitions {
    let get_method_table = class.api_level.table_global_getter();

    let definitions = methods.iter().map(|method| {
        make_class_method_definition(class, method, &get_method_table, cfg_attributes, ctx)
    });

    FnDefinitions::expand(definitions)
}

fn make_class_method_definition(
    class: &Class,
    method: &ClassMethod,
    get_method_table: &Ident,
    cfg_attributes: &TokenStream,
    ctx: &mut Context,
) -> FnDefinition {
    let FnDirection::Outbound { hash } = method.direction() else {
        return FnDefinition::none();
    };

    let rust_class_name = class.name().rust_ty.to_string();
    let rust_method_name = method.name();
    let godot_method_name = method.godot_name();

    let receiver = functions_common::make_receiver(method.qualifier(), quote! { self.object_ptr });

    let table_index = ctx.get_table_index(&MethodTableKey::from_class(class, method));

    let maybe_instance_id = if method.qualifier() == FnQualifier::Static {
        quote! { None }
    } else {
        quote! { self.__checked_id() }
    };

    let fptr_access = if cfg!(feature = "codegen-lazy-fptrs") {
        let godot_class_name = &class.name().godot_ty;
        quote! {
            fptr_by_key(sys::lazy_keys::ClassMethodKey {
                class_name: #godot_class_name,
                method_name: #godot_method_name,
                hash: #hash,
            })
        }
    } else {
        quote! { fptr_by_index(#table_index) }
    };

    let object_ptr = &receiver.ffi_arg;
    let ptrcall_invocation = quote! {
        let method_bind = sys::#get_method_table().#fptr_access;

        Signature::<CallParams, CallRet>::out_class_ptrcall(
            method_bind,
            #rust_class_name,
            #rust_method_name,
            #object_ptr,
            #maybe_instance_id,
            args,
        )
    };

    let varcall_invocation = quote! {
        let method_bind = sys::#get_method_table().#fptr_access;

        Signature::<CallParams, CallRet>::out_class_varcall(
            method_bind,
            #rust_class_name,
            #rust_method_name,
            #object_ptr,
            #maybe_instance_id,
            args,
            varargs
        )
    };

    functions_common::make_function_definition(
        method,
        &FnCode {
            receiver,
            varcall_invocation,
            ptrcall_invocation,
            is_virtual_required: false,
            is_varcall_fallible: true,
        },
        None,
        cfg_attributes,
    )
}
