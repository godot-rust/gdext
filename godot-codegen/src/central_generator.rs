/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use proc_macro2::{Ident, Literal, TokenStream};
use quote::{format_ident, quote, ToTokens, TokenStreamExt};
use std::collections::HashMap;
use std::hash::Hasher;
use std::path::Path;

use crate::api_parser::*;
use crate::util::{
    make_builtin_method_ptr_name, make_class_method_ptr_name, option_as_slice, to_pascal_case,
    to_rust_type, to_snake_case, ClassCodegenLevel, MethodTableKey,
};
use crate::{codegen_special_cases, ident, special_cases, util, Context, SubmitFn, TyName};

struct CentralItems {
    opaque_types: [Vec<TokenStream>; 2],
    variant_ty_enumerators_pascal: Vec<Ident>,
    variant_ty_enumerators_rust: Vec<TokenStream>,
    variant_ty_enumerators_ord: Vec<Literal>,
    variant_op_enumerators_pascal: Vec<Ident>,
    variant_op_enumerators_ord: Vec<Literal>,
    global_enum_defs: Vec<TokenStream>,
    godot_version: Header,
}

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

struct IndexedMethodTable {
    table_name: Ident,
    imports: TokenStream,
    ctor_parameters: TokenStream,
    pre_init_code: TokenStream,
    fptr_type: TokenStream,
    method_inits: Vec<MethodInit>,
    named_accessors: Vec<AccessorMethod>,
    class_count: usize,
    method_count: usize,
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

struct MethodInit {
    method_init: TokenStream,
    index: usize,
}

impl ToTokens for MethodInit {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.method_init.to_tokens(tokens);
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

struct AccessorMethod {
    name: Ident,
    index: usize,
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

pub struct TypeNames {
    /// Name in JSON: "int" or "PackedVector2Array"
    pub json_builtin_name: String,

    /// "packed_vector2_array"
    pub snake_case: String,

    /// "PACKED_VECTOR2_ARRAY"
    //pub shout_case: String,

    /// GDEXTENSION_VARIANT_TYPE_PACKED_VECTOR2_ARRAY
    pub sys_variant_type: Ident,
}

impl Eq for TypeNames {}

impl PartialEq for TypeNames {
    fn eq(&self, other: &Self) -> bool {
        self.json_builtin_name == other.json_builtin_name
    }
}

impl std::hash::Hash for TypeNames {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.json_builtin_name.hash(state);
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

/// Allows collecting all builtin TypeNames before generating methods
pub(crate) struct BuiltinTypeInfo<'a> {
    pub value: i32,
    pub type_names: TypeNames,

    /// If `variant_get_ptr_destructor` returns a non-null function pointer for this type.
    /// List is directly sourced from extension_api.json (information would also be in variant_destruct.cpp).
    pub has_destructor: bool,
    pub constructors: Option<&'a Vec<Constructor>>,
    pub operators: Option<&'a Vec<Operator>>,
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

pub(crate) struct BuiltinTypeMap<'a> {
    map: HashMap<String, BuiltinTypeInfo<'a>>,
}

impl<'a> BuiltinTypeMap<'a> {
    pub fn load(api: &'a ExtensionApi) -> Self {
        Self {
            map: collect_builtin_types(api),
        }
    }

    /// Returns an iterator over the builtin types, ordered by `VariantType` value.
    fn ordered(&self) -> impl Iterator<Item = &BuiltinTypeInfo<'a>> {
        let mut ordered: Vec<_> = self.map.values().collect();
        ordered.sort_by_key(|info| info.value);
        ordered.into_iter()
    }

