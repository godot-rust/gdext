/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::collections::HashMap;

use proc_macro2::Ident;

use crate::context::Context;
use crate::models::api_json::{
    JsonBuiltinClass, JsonBuiltinMethod, JsonBuiltinSizes, JsonClass, JsonClassConstant,
    JsonClassMethod, JsonConstructor, JsonEnum, JsonEnumConstant, JsonExtensionApi, JsonHeader,
    JsonMethodArg, JsonMethodReturn, JsonNativeStructure, JsonOperator, JsonSignal, JsonSingleton,
    JsonUtilityFunction,
};
use crate::models::domain::{
    BuildConfiguration, BuiltinClass, BuiltinMethod, BuiltinSize, BuiltinVariant, Class,
    ClassCommons, ClassConstant, ClassConstantValue, ClassMethod, ClassSignal, Constructor, Enum,
    EnumReplacements, Enumerator, EnumeratorValue, ExtensionApi, FlowDirection, FnDirection,
    FnParam, FnQualifier, FnReturn, FunctionCommon, GodotApiVersion, ModName, NativeStructure,
    Operator, RustTy, Singleton, TyName, UtilityFunction,
};
use crate::util::{get_api_level, ident};
use crate::{conv, special_cases};

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Top-level

impl ExtensionApi {
    pub fn from_json(json: JsonExtensionApi, ctx: &mut Context) -> Self {
        let JsonExtensionApi {
            header,
            builtin_class_sizes,
            builtin_classes,
            classes,
            global_enums,
            utility_functions,
            native_structures,
            singletons,
        } = json;

        Self {
            builtins: BuiltinVariant::all_from_json(&global_enums, builtin_classes, ctx),
            classes: classes
                .into_iter()
                .filter_map(|json| Class::from_json(json, ctx))
                .collect(),
            singletons: singletons.into_iter().map(Singleton::from_json).collect(),
            native_structures: native_structures
                .into_iter()
                .map(NativeStructure::from_json)
                .collect(),
            utility_functions: utility_functions
                .into_iter()
                .filter_map(|json| UtilityFunction::from_json(json, ctx))
                .collect(),
            global_enums: global_enums
                .into_iter()
                .map(|json| Enum::from_json(json, None))
                .collect(),
            godot_version: GodotApiVersion::from_json(header),
            builtin_sizes: Self::builtin_size_from_json(builtin_class_sizes),
        }
    }

