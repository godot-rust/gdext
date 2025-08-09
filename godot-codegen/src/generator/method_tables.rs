/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::fmt;

use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote, ToTokens, TokenStreamExt};

use crate::context::Context;
use crate::generator::lifecycle_builtins;
use crate::models::domain::{
    BuiltinClass, BuiltinMethod, BuiltinVariant, Class, ClassCodegenLevel, ClassLike, ClassMethod,
    ExtensionApi, FnDirection, Function, TyName,
};
use crate::util::ident;
use crate::{conv, generator, special_cases, util};

pub fn make_builtin_lifecycle_table(api: &ExtensionApi) -> TokenStream {
    let builtins = &api.builtins;
    let len = builtins.len();

    let mut table = NamedMethodTable {
        table_name: ident("BuiltinLifecycleTable"),
        imports: quote! {
            use crate::{
                GDExtensionConstTypePtr, GDExtensionTypePtr, GDExtensionUninitializedTypePtr,
                GDExtensionUninitializedVariantPtr, GDExtensionVariantPtr,
            };
        },
        ctor_parameters: quote! {
            interface: &crate::GDExtensionInterface,
        },
        pre_init_code: quote! {
            let get_construct_fn = interface.variant_get_ptr_constructor.unwrap();
            let get_destroy_fn = interface.variant_get_ptr_destructor.unwrap();
            let get_operator_fn = interface.variant_get_ptr_operator_evaluator.unwrap();

            let get_to_variant_fn = interface.get_variant_from_type_constructor.unwrap();
            let get_from_variant_fn = interface.get_variant_to_type_constructor.unwrap();
        },
        method_decls: Vec::with_capacity(len),
        method_inits: Vec::with_capacity(len),
        class_count: len,
        method_count: 0,
    };

    // Note: NIL is not part of this iteration, it will be added manually.
    for variant in builtins.iter() {
        let (decls, inits) = lifecycle_builtins::make_variant_fns(api, variant);

        table.method_decls.push(decls);
        table.method_inits.push(inits);
    }

    make_named_method_table(table)
}

pub fn make_class_method_table(
    api: &ExtensionApi,
    api_level: ClassCodegenLevel,
    ctx: &mut Context,
) -> TokenStream {
    let mut table = IndexedMethodTable {
        table_name: api_level.table_struct(),
        imports: TokenStream::new(),
        ctor_parameters: quote! {
            interface: &crate::GDExtensionInterface,
            string_names: &mut crate::StringCache,
        },
        pre_init_code: TokenStream::new(), // late-init, depends on class string names
        fptr_type: quote! { crate::ClassMethodBind },
        fetch_fptr_type: quote! { crate::GDExtensionInterfaceClassdbGetMethodBind },
        method_init_groups: vec![],
        lazy_key_type: quote! { crate::lazy_keys::ClassMethodKey },
        lazy_method_init: quote! {
            let get_method_bind = crate::interface_fn!(classdb_get_method_bind);
            crate::load_class_method(
                get_method_bind,
                &mut inner.string_cache,
                None,
                key.class_name,
                key.method_name,
                key.hash
            )
        },
        named_accessors: vec![],
        class_count: 0,
        method_count: 0,
    };

    api.classes
        .iter()
        .filter(|c| c.api_level == api_level)
        .for_each(|c| populate_class_methods(&mut table, c, ctx));

    table.pre_init_code = quote! {
        let fetch_fptr = interface.classdb_get_method_bind.expect("classdb_get_method_bind absent");
    };

    make_method_table(table)
}

pub fn make_builtin_method_table(api: &ExtensionApi, ctx: &mut Context) -> TokenStream {
    let mut table = IndexedMethodTable {
        table_name: ident("BuiltinMethodTable"),
        imports: TokenStream::new(),
        ctor_parameters: quote! {
            interface: &crate::GDExtensionInterface,
            string_names: &mut crate::StringCache,
        },
        pre_init_code: quote! {
            let fetch_fptr = interface.variant_get_ptr_builtin_method.expect("variant_get_ptr_builtin_method absent");
        },
        fptr_type: quote! { crate::BuiltinMethodBind },
        fetch_fptr_type: quote! { crate::GDExtensionInterfaceVariantGetPtrBuiltinMethod },
        method_init_groups: vec![],
        lazy_key_type: quote! { crate::lazy_keys::BuiltinMethodKey },
        lazy_method_init: quote! {
            let fetch_fptr = crate::interface_fn!(variant_get_ptr_builtin_method);
            crate::load_builtin_method(
                fetch_fptr,
                &mut inner.string_cache,
                key.variant_type.sys(),
                key.variant_type_str,
                key.method_name,
                key.hash
            )
        },
        named_accessors: vec![],
        class_count: 0,
        method_count: 0,
    };

    for builtin in api.builtins.iter() {
        populate_builtin_methods(&mut table, builtin, ctx);
    }

    make_method_table(table)
}