    fn count(&self) -> usize {
        self.map.len()
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

pub(crate) fn generate_sys_central_file(
    api: &ExtensionApi,
    ctx: &mut Context,
    build_config: [&str; 2],
    sys_gen_path: &Path,
    submit_fn: &mut SubmitFn,
) {
    let builtin_types = BuiltinTypeMap::load(api);
    let central_items = make_central_items(api, build_config, builtin_types, ctx);
    let sys_code = make_sys_code(&central_items);

    submit_fn(sys_gen_path.join("central.rs"), sys_code);
}

pub(crate) fn generate_sys_classes_file(
    api: &ExtensionApi,
    sys_gen_path: &Path,
    watch: &mut godot_bindings::StopWatch,
    ctx: &mut Context,
    submit_fn: &mut SubmitFn,
) {
    for api_level in ClassCodegenLevel::with_tables() {
        let code = make_class_method_table(api, api_level, ctx);
        let filename = api_level.table_file();

        submit_fn(sys_gen_path.join(filename), code);
        watch.record(format!("generate_classes_{}_file", api_level.lower()));
    }
}

pub(crate) fn generate_sys_utilities_file(
    api: &ExtensionApi,
    sys_gen_path: &Path,
    ctx: &mut Context,
    submit_fn: &mut SubmitFn,
) {
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
        if codegen_special_cases::is_function_excluded(function, ctx) {
            continue;
        }

        let fn_name_str = &function.name;
        let field = util::make_utility_function_ptr_name(function);
        let hash = function.hash;

        table.method_decls.push(quote! {
            pub #field: crate::UtilityFunctionBind,
        });

        table.method_inits.push(quote! {
            #field: crate::load_utility_function(get_utility_fn, string_names, #fn_name_str, #hash),
        });

        table.method_count += 1;
    }

    let code = make_named_method_table(table);

    submit_fn(sys_gen_path.join("table_utilities.rs"), code);
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

            pub fn load(
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

fn make_indexed_method_table(info: IndexedMethodTable) -> TokenStream {
    let IndexedMethodTable {
        table_name,
        imports,
        ctor_parameters,
        pre_init_code,
        fptr_type,
        mut method_inits,
        named_accessors,
        class_count,
        method_count,
    } = info;

    // Editor table can be empty, if the Godot binary is compiled without editor.
    let unused_attr = (method_count == 0).then(|| quote! { #[allow(unused_variables)] });
    let named_method_api = make_named_accessors(&named_accessors, &fptr_type);

    // Make sure methods are complete and in order of index.
    assert_eq!(
        method_inits.len(),
        method_count,
        "number of methods does not match count"
    );
    method_inits.sort_by_key(|init| init.index);

    if let Some(last) = method_inits.last() {
        assert_eq!(
            last.index,
            method_count - 1,
            "last method should have highest index"
        );
    } else {
        assert_eq!(method_count, 0, "empty method table should have count 0");
    }

    // Assumes that inits already have a trailing comma.
    // This is necessary because some generators emit multiple lines (statements) per element.
    quote! {
        #imports

        pub struct #table_name {
            function_pointers: Vec<#fptr_type>,
        }

        impl #table_name {
            pub const CLASS_COUNT: usize = #class_count;
            pub const METHOD_COUNT: usize = #method_count;

            #unused_attr
            pub fn load(
                #ctor_parameters
            ) -> Self {
                #pre_init_code

                Self {
                    function_pointers: vec![
                        #( #method_inits )*
                    ]
                }
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
    }
}

pub(crate) fn generate_sys_builtin_methods_file(
    api: &ExtensionApi,
    builtin_types: &BuiltinTypeMap,
    sys_gen_path: &Path,
    ctx: &mut Context,
    submit_fn: &mut SubmitFn,
) {
    let code = make_builtin_method_table(api, builtin_types, ctx);
    submit_fn(sys_gen_path.join("table_builtins.rs"), code);
}

pub(crate) fn generate_sys_builtin_lifecycle_file(
    builtin_types: &BuiltinTypeMap,
    sys_gen_path: &Path,
    submit_fn: &mut SubmitFn,
) {
    // TODO merge this and the one in central.rs, to only collect once
    let code = make_builtin_lifecycle_table(builtin_types);
    submit_fn(sys_gen_path.join("table_builtins_lifecycle.rs"), code);
}

pub(crate) fn generate_core_mod_file(gen_path: &Path, submit_fn: &mut SubmitFn) {
    // When invoked by another crate during unit-test (not integration test), don't run generator
    let code = quote! {
        pub mod central;
        pub mod classes;
        pub mod builtin_classes;
        pub mod utilities;
        pub mod native;
    };

    submit_fn(gen_path.join("mod.rs"), code);
}

pub(crate) fn generate_core_central_file(
    api: &ExtensionApi,
    ctx: &mut Context,
    build_config: [&str; 2],
    gen_path: &Path,
    submit_fn: &mut SubmitFn,
) {
    let builtin_types = BuiltinTypeMap::load(api);
    let central_items = make_central_items(api, build_config, builtin_types, ctx);
    let core_code = make_core_code(&central_items);

    submit_fn(gen_path.join("central.rs"), core_code);
}

fn make_sys_code(central_items: &CentralItems) -> TokenStream {
    let CentralItems {
        opaque_types,
        variant_ty_enumerators_pascal,
        variant_ty_enumerators_ord,
        variant_op_enumerators_pascal,
        variant_op_enumerators_ord,
        godot_version,
        ..
    } = central_items;

    let build_config_struct = make_build_config(godot_version);
    let [opaque_32bit, opaque_64bit] = opaque_types;

    quote! {
        use crate::{GDExtensionVariantOperator, GDExtensionVariantType};

        #[cfg(target_pointer_width = "32")]
        pub mod types {
            #(#opaque_32bit)*
        }
        #[cfg(target_pointer_width = "64")]
        pub mod types {
            #(#opaque_64bit)*
        }


        // ----------------------------------------------------------------------------------------------------------------------------------------------

        #build_config_struct

        // ----------------------------------------------------------------------------------------------------------------------------------------------

        #[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
        #[repr(i32)]
        pub enum VariantType {
            Nil = 0,
            #(
                #variant_ty_enumerators_pascal = #variant_ty_enumerators_ord,
            )*
        }

        impl VariantType {
            #[doc(hidden)]
            pub fn from_sys(enumerator: GDExtensionVariantType) -> Self {
                // Annoying, but only stable alternative is transmute(), which dictates enum size
                match enumerator {
                    0 => Self::Nil,
                    #(
                        #variant_ty_enumerators_ord => Self::#variant_ty_enumerators_pascal,
                    )*
                    _ => unreachable!("invalid variant type {}", enumerator)
                }
            }

            #[doc(hidden)]
            pub fn sys(self) -> GDExtensionVariantType {
                self as _
            }
        }

        // ----------------------------------------------------------------------------------------------------------------------------------------------

        #[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
        #[repr(i32)]
        pub enum VariantOperator {
            #(
                #variant_op_enumerators_pascal = #variant_op_enumerators_ord,
            )*
        }

        impl VariantOperator {
            #[doc(hidden)]
            pub fn from_sys(enumerator: GDExtensionVariantOperator) -> Self {
                match enumerator {
                    #(
                        #variant_op_enumerators_ord => Self::#variant_op_enumerators_pascal,
                    )*
                    _ => unreachable!("invalid variant operator {}", enumerator)
                }
            }

            #[doc(hidden)]
            pub fn sys(self) -> GDExtensionVariantOperator {
                self as _
            }
        }
    }
}

fn make_build_config(header: &Header) -> TokenStream {
    let version_string = header
        .version_full_name
        .strip_prefix("Godot Engine ")
        .unwrap_or(&header.version_full_name);
    let major = header.version_major;
    let minor = header.version_minor;
    let patch = header.version_patch;

    // Should this be mod?
    quote! {
        /// Provides meta-information about the library and the Godot version in use.
        pub struct GdextBuild;

        impl GdextBuild {
            /// Godot version against which gdext was compiled.
            ///
            /// Example format: `v4.0.stable.official`
            pub const fn godot_static_version_string() -> &'static str {
                #version_string
            }

            /// Godot version against which gdext was compiled, as `(major, minor, patch)` triple.
            pub const fn godot_static_version_triple() -> (u8, u8, u8) {
                (#major, #minor, #patch)
            }

            /// Version of the Godot engine which loaded gdext via GDExtension binding.
            pub fn godot_runtime_version_string() -> String {
                unsafe {
                    let char_ptr = crate::runtime_metadata().godot_version.string;
                    let c_str = std::ffi::CStr::from_ptr(char_ptr);
                    String::from_utf8_lossy(c_str.to_bytes()).to_string()
                }
            }

            /// Version of the Godot engine which loaded gdext via GDExtension binding, as
            /// `(major, minor, patch)` triple.
            pub fn godot_runtime_version_triple() -> (u8, u8, u8) {
                let version = unsafe {
                    crate::runtime_metadata().godot_version
                };
                (version.major as u8, version.minor as u8, version.patch as u8)
            }
        }
    }
}

fn make_core_code(central_items: &CentralItems) -> TokenStream {
    let CentralItems {
        variant_ty_enumerators_pascal,
        variant_ty_enumerators_rust,
        global_enum_defs,
        ..
    } = central_items;

    // TODO impl Clone, Debug, PartialEq, PartialOrd, Hash for VariantDispatch
    // TODO could use try_to().unwrap_unchecked(), since type is already verified. Also directly overload from_variant().
    // But this requires that all the variant types support this
    quote! {
        use crate::builtin::*;
        use crate::engine::Object;
        use crate::obj::Gd;

        #[allow(dead_code)]
        pub enum VariantDispatch {
            Nil,
            #(
                #variant_ty_enumerators_pascal(#variant_ty_enumerators_rust),
            )*
        }

        #[cfg(FALSE)]
        impl FromVariant for VariantDispatch {
            fn try_from_variant(variant: &Variant) -> Result<Self, VariantConversionError> {
                let dispatch = match variant.get_type() {
                    VariantType::Nil => Self::Nil,
                    #(
                        VariantType::#variant_ty_enumerators_pascal
                            => Self::#variant_ty_enumerators_pascal(variant.to::<#variant_ty_enumerators_rust>()),
                    )*
                };

                Ok(dispatch)
            }
        }

