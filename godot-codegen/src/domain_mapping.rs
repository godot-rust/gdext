/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::context::Context;
use crate::domain_models::{
    BuiltinMethod, ClassMethod, FnDirection, FnParam, FnQualifier, FnReturn, FunctionCommon,
    UtilityFunction,
};
use crate::json_models::{
    JsonBuiltinMethod, JsonClassMethod, JsonMethodReturn, JsonUtilityFunction,
};
use crate::{special_cases, TyName};

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