pub fn make_utility_function_table(api: &ExtensionApi) -> TokenStream {
    let mut table = NamedMethodTable {
        table_name: ident("UtilityFunctionTable"),
        imports: quote! {},
        ctor_parameters: quote! {
            interface: &crate::GDExtensionInterface,
            string_names: &mut crate::StringCache,
        },
        pre_init_code: quote! {
            let get_utility_fn = interface.variant_get_ptr_utility_function
                .expect("variant_get_ptr_utility_function absent");
        },
        method_decls: vec![],
        method_inits: vec![],
        class_count: 0,
        method_count: 0,
    };

    for function in api.utility_functions.iter() {
        let field = generator::utility_functions::make_utility_function_ptr_name(function);
        let fn_name_str = function.name();
        let hash = function.hash();

        table.method_decls.push(quote! {
            pub #field: crate::UtilityFunctionBind,
        });

        table.method_inits.push(quote! {
            #field: crate::load_utility_function(get_utility_fn, string_names, #fn_name_str, #hash),
        });

        table.method_count += 1;
    }

    make_named_method_table(table)
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Implementation

struct NamedMethodTable {
    table_name: Ident,
    imports: TokenStream,
    ctor_parameters: TokenStream,
    pre_init_code: TokenStream,
    method_decls: Vec<TokenStream>,
    method_inits: Vec<TokenStream>,
    class_count: usize,
    method_count: usize,
}

#[allow(dead_code)] // Individual fields would need to be cfg'ed with: feature = "codegen-lazy-fptrs".
struct IndexedMethodTable {
    table_name: Ident,
    imports: TokenStream,
    ctor_parameters: TokenStream,
    pre_init_code: TokenStream,
    fptr_type: TokenStream,
    fetch_fptr_type: TokenStream,
    method_init_groups: Vec<MethodInitGroup>,
    lazy_key_type: TokenStream,
    lazy_method_init: TokenStream,
    named_accessors: Vec<AccessorMethod>,
    class_count: usize,
    method_count: usize,
}

#[cfg_attr(feature = "codegen-lazy-fptrs", allow(dead_code))]
struct MethodInit {
    method_init: TokenStream,
    index: usize,
}

impl ToTokens for MethodInit {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.method_init.to_tokens(tokens);
    }
}

#[cfg_attr(feature = "codegen-lazy-fptrs", allow(dead_code))]
struct MethodInitGroup {
    class_name: Ident,
    class_var_init: Option<TokenStream>,
    method_inits: Vec<MethodInit>,
}

impl MethodInitGroup {
    fn new(
        godot_class_name: &str,
        class_var: Option<Ident>,
        method_inits: Vec<MethodInit>,
    ) -> Self {
        Self {
            class_name: ident(godot_class_name),
            // Only create class variable if any methods have been added.
            class_var_init: if class_var.is_none() || method_inits.is_empty() {
                None
            } else {
                let initializer_expr = util::make_sname_ptr(godot_class_name);
                Some(quote! {
                    let #class_var = #initializer_expr;
                })
            },
            method_inits,
        }
    }

    #[cfg(not(feature = "codegen-lazy-fptrs"))]
    fn function_name(&self) -> Ident {
        format_ident!("load_{}_methods", self.class_name)
    }
}

struct AccessorMethod {
    name: Ident,
    index: usize,
    lazy_key: TokenStream,
}

/// Generate code for a method table based on shared layout.
fn make_named_method_table(info: NamedMethodTable) -> TokenStream {
    let NamedMethodTable {
        table_name,
        imports,
        ctor_parameters,
        pre_init_code,
        method_decls,
        method_inits,
        class_count,
        method_count,
    } = info;

    // Assumes that both decls and inits already have a trailing comma.
    // This is necessary because some generators emit multiple lines (statements) per element.
    quote! {
        #imports

        #[allow(non_snake_case)]
        pub struct #table_name {
            #( #method_decls )*
        }

        impl #table_name {
            pub const CLASS_COUNT: usize = #class_count;
            pub const METHOD_COUNT: usize = #method_count;

            // TODO: Figure out the right safety preconditions. This currently does not have any because incomplete safety docs
            // can cause issues with people assuming they are sufficient.
            #[allow(clippy::missing_safety_doc)]
            pub unsafe fn load(
                #ctor_parameters
            ) -> Self {
                #pre_init_code

                Self {
                    #( #method_inits )*
                }
            }
        }
    }
}