        /// Global enums and constants.
        ///
        /// A list of global-scope enumerated constants.
        /// For global built-in functions, check out the [`utilities` module][crate::engine::utilities].
        ///
        /// See also [Godot docs for `@GlobalScope`](https://docs.godotengine.org/en/stable/classes/class_@globalscope.html#enumerations).
        pub mod global {
            use crate::sys;
            #( #global_enum_defs )*
        }
    }
}

fn make_central_items(
    api: &ExtensionApi,
    build_config: [&str; 2],
    builtin_types: BuiltinTypeMap,
    ctx: &mut Context,
) -> CentralItems {
    let mut opaque_types = [Vec::new(), Vec::new()];
    for class in &api.builtin_class_sizes {
        for i in 0..2 {
            if class.build_configuration == build_config[i] {
                for ClassSize { name, size } in &class.sizes {
                    opaque_types[i].push(make_opaque_type(name, *size));
                }
                break;
            }
        }
    }

    let variant_operators = collect_variant_operators(api);

    // Generate builtin methods, now with info for all types available.
    // Separate vectors because that makes usage in quote! easier.
    let len = builtin_types.count();

    let mut result = CentralItems {
        opaque_types,
        variant_ty_enumerators_pascal: Vec::with_capacity(len),
        variant_ty_enumerators_rust: Vec::with_capacity(len),
        variant_ty_enumerators_ord: Vec::with_capacity(len),
        variant_op_enumerators_pascal: Vec::new(),
        variant_op_enumerators_ord: Vec::new(),
        global_enum_defs: Vec::new(),
        godot_version: api.header.clone(),
    };

    // Note: NIL is not part of this iteration, it will be added manually
    for ty in builtin_types.ordered() {
        let (pascal_name, rust_ty, ord) = make_enumerator(&ty.type_names, ty.value, ctx);

        result.variant_ty_enumerators_pascal.push(pascal_name);
        result.variant_ty_enumerators_rust.push(rust_ty);
        result.variant_ty_enumerators_ord.push(ord);
    }

    for op in variant_operators {
        let name = op
            .name
            .strip_prefix("OP_")
            .expect("expected `OP_` prefix for variant operators");

        if name == "MAX" {
            continue;
        }

        let op_enumerator_pascal = util::shout_to_pascal(name);
        let op_enumerator_pascal = if op_enumerator_pascal == "Module" {
            "Modulo"
        } else {
            &op_enumerator_pascal
        };

        result
            .variant_op_enumerators_pascal
            .push(ident(op_enumerator_pascal));
        result
            .variant_op_enumerators_ord
            .push(util::make_enumerator_ord(op.value));
    }

    for enum_ in api.global_enums.iter() {
        // Skip those enums which are already explicitly handled
        if matches!(enum_.name.as_str(), "Variant.Type" | "Variant.Operator") {
            continue;
        }

        let def = util::make_enum_definition(enum_);
        result.global_enum_defs.push(def);
    }

    result
}

fn make_builtin_lifecycle_table(builtin_types: &BuiltinTypeMap) -> TokenStream {
    let len = builtin_types.count();
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

    // Note: NIL is not part of this iteration, it will be added manually
    for ty in builtin_types.ordered() {
        let (decls, inits) = make_variant_fns(
            &ty.type_names,
            ty.has_destructor,
            ty.constructors,
            ty.operators,
            &builtin_types.map,
        );

        table.method_decls.push(decls);
        table.method_inits.push(inits);
    }

    make_named_method_table(table)
}

fn make_class_method_table(
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
        method_inits: vec![],
        named_accessors: vec![],
        class_count: 0,
        method_count: 0,
    };

