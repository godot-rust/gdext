/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::context::Context;
use crate::domain_models::{
    BuiltinMethod, ClassConstant, ClassConstantValue, ClassMethod, Constructor, Enum, Enumerator,
    EnumeratorValue, FnDirection, FnParam, FnQualifier, FnReturn, FunctionCommon, Operator,
    UtilityFunction,
};
use crate::json_models::{
    JsonBuiltinMethod, JsonClassConstant, JsonClassMethod, JsonConstructor, JsonEnum,
    JsonEnumConstant, JsonMethodReturn, JsonOperator, JsonUtilityFunction,
};
use crate::util::ident;
use crate::{conv, special_cases, TyName};
use proc_macro2::Ident;

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
            name: json.name.clone(),
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

        // use BuiltinMethod, but adopt values from above FnSignature expr
        Some(Self {
            common: FunctionCommon {
                // Fill in these fields
                name: method.name.clone(),
                godot_name: method.name.clone(),
                // Disable default parameters for builtin classes.
                // They are not public-facing and need more involved implementation (lifetimes etc). Also reduces number of symbols in API.
                parameters: FnParam::new_range_no_defaults(&method.arguments, ctx),
                return_value: FnReturn::new(&return_value, ctx),
                is_vararg: method.is_vararg,
                is_private: special_cases::is_method_private(builtin_name, &method.name),
                direction: FnDirection::Outbound {
                    hash: method.hash.expect("hash absent for builtin method"),
                },
            },
            qualifier: FnQualifier::from_const_static(method.is_const, method.is_static),
            surrounding_class: inner_class_name.clone(),
        })
    }
}

impl ClassMethod {
    pub fn from_json_outbound(
        method: &JsonClassMethod,
        class_name: &TyName,
        ctx: &mut Context,
    ) -> Option<Self> {
        if method.is_virtual {
            return None;
        }

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
    pub fn from_json_virtual(
        method: &JsonClassMethod,
        class_name: &TyName,
        ctx: &mut Context,
    ) -> Option<Self> {
        if !method.is_virtual {
            return None;
        }

        assert!(
            method.hash.is_none(),
            "hash present for virtual class method"
        );

        let rust_method_name = Self::make_virtual_method_name(&method.name);

        Self::from_json_inner(
            method,
            rust_method_name,
            class_name,
            FnDirection::Virtual,
            ctx,
        )
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

        Some(Self {
            common: FunctionCommon {
                name: rust_method_name.to_string(),
                godot_name: godot_method_name,
                parameters: FnParam::new_range(&method.arguments, ctx),
                return_value: FnReturn::new(&method.return_value, ctx),
                is_vararg: method.is_vararg,
                is_private,
                direction,
            },
            qualifier,
            surrounding_class: class_name.clone(),
        })
    }

    fn make_virtual_method_name(godot_method_name: &str) -> &str {
        // Remove leading underscore from virtual method names.
        let method_name = godot_method_name
            .strip_prefix('_')
            .unwrap_or(godot_method_name);

        special_cases::maybe_rename_virtual_method(method_name)
    }
}

impl UtilityFunction {
    pub fn from_json(function: &JsonUtilityFunction, ctx: &mut Context) -> Option<Self> {
        if special_cases::is_utility_function_deleted(function, ctx) {
            return None;
        }

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
                parameters: FnParam::new_range(&function.arguments, ctx),
                return_value: FnReturn::new(&return_value, ctx),
                is_vararg: function.is_vararg,
                is_private: false,
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
        let is_bitfield = json_enum.is_bitfield;

        let rust_enum_name = conv::make_enum_name_str(godot_name);
        let rust_enumerator_names = {
            let godot_enumerator_names = json_enum.values.iter().map(|e| e.name.as_str()).collect();
            let godot_class_name = surrounding_class.as_ref().map(|ty| ty.godot_ty.as_str());

            conv::make_enumerator_names(godot_class_name, &rust_enum_name, godot_enumerator_names)
        };

        let enumerators = json_enum
            .values
            .iter()
            .zip(rust_enumerator_names)
            .map(|(json_constant, rust_name)| {
                Enumerator::from_json(json_constant, rust_name, is_bitfield)
            })
            .collect();

        Self {
            name: ident(&rust_enum_name),
            godot_name: godot_name.clone(),
            is_bitfield,
            enumerators,
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