#[cfg(not(feature = "codegen-lazy-fptrs"))]
fn make_method_table(info: IndexedMethodTable) -> TokenStream {
    let IndexedMethodTable {
        table_name,
        imports,
        ctor_parameters,
        pre_init_code,
        fptr_type,
        fetch_fptr_type,
        method_init_groups,
        lazy_key_type: _,
        lazy_method_init: _,
        named_accessors,
        class_count,
        method_count,
    } = info;

    // Editor table can be empty, if the Godot binary is compiled without editor.
    let unused_attr = (method_count == 0).then(|| quote! { #[allow(unused_variables)] });
    let named_method_api = make_named_accessors(&named_accessors, &fptr_type);

    // Make sure methods are complete and in order of index.
    assert_eq!(
        method_init_groups
            .iter()
            .map(|group| group.method_inits.len())
            .sum::<usize>(),
        method_count,
        "number of methods does not match count"
    );

    if let Some(last) = method_init_groups.last() {
        assert_eq!(
            last.method_inits.last().unwrap().index,
            method_count - 1,
            "last method should have highest index (table {table_name})"
        );
    } else {
        assert_eq!(method_count, 0, "empty method table should have count 0");
    }

    let method_load_inits = method_init_groups.iter().map(|group| {
        let func = group.function_name();
        quote! {
            #func(&mut function_pointers, string_names, fetch_fptr);
        }
    });

    let method_load_decls = method_init_groups.iter().map(|group| {
        let func = group.function_name();
        let method_inits = &group.method_inits;
        let class_var_init = &group.class_var_init;

        quote! {
            fn #func(
                function_pointers: &mut Vec<#fptr_type>,
                string_names: &mut crate::StringCache,
                fetch_fptr: FetchFn,
            ) {
                #class_var_init

                #(
                    function_pointers.push(#method_inits);
                )*
            }
        }
    });

    // Assumes that inits already have a trailing comma.
    // This is necessary because some generators emit multiple lines (statements) per element.
    quote! {
        #imports

        type FetchFn = <#fetch_fptr_type as crate::Inner>::FnPtr;

        pub struct #table_name {
            function_pointers: Vec<#fptr_type>,
        }

        impl #table_name {
            pub const CLASS_COUNT: usize = #class_count;
            pub const METHOD_COUNT: usize = #method_count;

            // TODO: Figure out the right safety preconditions. This currently does not have any because incomplete safety docs
            // can cause issues with people assuming they are sufficient.
            #[allow(clippy::missing_safety_doc)]
            #unused_attr
            pub unsafe fn load(
                #ctor_parameters
            ) -> Self {
                #pre_init_code

                let mut function_pointers = Vec::with_capacity(#method_count);
                #( #method_load_inits )*

                Self { function_pointers }
            }

            #[inline(always)]
            pub fn fptr_by_index(&self, index: usize) -> #fptr_type {
                // SAFETY: indices are statically generated and guaranteed to be in range.
                unsafe {
                    *self.function_pointers.get_unchecked(index)
                }
            }

            #named_method_api
        }

        #( #method_load_decls )*
    }
}

#[cfg(feature = "codegen-lazy-fptrs")]
fn make_method_table(info: IndexedMethodTable) -> TokenStream {
    let IndexedMethodTable {
        table_name,
        imports,
        ctor_parameters: _,
        pre_init_code: _,
        fptr_type,
        fetch_fptr_type: _,
        method_init_groups: _,
        lazy_key_type,
        lazy_method_init,
        named_accessors,
        class_count,
        method_count,
    } = info;

    // Editor table can be empty, if the Godot binary is compiled without editor.
    let unused_attr = (method_count == 0).then(|| quote! { #[allow(unused_variables)] });
    let named_method_api = make_named_accessors(&named_accessors, &fptr_type);

    // Assumes that inits already have a trailing comma.
    // This is necessary because some generators emit multiple lines (statements) per element.
    quote! {
        #imports
        use crate::StringCache;
        use std::collections::HashMap;
        use std::cell::RefCell;

        // Exists to be stored inside RefCell.
        struct InnerTable {
            // 'static because at this point, the interface and lifecycle tables are globally available.
            string_cache: StringCache<'static>,
            function_pointers: HashMap<#lazy_key_type, #fptr_type>,
        }

        // Note: get_method_bind and other function pointers could potentially be stored as fields in table, to avoid interface_fn!.
        pub struct #table_name {
            inner: RefCell<InnerTable>,
        }

        impl #table_name {
            pub const CLASS_COUNT: usize = #class_count;
            pub const METHOD_COUNT: usize = #method_count;

            // TODO: Figure out the right safety preconditions. This currently does not have any because incomplete safety docs
            // can cause issues with people assuming they are sufficient.
            #[allow(clippy::missing_safety_doc)]
            #unused_attr
            pub unsafe fn load() -> Self {
                // SAFETY: interface and lifecycle tables are initialized at this point, so we can get 'static references to them.
                let (interface, lifecycle_table) = unsafe {
                    (crate::get_interface(), crate::builtin_lifecycle_api())
                };

                Self {
                    inner: RefCell::new(InnerTable {
                        string_cache: StringCache::new(interface, lifecycle_table),
                        function_pointers: HashMap::new(),
                    }),
                }
            }

            #[inline(always)]
            pub fn fptr_by_key(&self, key: #lazy_key_type) -> #fptr_type {
                let mut guard = self.inner.borrow_mut();
                let inner = &mut *guard;
                *inner.function_pointers.entry(key.clone()).or_insert_with(|| {
                    #lazy_method_init
                })
            }

            #named_method_api
        }
    }
}