    let mut class_sname_decls = Vec::new();
    for class in api.classes.iter() {
        let class_ty = TyName::from_godot(&class.name);
        if special_cases::is_class_deleted(&class_ty)
            || codegen_special_cases::is_class_excluded(&class.name)
            || util::get_api_level(class) != api_level
        {
            continue;
        }

        let class_var = format_ident!("sname_{}", &class.name);
        let initializer_expr = util::make_sname_ptr(&class.name);

        let prev_method_count = table.method_count;
        populate_class_methods(&mut table, class, &class_ty, &class_var, ctx);
        if table.method_count > prev_method_count {
            // Only create class variable if any methods have been added.
            class_sname_decls.push(quote! {
                let #class_var = #initializer_expr;
            });
        }

        table.class_count += 1;
    }

    table.pre_init_code = quote! {
        let get_method_bind = interface.classdb_get_method_bind.expect("classdb_get_method_bind absent");

        #( #class_sname_decls )*
    };

    make_indexed_method_table(table)
}

/// For index-based method tables, have select methods exposed by name for internal use.
fn make_named_accessors(accessors: &[AccessorMethod], fptr: &TokenStream) -> TokenStream {
    let mut result_api = TokenStream::new();

    for AccessorMethod { name, index } in accessors {
        let code = quote! {
            #[inline(always)]
            pub fn #name(&self) -> #fptr {
                self.fptr_by_index(#index)
            }
        };

        result_api.append_all(code.into_iter());
    }
    result_api
}