    fn builtin_size_from_json(json_builtin_sizes: Vec<JsonBuiltinSizes>) -> Vec<BuiltinSize> {
        let mut result = Vec::new();

        for json_builtin_size in json_builtin_sizes {
            let build_config_str = json_builtin_size.build_configuration.as_str();
            let config = BuildConfiguration::from_json(build_config_str);

            if config.is_applicable() {
                for size_for_config in json_builtin_size.sizes {
                    result.push(BuiltinSize {
                        builtin_original_name: size_for_config.name,
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
    pub fn from_json(json: JsonClass, ctx: &mut Context) -> Option<Self> {
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
        let api_level = get_api_level(&json);

        let JsonClass {
            is_refcounted,
            inherits,
            constants,
            enums,
            methods,
            signals,
            description,
            ..
        } = json;

        let constants = constants
            .unwrap_or_default()
            .into_iter()
            .map(ClassConstant::from_json)
            .collect();

        let enums = enums
            .unwrap_or_default()
            .into_iter()
            .map(|e| Enum::from_json(e, Some(&ty_name)))
            .collect();

        let methods = methods
            .unwrap_or_default()
            .into_iter()
            .filter_map(|m| ClassMethod::from_json(m, &ty_name, ctx))
            .collect();

        let signals = signals
            .unwrap_or_default()
            .into_iter()
            .filter_map(|s| ClassSignal::from_json(s, &ty_name, ctx))
            .collect();

        let base_class = inherits.map(|godot_name| TyName::from_godot(&godot_name));

        Some(Self {
            common: ClassCommons {
                name: ty_name,
                mod_name,
            },
            is_refcounted,
            is_instantiable,
            is_experimental,
            is_final,
            base_class,
            api_level,
            constants,
            enums,
            methods,
            signals,
            description,
        })
    }
}

impl BuiltinClass {
    pub fn from_json(json: JsonBuiltinClass, ctx: &mut Context) -> Option<Self> {
        let ty_name = TyName::from_godot_builtin(&json);

        if special_cases::is_builtin_type_deleted(&ty_name) {
            return None;
        }

        let mod_name = ModName::from_godot_builtin(&json);
        let inner_name = TyName::from_godot(&format!("Inner{}", ty_name.godot_ty));
        let JsonBuiltinClass {
            operators,
            methods,
            constructors,
            has_destructor,
            enums,
            ..
        } = json;

        let operators = operators.into_iter().map(Operator::from_json).collect();

        let methods = methods
            .unwrap_or_default()
            .into_iter()
            .filter_map(|m| {
                // Pass inner_name "Inner*" as surrounding class. This is later overridden to the outer type (e.g. "GString")
                // for methods exposed in the public API via is_builtin_method_exposed().
                BuiltinMethod::from_json(m, &ty_name, &inner_name, ctx)
            })
            .collect();

        let constructors = constructors
            .into_iter()
            .map(Constructor::from_json)
            .collect();

        let enums = enums
            .unwrap_or_default()
            .into_iter()
            .map(|e| Enum::from_json(e.into_enum(), Some(&ty_name)))
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
    pub fn from_json(json: JsonSingleton) -> Self {
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
        builtin_classes: Vec<JsonBuiltinClass>,
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
        let mut builtin_classes: HashMap<String, JsonBuiltinClass> = builtin_classes
            .into_iter()
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
                let json_builtin_class = builtin_classes.remove(&name);
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
        json_builtin_class: Option<JsonBuiltinClass>,
        ctx: &mut Context,
    ) -> Self {
        let builtin_class;
        let godot_original_name;

        // Nil, int, float etc. are not represented by a BuiltinVariant.
        // Object has no BuiltinClass, but still gets its BuiltinVariant instance.
        if let Some(json_builtin) = json_builtin_class {
            godot_original_name = json_builtin.name.clone();
            builtin_class = BuiltinClass::from_json(json_builtin, ctx);
        } else {
            assert_eq!(
                json_variant_enumerator_name, "OBJECT",
                "variant type {json_variant_enumerator_name:?} has no builtin class entry in the JSON; \
                 only OBJECT is expected to lack one -- Godot's Variant.Type enum may have gained a new class-less type"
            );

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
    pub fn from_json(json: JsonConstructor) -> Self {
        Self {
            index: json.index, // TODO use enum for Default/Copy/Other(index)
            raw_parameters: json.arguments.unwrap_or_default(),
        }
    }
}

impl Operator {
    pub fn from_json(json: JsonOperator) -> Self {
        Self { symbol: json.name }
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
    pub fn from_json(json: JsonHeader) -> Self {
        let JsonHeader {
            version_major,
            version_minor,
            version_patch,
            version_full_name,
            ..
        } = json;

        let version_string = version_full_name
            .strip_prefix("Godot Engine ")
            .unwrap_or(&version_full_name)
            .to_string();

        Self {
            major: version_major,
            minor: version_minor,
            patch: version_patch,
            version_string,
        }
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Functions

impl BuiltinMethod {
    pub fn from_json(
        method: JsonBuiltinMethod,
        builtin_name: &TyName,
        inner_class_name: &TyName,
        ctx: &mut Context,
    ) -> Option<Self> {
        if special_cases::is_builtin_method_deleted(builtin_name, &method) {
            return None;
        }

        let is_exposed_in_outer =
            special_cases::is_builtin_method_exposed(builtin_name, &method.name);

        let JsonBuiltinMethod {
            name,
            return_type,
            is_vararg,
            is_const,
            is_static,
            hash,
            arguments,
            description,
        } = method;

        let return_value = {
            let return_value = &return_type
                .as_deref()
                .map(JsonMethodReturn::from_type_no_meta);

            // Builtin methods are always outbound (not virtual), thus flow for return type is Godot -> Rust.
            // Exception: Inner{Array,Dictionary} methods return Any{Array,Dictionary} instead of Var{Array,Dictionary}. Reason is that
            // arrays/dicts can be generic and store type info, thus typing returned collections differently. Thus use RustToGodot flow.
            let flow = if !is_exposed_in_outer
                && matches!(builtin_name.godot_ty.as_str(), "Array" | "Dictionary")
                && matches!(return_type.as_deref(), Some("Array" | "Dictionary"))
            {
                FlowDirection::RustToGodot // AnyArray + AnyDictionary.
            } else {
                FlowDirection::GodotToRust
            };

            FnReturn::new(return_value, flow, ctx)
        };

        // For parameters in builtin methods, flow is always Rust -> Godot.
        // Enable default parameters for builtin classes, generating _ex builders.
        let parameters = FnParam::builder().build_many(arguments, FlowDirection::RustToGodot, ctx);

        // Construct surrounding_class with correct type names:
        // * godot_ty: Always the real Godot type (e.g. "String").
        // * rust_ty: Rust struct where the method is declared ("GString" for exposed, "InnerString" for private one).
        let surrounding_class = {
            let rust_ty = if is_exposed_in_outer {
                match conv::to_rust_type(&builtin_name.godot_ty, None, None, ctx) {
                    RustTy::BuiltinIdent { ty, .. } => ty,
                    _ => panic!("Builtin type should map to BuiltinIdent"),
                }
            } else {
                inner_class_name.rust_ty.clone()
            };

            TyName {
                godot_ty: builtin_name.godot_ty.clone(),
                rust_ty,
            }
        };

        Some(Self {
            common: FunctionCommon {
                // Fill in these fields
                name: name.clone(),
                godot_name: name,
                parameters,
                return_value,
                is_vararg,
                is_private: false, // See 'exposed' below. Could be special_cases::is_method_private(builtin_name, &method.name),
                is_virtual_required: false,
                is_unsafe: false, // Builtin methods don't use raw pointers.
                direction: FnDirection::Outbound {
                    hash: hash.expect("hash absent for builtin method"),
                },
                deprecation_msg: None, // Builtin methods are not deprecated yet.
                description,
            },
            qualifier: FnQualifier::from_const_static(is_const, is_static),
            surrounding_class,
            is_exposed_in_outer,
        })
    }
}

impl ClassMethod {
    pub fn from_json(
        method: JsonClassMethod,
        class_name: &TyName,
        ctx: &mut Context,
    ) -> Option<ClassMethod> {
        assert!(!special_cases::is_class_deleted(class_name));

        if special_cases::is_class_method_deleted(class_name, &method, ctx) {
            return None;
        }

        if method.is_virtual {
            Self::from_json_virtual(method, class_name, ctx)
        } else {
            Self::from_json_outbound(method, class_name, ctx)
        }
    }

    fn from_json_outbound(
        method: JsonClassMethod,
        class_name: &TyName,
        ctx: &mut Context,
    ) -> Option<Self> {
        assert!(!method.is_virtual);
        let hash = method
            .hash
            .expect("hash absent for non-virtual class method");

        let rust_method_name =
            special_cases::maybe_rename_class_method(class_name, &method.name).into_owned();

        Self::from_json_inner(
            method,
            rust_method_name,
            class_name,
            FnDirection::Outbound { hash },
            ctx,
        )
    }

    fn from_json_virtual(
        method: JsonClassMethod,
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
        let rust_method_name = Self::make_virtual_method_name(class_name, &method.name).to_string();

        Self::from_json_inner(method, rust_method_name, class_name, direction, ctx)
    }

    fn from_json_inner(
        method: JsonClassMethod,
        rust_method_name: String,
        class_name: &TyName,
        direction: FnDirection,
        ctx: &mut Context,
    ) -> Option<ClassMethod> {
        let is_private = special_cases::is_method_private(class_name, &method.name);

        let qualifier = {
            // Override const-qualification for known special cases (FileAccess::get_16, StreamPeer::get_u16, etc.).
            let mut is_actually_const = method.is_const;
            if let Some(override_const) = special_cases::is_class_method_const(class_name, &method)
            {
                is_actually_const = override_const;
            }

            FnQualifier::from_const_static(is_actually_const, method.is_static)
        };

        // Since Godot 4.4, GDExtension advertises whether virtual methods have a default implementation or are required to be overridden.
        #[cfg(before_api = "4.4")]
        let is_virtual_required = method.is_virtual
            && special_cases::is_virtual_method_required(&class_name, &method.name);

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

        // Ensure that parameters/return types listed in the replacement truly exist in the method.
        // The validation function now returns the validated replacement slice for reuse.
        let enum_replacements = validate_enum_replacements(
            class_name,
            &method.name,
            method.arguments.as_deref().unwrap_or(&[]),
            method.return_value.is_some(),
        );

        let (param_flow, return_flow) = match &direction {
            FnDirection::Outbound { .. } => {
                (FlowDirection::RustToGodot, FlowDirection::GodotToRust)
            }
            FnDirection::Virtual { .. } => (FlowDirection::GodotToRust, FlowDirection::RustToGodot),
        };

        let deprecation_msg = special_cases::get_class_method_deprecation(class_name, &method);

        let JsonClassMethod {
            name,
            is_vararg,
            is_virtual,
            return_value,
            arguments,
            description,
            ..
        } = method;

        let parameters = FnParam::builder()
            .enum_replacements(enum_replacements)
            .build_many(arguments, param_flow, ctx);

        let return_value =
            FnReturn::with_enum_replacements(&return_value, enum_replacements, return_flow, ctx);

        let is_unsafe = Self::function_uses_pointers(&parameters, &return_value);

        // Future note: if further changes are made to the virtual method name, make sure to make it reversible so that #[godot_api]
        // can match on the Godot name of the virtual method.
        let rust_method_name = if is_unsafe && is_virtual {
            // If the method is unsafe, we need to rename it to avoid conflicts with the safe version.
            conv::make_unsafe_virtual_fn_name(&rust_method_name)
        } else {
            rust_method_name
        };

        Some(Self {
            common: FunctionCommon {
                name: rust_method_name,
                godot_name: name,
                parameters,
                return_value,
                is_vararg,
                is_private,
                is_virtual_required,
                is_unsafe,
                direction,
                deprecation_msg,
                description,
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

        // In general, just remove leading underscore from virtual method names.
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
        json_signal: JsonSignal,
        surrounding_class: &TyName,
        ctx: &mut Context,
    ) -> Option<Self> {
        if special_cases::is_signal_deleted(surrounding_class, &json_signal) {
            return None;
        }

        // Signals only have parameters, no return type; emitted data always flows Rust -> Godot.
        let flow = FlowDirection::RustToGodot;

        Some(Self {
            name: json_signal.name,
            parameters: FnParam::builder().build_many(json_signal.arguments, flow, ctx),
            surrounding_class: surrounding_class.clone(),
        })
    }
}

impl UtilityFunction {
    pub fn from_json(function: JsonUtilityFunction, ctx: &mut Context) -> Option<Self> {
        if special_cases::is_utility_function_deleted(&function, ctx) {
            return None;
        }
        let is_private = special_cases::is_utility_function_private(&function);
        let is_thread_safe = special_cases::is_utility_function_thread_safe(&function);

        let JsonUtilityFunction {
            name,
            return_type,
            is_vararg,
            hash,
            arguments,
            description,
            ..
        } = function;

        // Some vararg functions like print() or str() are declared with a single argument "arg1: Variant", but that seems
        // to be a mistake. We change their parameter list by removing that.
        let parameters =
            if is_vararg && matches!(arguments.as_deref(), Some([arg]) if arg.name == "arg1") {
                vec![]
            } else {
                // Parameters in utility functions always flow Rust -> Godot.
                FnParam::builder().build_many(arguments, FlowDirection::RustToGodot, ctx)
            };

        let json_return = return_type
            .as_deref()
            .map(JsonMethodReturn::from_type_no_meta);
        let return_value = FnReturn::new(&json_return, FlowDirection::GodotToRust, ctx);

        let godot_method_name = name;
        let rust_method_name = godot_method_name.clone(); // No change for now.

        Some(Self {
            common: FunctionCommon {
                name: rust_method_name,
                godot_name: godot_method_name,
                parameters,
                return_value,
                is_vararg,
                is_private,
                is_virtual_required: false,
                is_unsafe: false, // Utility functions don't use raw pointers.
                direction: FnDirection::Outbound { hash },
                deprecation_msg: None, // Utility functions are not deprecated.
                description,
            },
            is_thread_safe,
        })
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Enums + enumerator constants

impl Enum {
    pub fn from_json(json_enum: JsonEnum, surrounding_class: Option<&TyName>) -> Self {
        let JsonEnum {
            name: godot_name,
            is_bitfield: json_is_bitfield,
            values,
        } = json_enum;

        let is_bitfield = special_cases::is_enum_bitfield(surrounding_class, &godot_name)
            .unwrap_or(json_is_bitfield);
        let is_private = special_cases::is_enum_private(surrounding_class, &godot_name);
        let is_exhaustive = special_cases::is_enum_exhaustive(surrounding_class, &godot_name);

        let rust_enum_name = conv::make_enum_name_str(&godot_name);
        let rust_enumerator_names = {
            let godot_enumerator_names = values
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

        let enumerators: Vec<Enumerator> = values
            .into_iter()
            .zip(rust_enumerator_names)
            .map(|(json_constant, rust_name)| {
                Enumerator::from_json(json_constant, rust_name, is_bitfield)
            })
            .collect();

        let max_index = Enum::find_index_enum_max_impl(is_bitfield, &enumerators);

        Self {
            name: ident(&rust_enum_name),
            godot_name,
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
    pub fn from_json(json: JsonEnumConstant, rust_name: Ident, is_bitfield: bool) -> Self {
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
            godot_name: json.name,
            value,
        }
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Constants

impl ClassConstant {
    pub fn from_json(json: JsonClassConstant) -> Self {
        // Godot types only use i32, but other extensions may have i64. Use smallest possible type.
        let value = if let Ok(i32_value) = i32::try_from(json.value) {
            ClassConstantValue::I32(i32_value)
        } else {
            ClassConstantValue::I64(json.value)
        };

        Self {
            name: json.name,
            value,
        }
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

/// Validates that all parameters and non-unit return types declared in an enum replacement slices actually exist in the method.
///
/// This is a measure to prevent accidental typos or listing inexistent parameters, which would have no effect.
fn validate_enum_replacements(
    class_ty: &TyName,
    godot_method_name: &str,
    method_arguments: &[JsonMethodArg],
    has_return_type: bool,
) -> EnumReplacements {
    let replacements =
        special_cases::get_class_method_param_enum_replacement(class_ty, godot_method_name);

    for (param_name, enum_name, _) in replacements {
        if param_name.is_empty() {
            assert!(
                has_return_type,
                "Method `{class}.{godot_method_name}` has no return type, but replacement with `{enum_name}` is declared",
                class = class_ty.godot_ty
            );
        } else if !method_arguments.iter().any(|arg| arg.name == *param_name) {
            let available_params = method_arguments
                .iter()
                .map(|arg| format!("  * {}: {}", arg.name, arg.type_))
                .collect::<Vec<_>>()
                .join("\n");

            panic!(
                "Method `{class}.{godot_method_name}` has no parameter `{param_name}`, but a replacement with `{enum_name}` is declared\n\
                \n{count} parameters available:\n{available_params}\n",
                class = class_ty.godot_ty,
                count = method_arguments.len(),
            );
        }
    }

    replacements
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Native structures

impl NativeStructure {
    pub fn from_json(json: JsonNativeStructure) -> Self {
        let JsonNativeStructure { name, format } = json;

        // Some native-struct definitions are incorrect in earlier Godot versions; this backports corrections.
        let format = special_cases::get_native_struct_definition(&name)
            .map(|s| s.to_string())
            .unwrap_or(format);

        Self { name, format }
    }
}
