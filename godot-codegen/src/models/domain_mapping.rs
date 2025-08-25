/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::collections::HashMap;

use proc_macro2::Ident;

use crate::context::Context;
use crate::models::domain::{
    BuildConfiguration, BuiltinClass, BuiltinMethod, BuiltinSize, BuiltinVariant, Class,
    ClassCommons, ClassConstant, ClassConstantValue, ClassMethod, ClassSignal, Constructor, Enum,
    Enumerator, EnumeratorValue, ExtensionApi, FnDirection, FnParam, FnQualifier, FnReturn,
    FunctionCommon, GodotApiVersion, ModName, NativeStructure, Operator, RustTy, Singleton, TyName,
    UtilityFunction,
};
use crate::models::json::{
    JsonBuiltinClass, JsonBuiltinMethod, JsonBuiltinSizes, JsonClass, JsonClassConstant,
    JsonClassMethod, JsonConstructor, JsonEnum, JsonEnumConstant, JsonExtensionApi, JsonHeader,
    JsonMethodReturn, JsonNativeStructure, JsonOperator, JsonSignal, JsonSingleton,
    JsonUtilityFunction,
};
use crate::util::{get_api_level, ident, option_as_slice};
use crate::{conv, special_cases};

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Top-level

impl ExtensionApi {
    pub fn from_json(json: &JsonExtensionApi, ctx: &mut Context) -> Self {
        Self {
            builtins: BuiltinVariant::all_from_json(&json.global_enums, &json.builtin_classes, ctx),
            classes: json
                .classes
                .iter()
                .filter_map(|json| Class::from_json(json, ctx))
                .collect(),
            singletons: json.singletons.iter().map(Singleton::from_json).collect(),
            native_structures: json
                .native_structures
                .iter()
                .map(NativeStructure::from_json)
                .collect(),
            utility_functions: json
                .utility_functions
                .iter()
                .filter_map(|json| UtilityFunction::from_json(json, ctx))
                .collect(),
            global_enums: json
                .global_enums
                .iter()
                .map(|json| Enum::from_json(json, None))
                .collect(),
            godot_version: GodotApiVersion::from_json(&json.header),
            builtin_sizes: Self::builtin_size_from_json(&json.builtin_class_sizes),
        }
    }