fn make_builtin_method_table(
    api: &ExtensionApi,
    builtin_types: &BuiltinTypeMap,
    ctx: &mut Context,
) -> TokenStream {
    let mut table = IndexedMethodTable {
        table_name: ident("BuiltinMethodTable"),
        imports: TokenStream::new(),
        ctor_parameters: quote! {
            interface: &crate::GDExtensionInterface,
            string_names: &mut crate::StringCache,
        },
        pre_init_code: quote! {
            use crate as sys;
            let get_builtin_method = interface.variant_get_ptr_builtin_method.expect("variant_get_ptr_builtin_method absent");
        },
        fptr_type: quote! { crate::BuiltinMethodBind },
        method_inits: vec![],
        named_accessors: vec![],
        class_count: 0,
        method_count: 0,
    };

    // TODO reuse builtin_types without api
    for builtin in api.builtin_classes.iter() {
        let Some(builtin_type) = builtin_types.map.get(&builtin.name) else {
            continue; // for Nil
        };

        populate_builtin_methods(&mut table, builtin, &builtin_type.type_names, ctx);
        table.class_count += 1;
    }

    make_indexed_method_table(table)
}

fn populate_class_methods(
    table: &mut IndexedMethodTable,
    class: &Class,
    class_ty: &TyName,
    class_var: &Ident,
    ctx: &mut Context,
) {
    for method in option_as_slice(&class.methods) {
        if special_cases::is_deleted(class_ty, method, ctx) {
            continue;
        }

        // Note: varcall/ptrcall is only decided at call time; the method bind is the same for both.
        let index = ctx.get_table_index(&MethodTableKey::ClassMethod {
            api_level: util::get_api_level(class),
            class_ty: class_ty.clone(),
            method_name: method.name.clone(),
        });
        let method_init = make_class_method_init(method, class_var, class_ty);

        table.method_inits.push(MethodInit { method_init, index });
        table.method_count += 1;

        // If requested, add a named accessor for this method.
        if special_cases::is_named_accessor_in_table(class_ty, &method.name) {
            table.named_accessors.push(AccessorMethod {
                name: make_class_method_ptr_name(class_ty, method),
                index,
            });
        }
    }
}

fn populate_builtin_methods(
    table: &mut IndexedMethodTable,
    builtin_class: &BuiltinClass,
    builtin_name: &TypeNames,
    ctx: &mut Context,
) {
    for method in option_as_slice(&builtin_class.methods) {
        let builtin_ty = TyName::from_godot(&builtin_class.name);
        if special_cases::is_builtin_deleted(&builtin_ty, method) {
            continue;
        }

        let index = ctx.get_table_index(&MethodTableKey::BuiltinMethod {
            builtin_ty: builtin_ty.clone(),
            method_name: method.name.clone(),
        });

        let method_init = make_builtin_method_init(method, builtin_name, index);

        table.method_inits.push(MethodInit { method_init, index });
        table.method_count += 1;

        // If requested, add a named accessor for this method.
        if special_cases::is_named_accessor_in_table(&builtin_ty, &method.name) {
            table.named_accessors.push(AccessorMethod {
                name: make_builtin_method_ptr_name(&builtin_ty, method),
                index,
            });
        }
    }
}