/// For index-based method tables, have select methods exposed by name for internal use.
fn make_named_accessors(accessors: &[AccessorMethod], fptr: &TokenStream) -> TokenStream {
    let mut result_api = TokenStream::new();

    for accessor in accessors {
        let AccessorMethod {
            name,
            index,
            lazy_key,
        } = accessor;

        let code = if cfg!(feature = "codegen-lazy-fptrs") {
            quote! {
                #[inline(always)]
                pub fn #name(&self) -> #fptr {
                    self.fptr_by_key(#lazy_key)
                }
            }
        } else {
            quote! {
                #[inline(always)]
                pub fn #name(&self) -> #fptr {
                    self.fptr_by_index(#index)
                }
            }
        };

        result_api.append_all(code.into_iter());
    }

    result_api
}

fn populate_class_methods(table: &mut IndexedMethodTable, class: &Class, ctx: &mut Context) {
    // Note: already checked outside whether class is active in codegen.

    let class_ty = class.name();
    let class_var = format_ident!("sname_{}", class_ty.godot_ty);
    let mut method_inits = vec![];

    for method in class.methods.iter() {
        // Virtual methods are not part of the class API itself, but exposed as an accompanying trait.
        let FnDirection::Outbound { hash } = method.direction() else {
            continue;
        };

        // Note: varcall/ptrcall is only decided at call time; the method bind is the same for both.
        let index = ctx.get_table_index(&MethodTableKey::from_class(class, method));

        let method_init = make_class_method_init(method, hash, &class_var, class_ty);
        method_inits.push(MethodInit { method_init, index });
        table.method_count += 1;

        // If requested, add a named accessor for this method.
        if special_cases::is_named_accessor_in_table(class_ty, method.godot_name()) {
            let class_name_str = class_ty.godot_ty.as_str();
            let method_name_str = method.name();

            table.named_accessors.push(AccessorMethod {
                name: make_table_accessor_name(class_ty, method),
                index,
                lazy_key: quote! {
                    crate::lazy_keys::ClassMethodKey {
                        class_name: #class_name_str,
                        method_name: #method_name_str,
                        hash: #hash,
                    }
                },
            });
        }
    }

    // No methods available, or all excluded (e.g. virtual ones) -> no group needed.
    if !method_inits.is_empty() {
        table.method_init_groups.push(MethodInitGroup::new(
            &class_ty.godot_ty,
            Some(class_var),
            method_inits,
        ));

        table.class_count += 1;
    }
}

fn populate_builtin_methods(
    table: &mut IndexedMethodTable,
    builtin: &BuiltinVariant,
    ctx: &mut Context,
) {
    let Some(builtin_class) = builtin.associated_builtin_class() else {
        // Ignore those where no class is generated (Object, int, bool etc.).
        return;
    };

    let builtin_ty = builtin_class.name();

    let mut method_inits = vec![];
    for method in builtin_class.methods.iter() {
        let index = ctx.get_table_index(&MethodTableKey::from_builtin(builtin_class, method));

        let method_init = make_builtin_method_init(builtin, method, index);
        method_inits.push(MethodInit { method_init, index });
        table.method_count += 1;

        // If requested, add a named accessor for this method.
        if special_cases::is_named_accessor_in_table(builtin_ty, method.godot_name()) {
            let variant_type = builtin.sys_variant_type();
            let variant_type_str = builtin.godot_original_name();
            let method_name_str = method.name();
            let hash = method.hash();

            table.named_accessors.push(AccessorMethod {
                name: make_table_accessor_name(builtin_ty, method),
                index,
                lazy_key: quote! {
                    crate::lazy_keys::BuiltinMethodKey {
                        variant_type: #variant_type,
                        variant_type_str: #variant_type_str,
                        method_name: #method_name_str,
                        hash: #hash,
                    }
                },
            });
        }
    }

    table.method_init_groups.push(MethodInitGroup::new(
        &builtin_class.name().godot_ty,
        None, // load_builtin_method() doesn't need a StringName for the class, as it accepts the VariantType enum.
        method_inits,
    ));
    table.class_count += 1;
}