    fn builtin_size_from_json(json_builtin_sizes: &[JsonBuiltinSizes]) -> Vec<BuiltinSize> {
        let mut result = Vec::new();

        for json_builtin_size in json_builtin_sizes {
            let build_config_str = json_builtin_size.build_configuration.as_str();
            let config = BuildConfiguration::from_json(build_config_str);

            if config.is_applicable() {
                for size_for_config in &json_builtin_size.sizes {
                    result.push(BuiltinSize {
                        builtin_original_name: size_for_config.name.clone(),
                        config,
                        size: size_for_config.size,
                    });
                }
            }
        }

        result
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Builtins + classes + singletons

impl Class {
    pub fn from_json(json: &JsonClass, ctx: &mut Context) -> Option<Self> {
        let ty_name = TyName::from_godot(&json.name);
        if special_cases::is_class_deleted(&ty_name) {
            return None;
        }

        // Already checked in is_class_deleted(), but code remains more maintainable if those are separate, and it's cheap to validate.
        let is_experimental = special_cases::is_class_experimental(&ty_name.godot_ty);

        let is_instantiable = special_cases::is_class_instantiable(&ty_name) //.
            .unwrap_or(json.is_instantiable);

        let is_final = ctx.is_final(&ty_name);

        let mod_name = ModName::from_godot(&ty_name.godot_ty);

        let constants = option_as_slice(&json.constants)
            .iter()
            .map(ClassConstant::from_json)
            .collect();

        let enums = option_as_slice(&json.enums)
            .iter()
            .map(|e| {
                let surrounding_class = Some(&ty_name);
                Enum::from_json(e, surrounding_class)
            })
            .collect();

        let methods = option_as_slice(&json.methods)
            .iter()
            .filter_map(|m| {
                let surrounding_class = &ty_name;
                ClassMethod::from_json(m, surrounding_class, ctx)
            })
            .collect();

        let signals = option_as_slice(&json.signals)
            .iter()
            .filter_map(|s| {
                let surrounding_class = &ty_name;
                ClassSignal::from_json(s, surrounding_class, ctx)
            })
            .collect();

        let base_class = json
            .inherits
            .as_ref()
            .map(|godot_name| TyName::from_godot(godot_name));

        Some(Self {
            common: ClassCommons {
                name: ty_name,
                mod_name,
            },
            is_refcounted: json.is_refcounted,
            is_instantiable,
            is_experimental,
            is_final,
            base_class,
            api_level: get_api_level(json),
            constants,
            enums,
            methods,
            signals,
        })
    }
}

impl BuiltinClass {
    pub fn from_json(json: &JsonBuiltinClass, ctx: &mut Context) -> Option<Self> {
        let ty_name = TyName::from_godot(&json.name);

        if special_cases::is_builtin_type_deleted(&ty_name) {
            return None;
        }

        let inner_name = TyName::from_godot(&format!("Inner{}", ty_name.godot_ty));
        let mod_name = ModName::from_godot(&ty_name.godot_ty);

        let operators = json.operators.iter().map(Operator::from_json).collect();

        let methods = option_as_slice(&json.methods)
            .iter()
            .filter_map(|m| {
                let inner_class_name = &ty_name;
                BuiltinMethod::from_json(m, &ty_name, inner_class_name, ctx)
            })
            .collect();

        let constructors = json
            .constructors
            .iter()
            .map(Constructor::from_json)
            .collect();

        let has_destructor = json.has_destructor;

        let enums = option_as_slice(&json.enums)
            .iter()
            .map(|e| {
                let surrounding_class = Some(&ty_name);
                Enum::from_json(&e.to_enum(), surrounding_class)
            })
            .collect();

        Some(Self {
            common: ClassCommons {
                name: ty_name,
                mod_name,
            },
            inner_name,
            operators,
            methods,
            constructors,
            has_destructor,
            enums,
        })
    }
}

impl Singleton {
    pub fn from_json(json: &JsonSingleton) -> Self {
        Self {
            name: TyName::from_godot(&json.name),
        }
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Builtin variants

impl BuiltinVariant {
    /// Returns all builtins, ordered by enum ordinal value.
    pub fn all_from_json(
        global_enums: &[JsonEnum],
        builtin_classes: &[JsonBuiltinClass],
        ctx: &mut Context,
    ) -> Vec<Self> {
        fn normalize(name: &str) -> String {
            name.to_ascii_lowercase().replace('_', "")
        }

        let variant_type_enum = global_enums
            .iter()
            .find(|e| &e.name == "Variant.Type")
            .expect("missing enum for VariantType in JSON");

        // Make HashMap from builtin_classes, keyed by a normalized version of their names (all-lower, no underscores)
        let builtin_classes: HashMap<String, &JsonBuiltinClass> = builtin_classes
            .iter()
            .map(|c| (normalize(&c.name), c))
            .collect();

        let mut all = variant_type_enum
            .values
            .iter()
            .filter_map(|e| {
                let json_shout_case = e
                    .name
                    .strip_prefix("TYPE_")
                    .expect("variant enumerator lacks prefix 'TYPE_'");

                if json_shout_case == "NIL" || json_shout_case == "MAX" {
                    return None;
                }

                let name = normalize(json_shout_case);
                let json_builtin_class = builtin_classes.get(&name).copied();
                let json_ord = e.to_enum_ord();

                Some(Self::from_json(
                    json_shout_case,
                    json_ord,
                    json_builtin_class,
                    ctx,
                ))
            })
            .collect::<Vec<_>>();

        all.sort_by_key(|v| v.variant_type_ord);
        all
    }

    pub fn from_json(
        json_variant_enumerator_name: &str,
        json_variant_enumerator_ord: i32,
        json_builtin_class: Option<&JsonBuiltinClass>,
        ctx: &mut Context,
    ) -> Self {
        let builtin_class;
        let godot_original_name;

        // Nil, int, float etc. are not represented by a BuiltinVariant.
        // Object has no BuiltinClass, but still gets its BuiltinVariant instance.
        if let Some(json_builtin) = json_builtin_class {
            builtin_class = BuiltinClass::from_json(json_builtin, ctx);
            godot_original_name = json_builtin.name.clone();
        } else {
            assert_eq!(json_variant_enumerator_name, "OBJECT");

            builtin_class = None;
            godot_original_name = "Object".to_string();
        };

        Self {
            godot_original_name,
            godot_shout_name: json_variant_enumerator_name.to_string(), // Without `TYPE_` prefix.
            godot_snake_name: conv::to_snake_case(json_variant_enumerator_name),
            builtin_class,
            variant_type_ord: json_variant_enumerator_ord,
        }
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Constructors, operators

impl Constructor {
    pub fn from_json(json: &JsonConstructor) -> Self {
        Self {
            index: json.index, // TODO use enum for Default/Copy/Other(index)
            raw_parameters: json.arguments.as_ref().map_or(vec![], |vec| vec.clone()),
        }
    }
}

impl Operator {
    pub fn from_json(json: &JsonOperator) -> Self {
        Self {
            symbol: json.name.clone(),
        }
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Build config + version

impl BuildConfiguration {
    pub fn from_json(json: &str) -> Self {
        match json {
            "float_32" => Self::Float32,
            "float_64" => Self::Float64,
            "double_32" => Self::Double32,
            "double_64" => Self::Double64,
            _ => panic!("invalid build configuration: {json}"),
        }
    }
}

impl GodotApiVersion {
    pub fn from_json(json: &JsonHeader) -> Self {
        let version_string = json
            .version_full_name
            .strip_prefix("Godot Engine ")
            .unwrap_or(&json.version_full_name)
            .to_string();

        Self {
            major: json.version_major,
            minor: json.version_minor,
            patch: json.version_patch,
            version_string,
        }
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Functions

impl BuiltinMethod {
    pub fn from_json(
        method: &JsonBuiltinMethod,
        builtin_name: &TyName,
        inner_class_name: &TyName,
        ctx: &mut Context,
    ) -> Option<Self> {
        if special_cases::is_builtin_method_deleted(builtin_name, method) {
            return None;
        }

        let return_value = method
            .return_type
            .as_deref()
            .map(JsonMethodReturn::from_type_no_meta);

        Some(Self {
            common: FunctionCommon {
                // Fill in these fields
                name: method.name.clone(),
                godot_name: method.name.clone(),
                // Disable default parameters for builtin classes.
                // They are not public-facing and need more involved implementation (lifetimes etc.). Also reduces number of symbols in API.
                parameters: FnParam::new_range_no_defaults(&method.arguments, ctx),
                return_value: FnReturn::new(&return_value, ctx),
                is_vararg: method.is_vararg,
                is_private: false, // See 'exposed' below. Could be special_cases::is_method_private(builtin_name, &method.name),
                is_virtual_required: false,
                is_unsafe: false, // Builtin methods don't use raw pointers.
                direction: FnDirection::Outbound {
                    hash: method.hash.expect("hash absent for builtin method"),
                },
            },
            qualifier: FnQualifier::from_const_static(method.is_const, method.is_static),
            surrounding_class: inner_class_name.clone(),
            is_exposed_in_outer: special_cases::is_builtin_method_exposed(
                builtin_name,
                &method.name,
            ),
        })
    }
}

impl ClassMethod {
    pub fn from_json(
        method: &JsonClassMethod,
        class_name: &TyName,
        ctx: &mut Context,
    ) -> Option<ClassMethod> {
        assert!(!special_cases::is_class_deleted(class_name));
		
		// ✅ Patch: Allow full overload of TileMap::set_cell
        if class_name.godot_ty == "TileMap" && method.name == "set_cell" {
            let arg_len = method.arguments.as_ref().map_or(0, |args| args.len());
            if arg_len < 5 {
                // Skip shorter overloads
                return None;
            }
        }

        if special_cases::is_class_method_deleted(class_name, method, ctx) {
            return None;
        }

        if method.is_virtual {
            Self::from_json_virtual(method, class_name, ctx)
        } else {
            Self::from_json_outbound(method, class_name, ctx)
        }
    }

    fn from_json_outbound(
        method: &JsonClassMethod,
        class_name: &TyName,
        ctx: &mut Context,
    ) -> Option<Self> {
        assert!(!method.is_virtual);
        let hash = method
            .hash
            .expect("hash absent for non-virtual class method");

        let rust_method_name = special_cases::maybe_rename_class_method(class_name, &method.name);

        Self::from_json_inner(
            method,
            rust_method_name,
            class_name,
            FnDirection::Outbound { hash },
            ctx,
        )
    }

    fn from_json_virtual(
        method: &JsonClassMethod,
        class_name: &TyName,
        ctx: &mut Context,
    ) -> Option<Self> {
        assert!(method.is_virtual);

        // Hash for virtual methods is available from Godot 4.4, see https://github.com/godotengine/godot/pull/100674.
        let direction = FnDirection::Virtual {
            #[cfg(since_api = "4.4")]
            hash: {
                let hash_i64 = method.hash.unwrap_or_else(|| {
                    panic!(
                        "virtual class methods must have a hash since Godot 4.4; missing: {}.{}",
                        class_name.godot_ty, method.name
                    )
                });

                // TODO see if we can use u32 everywhere.
                hash_i64.try_into().unwrap_or_else(|_| {
                    panic!(
                        "virtual method {}.{} has hash {} that is out of range for u32",
                        class_name.godot_ty, method.name, hash_i64
                    )
                })
            },
        };

        // May still be renamed further, for unsafe methods. Not done here because data to determine safety is not available yet.
        let rust_method_name = Self::make_virtual_method_name(class_name, &method.name);

        Self::from_json_inner(method, rust_method_name, class_name, direction, ctx)
    }

    fn from_json_inner(
        method: &JsonClassMethod,
        rust_method_name: &str,
        class_name: &TyName,
        direction: FnDirection,
        ctx: &mut Context,
    ) -> Option<ClassMethod> {
        if special_cases::is_class_method_deleted(class_name, method, ctx) {
            return None;
        }

        let is_private = special_cases::is_method_private(class_name, &method.name);
        let godot_method_name = method.name.clone();

        let qualifier = {
            // Override const-qualification for known special cases (FileAccess::get_16, StreamPeer::get_u16, etc.).
            let mut is_actually_const = method.is_const;
            if let Some(override_const) = special_cases::is_class_method_const(class_name, method) {
                is_actually_const = override_const;
            }

            FnQualifier::from_const_static(is_actually_const, method.is_static)
        };

        // Since Godot 4.4, GDExtension advertises whether virtual methods have a default implementation or are required to be overridden.
        #[cfg(before_api = "4.4")]
        let is_virtual_required =
            special_cases::is_virtual_method_required(&class_name, &method.name);

        #[cfg(since_api = "4.4")]
        #[allow(clippy::let_and_return)]
        let is_virtual_required = method.is_virtual && {
            // Evaluate this always first (before potential manual overrides), to detect mistakes in spec.
            let is_required_in_json = method.is_required.unwrap_or_else(|| {
                panic!(
                    "virtual method {}::{} lacks field `is_required`",
                    class_name.rust_ty, rust_method_name
                );
            });

            // Potential special cases come here. The situation "virtual function is required in base class, but not in derived"
            // is not handled here, but in virtual_traits.rs. Here, virtual methods appear only once, in their base.

            is_required_in_json
        };

        let parameters = FnParam::new_range(&method.arguments, ctx);
        let return_value = FnReturn::new(&method.return_value, ctx);
        let is_unsafe = Self::function_uses_pointers(&parameters, &return_value);

        // Future note: if further changes are made to the virtual method name, make sure to make it reversible so that #[godot_api]
        // can match on the Godot name of the virtual method.
        let rust_method_name = if is_unsafe && method.is_virtual {
            // If the method is unsafe, we need to rename it to avoid conflicts with the safe version.
            conv::make_unsafe_virtual_fn_name(rust_method_name)
        } else {
            rust_method_name.to_string()
        };

        Some(Self {
            common: FunctionCommon {
                name: rust_method_name,
                godot_name: godot_method_name,
                parameters,
                return_value,
                is_vararg: method.is_vararg,
                is_private,
                is_virtual_required,
                is_unsafe,
                direction,
            },
            qualifier,
            surrounding_class: class_name.clone(),
        })
    }

    fn make_virtual_method_name<'m>(class_name: &TyName, godot_method_name: &'m str) -> &'m str {
        // Hardcoded overrides.
        if let Some(rust_name) =
            special_cases::maybe_rename_virtual_method(class_name, godot_method_name)
        {
            return rust_name;
        }

        // In general, just rlemove leading underscore from virtual method names.
        godot_method_name
            .strip_prefix('_')
            .unwrap_or(godot_method_name)
    }

    fn function_uses_pointers(parameters: &[FnParam], return_value: &FnReturn) -> bool {
        let has_pointer_params = parameters
            .iter()
            .any(|param| matches!(param.type_, RustTy::RawPointer { .. }));

        let has_pointer_return = matches!(return_value.type_, Some(RustTy::RawPointer { .. }));

        // No short-circuiting due to variable decls, but that's fine.
        has_pointer_params || has_pointer_return
    }
}

impl ClassSignal {
    pub fn from_json(
        json_signal: &JsonSignal,
        surrounding_class: &TyName,
        ctx: &mut Context,
    ) -> Option<Self> {
        if special_cases::is_signal_deleted(surrounding_class, json_signal) {
            return None;
        }

        Some(Self {
            name: json_signal.name.clone(),
            parameters: FnParam::new_range(&json_signal.arguments, ctx),
            surrounding_class: surrounding_class.clone(),
        })
    }
}

impl UtilityFunction {
    pub fn from_json(function: &JsonUtilityFunction, ctx: &mut Context) -> Option<Self> {
        if special_cases::is_utility_function_deleted(function, ctx) {
            return None;
        }
        let is_private = special_cases::is_utility_function_private(function);

        // Some vararg functions like print() or str() are declared with a single argument "arg1: Variant", but that seems
        // to be a mistake. We change their parameter list by removing that.
        let args = option_as_slice(&function.arguments);
        let parameters = if function.is_vararg && args.len() == 1 && args[0].name == "arg1" {
            vec![]
        } else {
            FnParam::new_range(&function.arguments, ctx)
        };

        let godot_method_name = function.name.clone();
        let rust_method_name = godot_method_name.clone(); // No change for now.

        let return_value = function
            .return_type
            .as_deref()
            .map(JsonMethodReturn::from_type_no_meta);

        Some(Self {
            common: FunctionCommon {
                name: rust_method_name,
                godot_name: godot_method_name,
                parameters,
                return_value: FnReturn::new(&return_value, ctx),
                is_vararg: function.is_vararg,
                is_private,
                is_virtual_required: false,
                is_unsafe: false, // Utility functions don't use raw pointers.
                direction: FnDirection::Outbound {
                    hash: function.hash,
                },
            },
        })
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Enums + enumerator constants

impl Enum {
    pub fn from_json(json_enum: &JsonEnum, surrounding_class: Option<&TyName>) -> Self {
        let godot_name = &json_enum.name;
        let is_bitfield = special_cases::is_enum_bitfield(surrounding_class, godot_name)
            .unwrap_or(json_enum.is_bitfield);
        let is_private = special_cases::is_enum_private(surrounding_class, godot_name);
        let is_exhaustive = special_cases::is_enum_exhaustive(surrounding_class, godot_name);

        let rust_enum_name = conv::make_enum_name_str(godot_name);
        let rust_enumerator_names = {
            let godot_enumerator_names = json_enum
                .values
                .iter()
                .map(|e| {
                    // Special cases. Extract to special_cases mode if more are added.
                    if e.name == "OP_MODULE" {
                        "OP_MODULO"
                    } else {
                        e.name.as_str()
                    }
                })
                .collect();
            let godot_class_name = surrounding_class.as_ref().map(|ty| ty.godot_ty.as_str());

            conv::make_enumerator_names(godot_class_name, &rust_enum_name, godot_enumerator_names)
        };

        let enumerators: Vec<Enumerator> = json_enum
            .values
            .iter()
            .zip(rust_enumerator_names)
            .map(|(json_constant, rust_name)| {
                Enumerator::from_json(json_constant, rust_name, is_bitfield)
            })
            .collect();

        let max_index = Enum::find_index_enum_max_impl(is_bitfield, &enumerators);

        Self {
            name: ident(&rust_enum_name),
            godot_name: godot_name.clone(),
            surrounding_class: surrounding_class.cloned(),
            is_bitfield,
            is_private,
            is_exhaustive,
            enumerators,
            max_index,
        }
    }
}

impl Enumerator {
    pub fn from_json(json: &JsonEnumConstant, rust_name: Ident, is_bitfield: bool) -> Self {
        let value = if is_bitfield {
            let ord = json.value.try_into().unwrap_or_else(|_| {
                panic!(
                    "bitfield value {} = {} is negative; please report this",
                    json.name, json.value
                )
            });

            EnumeratorValue::Bitfield(ord)
        } else {
            let ord = json.value.try_into().unwrap_or_else(|_| {
                panic!(
                    "enum value {} = {} is out of range for i32; please report this",
                    json.name, json.value
                )
            });

            EnumeratorValue::Enum(ord)
        };

        Self {
            name: rust_name,
            godot_name: json.name.clone(),
            value,
        }
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Constants

impl ClassConstant {
    pub fn from_json(json: &JsonClassConstant) -> Self {
        // Godot types only use i32, but other extensions may have i64. Use smallest possible type.
        let value = if let Ok(i32_value) = i32::try_from(json.value) {
            ClassConstantValue::I32(i32_value)
        } else {
            ClassConstantValue::I64(json.value)
        };

        Self {
            name: json.name.clone(),
            value,
        }
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Native structures

impl NativeStructure {
    pub fn from_json(json: &JsonNativeStructure) -> Self {
        Self {
            name: json.name.clone(),
            format: json.format.clone(),
        }
    }
}