fn make_class_method_init(
    method: &ClassMethod,
    class_var: &Ident,
    class_ty: &TyName,
) -> TokenStream {
    let class_name_str = class_ty.godot_ty.as_str();
    let method_name_str = method.name.as_str();

    let hash = method.hash.unwrap_or_else(|| {
        panic!(
            "class method has no hash: {}::{}",
            class_ty.godot_ty, method_name_str
        )
    });

    quote! {
        crate::load_class_method(get_method_bind, string_names, #class_var, #class_name_str, #method_name_str, #hash),
    }
}

fn make_builtin_method_init(
    method: &BuiltinClassMethod,
    type_name: &TypeNames,
    index: usize,
) -> TokenStream {
    let method_name_str = method.name.as_str();

    let variant_type = &type_name.sys_variant_type;
    let variant_type_str = &type_name.json_builtin_name;

    let hash = method.hash.unwrap_or_else(|| {
        panic!(
            "builtin method has no hash: {}::{}",
            variant_type_str, method_name_str
        )
    });

    quote! {
        {let _ = #index;crate::load_builtin_method(get_builtin_method, string_names, sys::#variant_type, #variant_type_str, #method_name_str, #hash)},
    }
}

/// Creates a map from "normalized" class names (lowercase without underscore, makes it easy to map from different conventions)
/// to meta type information, including all the type name variants
fn collect_builtin_classes(api: &ExtensionApi) -> HashMap<String, &BuiltinClass> {
    let mut class_map = HashMap::new();
    for class in &api.builtin_classes {
        let normalized_name = class.name.to_ascii_lowercase();

        class_map.insert(normalized_name, class);
    }

    class_map
}

/// Returns map from the JSON names (e.g. "PackedStringArray") to all the info.
pub(crate) fn collect_builtin_types(api: &ExtensionApi) -> HashMap<String, BuiltinTypeInfo<'_>> {
    let class_map = collect_builtin_classes(api);

    let variant_type_enum = api
        .global_enums
        .iter()
        .find(|e| &e.name == "Variant.Type")
        .expect("missing enum for VariantType in JSON");

    // Collect all `BuiltinTypeInfo`s
    let mut builtin_types_map = HashMap::new();
    for ty in &variant_type_enum.values {
        let shout_case = ty
            .name
            .strip_prefix("TYPE_")
            .expect("enum name begins with 'TYPE_'");

        if shout_case == "NIL" || shout_case == "MAX" {
            continue;
        }

        // Lowercase without underscore, to map SHOUTY_CASE to shoutycase
        let normalized = shout_case.to_ascii_lowercase().replace('_', "");

        // TODO cut down on the number of cached functions generated
        // e.g. there's no point in providing operator< for int
        let class_name: String;
        let has_destructor: bool;
        let constructors: Option<&Vec<Constructor>>;
        let operators: Option<&Vec<Operator>>;
        if let Some(class) = class_map.get(&normalized) {
            class_name = class.name.clone();
            has_destructor = class.has_destructor;
            constructors = Some(&class.constructors);
            operators = Some(&class.operators);
        } else {
            assert_eq!(normalized, "object");
            class_name = "Object".to_string();
            has_destructor = false;
            constructors = None;
            operators = None;
        }

        let type_names = TypeNames {
            json_builtin_name: class_name.clone(),
            snake_case: to_snake_case(&class_name),
            //shout_case: shout_case.to_string(),
            sys_variant_type: format_ident!("GDEXTENSION_VARIANT_TYPE_{}", shout_case),
        };

        let value = ty.value;

        builtin_types_map.insert(
            type_names.json_builtin_name.clone(),
            BuiltinTypeInfo {
                value,
                type_names,
                has_destructor,
                constructors,
                operators,
            },
        );
    }
    builtin_types_map
}

fn collect_variant_operators(api: &ExtensionApi) -> Vec<&EnumConstant> {
    let variant_operator_enum = api
        .global_enums
        .iter()
        .find(|e| &e.name == "Variant.Operator")
        .expect("missing enum for VariantOperator in JSON");

    variant_operator_enum.values.iter().collect()
}

fn make_enumerator(
    type_names: &TypeNames,
    value: i32,
    ctx: &mut Context,
) -> (Ident, TokenStream, Literal) {
    let enumerator_name = &type_names.json_builtin_name;
    let pascal_name = to_pascal_case(enumerator_name);
    let rust_ty = to_rust_type(enumerator_name, None, ctx);
    let ord = util::make_enumerator_ord(value);

    (ident(&pascal_name), rust_ty.to_token_stream(), ord)
}

fn make_opaque_type(name: &str, size: usize) -> TokenStream {
    let name = to_pascal_case(name);
    let (first, rest) = name.split_at(1);

    // Capitalize: "int" -> "Int"
    let ident = format_ident!("Opaque{}{}", first.to_ascii_uppercase(), rest);
    quote! {
        pub type #ident = crate::opaque::Opaque<#size>;
    }
}

fn make_variant_fns(
    type_names: &TypeNames,
    has_destructor: bool,
    constructors: Option<&Vec<Constructor>>,
    operators: Option<&Vec<Operator>>,
    builtin_types: &HashMap<String, BuiltinTypeInfo>,
) -> (TokenStream, TokenStream) {
    let (construct_decls, construct_inits) =
        make_construct_fns(type_names, constructors, builtin_types);
    let (destroy_decls, destroy_inits) = make_destroy_fns(type_names, has_destructor);
    let (op_eq_decls, op_eq_inits) = make_operator_fns(type_names, operators, "==", "EQUAL");
    let (op_lt_decls, op_lt_inits) = make_operator_fns(type_names, operators, "<", "LESS");

    let to_variant = format_ident!("{}_to_variant", type_names.snake_case);
    let from_variant = format_ident!("{}_from_variant", type_names.snake_case);

    let to_variant_str = to_variant.to_string();
    let from_variant_str = from_variant.to_string();

    let variant_type = &type_names.sys_variant_type;
    let variant_type = quote! { crate::#variant_type };

    // Field declaration
    // The target types are uninitialized-ptrs, because Godot performs placement new on those:
    // https://github.com/godotengine/godot/blob/b40b35fb39f0d0768d7ec2976135adffdce1b96d/core/variant/variant_internal.h#L1535-L1535

    let decl = quote! {
        pub #to_variant: unsafe extern "C" fn(GDExtensionUninitializedVariantPtr, GDExtensionTypePtr),
        pub #from_variant: unsafe extern "C" fn(GDExtensionUninitializedTypePtr, GDExtensionVariantPtr),
        #op_eq_decls
        #op_lt_decls
        #construct_decls
        #destroy_decls
    };

    // Field initialization in new()
    let init = quote! {
        #to_variant: {
            let fptr = unsafe { get_to_variant_fn(#variant_type) };
            crate::validate_builtin_lifecycle(fptr, #to_variant_str)
        },
        #from_variant: {
            let fptr = unsafe { get_from_variant_fn(#variant_type) };
            crate::validate_builtin_lifecycle(fptr, #from_variant_str)
        },
        #op_eq_inits
        #op_lt_inits
        #construct_inits
        #destroy_inits
    };

    (decl, init)
}

fn make_construct_fns(
    type_names: &TypeNames,
    constructors: Option<&Vec<Constructor>>,
    builtin_types: &HashMap<String, BuiltinTypeInfo>,
) -> (TokenStream, TokenStream) {
    let constructors = match constructors {
        Some(c) => c,
        None => return (TokenStream::new(), TokenStream::new()),
    };

    if is_trivial(type_names) {
        return (TokenStream::new(), TokenStream::new());
    }

    // Constructor vec layout:
    //   [0]: default constructor
    //   [1]: copy constructor
    //   [2]: (optional) typically the most common conversion constructor (e.g. StringName -> String)
    //  rest: (optional) other conversion constructors and multi-arg constructors (e.g. Vector3(x, y, z))

    // Sanity checks -- ensure format is as expected
    for (i, c) in constructors.iter().enumerate() {
        assert_eq!(i, c.index);
    }

    assert!(constructors[0].arguments.is_none());

    if let Some(args) = &constructors[1].arguments {
        assert_eq!(args.len(), 1);
        assert_eq!(args[0].name, "from");
        assert_eq!(args[0].type_, type_names.json_builtin_name);
    } else {
        panic!(
            "type {}: no constructor args found for copy constructor",
            type_names.json_builtin_name
        );
    }

    let construct_default = format_ident!("{}_construct_default", type_names.snake_case);
    let construct_copy = format_ident!("{}_construct_copy", type_names.snake_case);
    let construct_default_str = construct_default.to_string();
    let construct_copy_str = construct_copy.to_string();
    let variant_type = &type_names.sys_variant_type;

    let (construct_extra_decls, construct_extra_inits) =
        make_extra_constructors(type_names, constructors, builtin_types);

    // Target types are uninitialized pointers, because Godot uses placement-new for raw pointer constructions. Callstack:
    // https://github.com/godotengine/godot/blob/b40b35fb39f0d0768d7ec2976135adffdce1b96d/core/extension/gdextension_interface.cpp#L511
    // https://github.com/godotengine/godot/blob/b40b35fb39f0d0768d7ec2976135adffdce1b96d/core/variant/variant_construct.cpp#L299
    // https://github.com/godotengine/godot/blob/b40b35fb39f0d0768d7ec2976135adffdce1b96d/core/variant/variant_construct.cpp#L36
    // https://github.com/godotengine/godot/blob/b40b35fb39f0d0768d7ec2976135adffdce1b96d/core/variant/variant_construct.h#L267
    // https://github.com/godotengine/godot/blob/b40b35fb39f0d0768d7ec2976135adffdce1b96d/core/variant/variant_construct.h#L50
    let decls = quote! {
        pub #construct_default: unsafe extern "C" fn(GDExtensionUninitializedTypePtr, *const GDExtensionConstTypePtr),
        pub #construct_copy: unsafe extern "C" fn(GDExtensionUninitializedTypePtr, *const GDExtensionConstTypePtr),
        #(
            #construct_extra_decls
        )*
    };

    let inits = quote! {
        #construct_default: {
            let fptr = unsafe { get_construct_fn(crate::#variant_type, 0i32) };
            crate::validate_builtin_lifecycle(fptr, #construct_default_str)
        },
        #construct_copy: {
            let fptr = unsafe { get_construct_fn(crate::#variant_type, 1i32) };
            crate::validate_builtin_lifecycle(fptr, #construct_copy_str)
        },
        #(
            #construct_extra_inits
        )*
    };

    (decls, inits)
}

/// Lists special cases for useful constructors
fn make_extra_constructors(
    type_names: &TypeNames,
    constructors: &Vec<Constructor>,
    builtin_types: &HashMap<String, BuiltinTypeInfo>,
) -> (Vec<TokenStream>, Vec<TokenStream>) {
    let mut extra_decls = Vec::with_capacity(constructors.len() - 2);
    let mut extra_inits = Vec::with_capacity(constructors.len() - 2);
    let variant_type = &type_names.sys_variant_type;

    for (i, ctor) in constructors.iter().enumerate().skip(2) {
        if let Some(args) = &ctor.arguments {
            let type_name = &type_names.snake_case;
            let construct_custom = if args.len() == 1 && args[0].name == "from" {
                // Conversion constructor is named according to the source type
                // String(NodePath from) => string_from_node_path
                let arg_type = &builtin_types[&args[0].type_].type_names.snake_case;
                format_ident!("{type_name}_from_{arg_type}")
            } else {
                // Type-specific constructor is named according to the argument names
                // Vector3(float x, float y, float z) => vector3_from_x_y_z
                let mut arg_names = args
                    .iter()
                    .fold(String::new(), |acc, arg| acc + &arg.name + "_");
                arg_names.pop(); // remove trailing '_'
                format_ident!("{type_name}_from_{arg_names}")
            };

            let construct_custom_str = construct_custom.to_string();
            extra_decls.push(quote! {
                pub #construct_custom: unsafe extern "C" fn(GDExtensionUninitializedTypePtr, *const GDExtensionConstTypePtr),
            });

            let i = i as i32;
            extra_inits.push(quote! {
                #construct_custom: {
                    let fptr = unsafe { get_construct_fn(crate::#variant_type, #i) };
                    crate::validate_builtin_lifecycle(fptr, #construct_custom_str)
                },
            });
        }
    }

    (extra_decls, extra_inits)
}

fn make_destroy_fns(type_names: &TypeNames, has_destructor: bool) -> (TokenStream, TokenStream) {
    if !has_destructor || is_trivial(type_names) {
        return (TokenStream::new(), TokenStream::new());
    }

    let destroy = format_ident!("{}_destroy", type_names.snake_case);
    let destroy_str = destroy.to_string();
    let variant_type = &type_names.sys_variant_type;

    let decls = quote! {
        pub #destroy: unsafe extern "C" fn(GDExtensionTypePtr),
    };

    let inits = quote! {
        #destroy: {
            let fptr = unsafe { get_destroy_fn(crate::#variant_type) };
            crate::validate_builtin_lifecycle(fptr, #destroy_str)
        },
    };

    (decls, inits)
}

fn make_operator_fns(
    type_names: &TypeNames,
    operators: Option<&Vec<Operator>>,
    json_name: &str,
    sys_name: &str,
) -> (TokenStream, TokenStream) {
    if operators.is_none()
        || !operators.unwrap().iter().any(|op| op.name == json_name)
        || is_trivial(type_names)
    {
        return (TokenStream::new(), TokenStream::new());
    }

    let operator = format_ident!(
        "{}_operator_{}",
        type_names.snake_case,
        sys_name.to_ascii_lowercase()
    );
    let operator_str = operator.to_string();

    let variant_type = &type_names.sys_variant_type;
    let variant_type = quote! { crate::#variant_type };
    let sys_ident = format_ident!("GDEXTENSION_VARIANT_OP_{}", sys_name);

    // Field declaration
    let decl = quote! {
        pub #operator: unsafe extern "C" fn(GDExtensionConstTypePtr, GDExtensionConstTypePtr, GDExtensionTypePtr),
    };

    // Field initialization in new()
    let init = quote! {
        #operator: {
            let fptr = unsafe { get_operator_fn(crate::#sys_ident, #variant_type, #variant_type) };
            crate::validate_builtin_lifecycle(fptr, #operator_str)
        },
    };

    (decl, init)
}

/// Returns true if the type is so trivial that most of its operations are directly provided by Rust, and there is no need
/// to expose the construct/destruct/operator methods from Godot
fn is_trivial(type_names: &TypeNames) -> bool {
    let list = ["bool", "int", "float"];

    list.contains(&type_names.json_builtin_name.as_str())
}