fn make_class_method_init(
    method: &ClassMethod,
    hash: i64,
    class_var: &Ident,
    class_ty: &TyName,
) -> TokenStream {
    let class_name_str = class_ty.godot_ty.as_str();
    let method_name_str = method.godot_name();

    // Could reuse lazy key, but less code like this -> faster parsing.
    quote! {
        crate::load_class_method(
            fetch_fptr,
            string_names,
            Some(#class_var),
            #class_name_str,
            #method_name_str,
            #hash
        ),
    }
}

fn make_builtin_method_init(
    builtin: &BuiltinVariant,
    method: &BuiltinMethod,
    index: usize,
) -> TokenStream {
    let method_name_str = method.name();

    let variant_type = builtin.sys_variant_type();
    let variant_type_str = builtin.godot_original_name();

    let hash = method.hash();

    // Could reuse lazy key, but less code like this -> faster parsing.
    quote! {
        {
            let _ = #index;
            crate::load_builtin_method(
                fetch_fptr,
                string_names,
                crate::#variant_type,
                #variant_type_str,
                #method_name_str,
                #hash
            )
        },
    }
}

/// Lookup key for indexed method tables.
// Could potentially save a lot of string allocations with lifetimes.
// See also crate::lazy_keys.
#[derive(Eq, PartialEq, Hash)]
pub enum MethodTableKey {
    ClassMethod {
        api_level: ClassCodegenLevel,
        class_ty: TyName,
        method_name: String,
    },
    BuiltinMethod {
        builtin_ty: TyName,
        method_name: String,
    },
    /*BuiltinLifecycleMethod {
        builtin_ty: TyName,
        method_name: String,
    },
    UtilityFunction {
        function_name: String,
    },*/
}

impl MethodTableKey {
    pub fn from_class(class: &Class, method: &ClassMethod) -> MethodTableKey {
        Self::ClassMethod {
            api_level: class.api_level,
            class_ty: class.name().clone(),
            method_name: method.godot_name().to_string(),
        }
    }

    pub fn from_builtin(builtin_class: &BuiltinClass, method: &BuiltinMethod) -> MethodTableKey {
        Self::BuiltinMethod {
            builtin_ty: builtin_class.name().clone(),
            method_name: method.godot_name().to_string(),
        }
    }

    /// Maps the method table key to a "category", meaning a distinct method table.
    ///
    /// Categories have independent address spaces for indices, meaning they begin again at 0 for each new category.
    pub fn category(&self) -> String {
        match self {
            MethodTableKey::ClassMethod { api_level, .. } => format!("class.{}", api_level.lower()),
            MethodTableKey::BuiltinMethod { .. } => "builtin".to_string(),
            // MethodTableKey::BuiltinLifecycleMethod { .. } => "builtin.lifecycle".to_string(),
            // MethodTableKey::UtilityFunction { .. } => "utility".to_string(),
        }
    }
}

impl fmt::Debug for MethodTableKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MethodTableKey::ClassMethod {
                api_level: _,
                class_ty: class_name,
                method_name,
            } => write!(f, "ClassMethod({}.{})", class_name.godot_ty, method_name),
            MethodTableKey::BuiltinMethod {
                builtin_ty: variant_type,
                method_name,
            } => write!(
                f,
                "BuiltinMethod({}.{})",
                variant_type.godot_ty, method_name
            ),
            /*MethodTableKey::BuiltinLifecycleMethod {
                builtin_ty: variant_type,
                method_name,
            } => write!(
                f,
                "BuiltinLifecycleMethod({}.{})",
                variant_type.godot_ty, method_name
            ),
            MethodTableKey::UtilityFunction { function_name } => {
                write!(f, "UtilityFunction({})", function_name)
            }*/
        }
    }
}

// Use &ClassMethod instead of &str, to make sure it's the original Godot name and no rename.
pub(crate) fn make_table_accessor_name(class_ty: &TyName, method: &dyn Function) -> Ident {
    format_ident!(
        "{}__{}",
        conv::to_snake_case(&class_ty.godot_ty),
        method.godot_name()
    )
}
