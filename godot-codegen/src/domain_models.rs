/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

#![allow(dead_code)] // TODO remove when mapped

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Domain models

use crate::context::Context;
use crate::json_models::{JsonMethodArg, JsonMethodReturn};
use crate::util::{option_as_slice, safe_ident, ClassCodegenLevel};
use crate::{conv, ModName, RustTy, TyName};

use proc_macro2::{Ident, Literal, TokenStream};
use quote::{format_ident, quote};
use std::fmt;

pub struct ExtensionApi {
    pub builtins: Vec<BuiltinVariant>,
    pub classes: Vec<Class>,
    pub singletons: Vec<Singleton>,
    pub native_structures: Vec<NativeStructure>,
    pub utility_functions: Vec<UtilityFunction>,
    pub global_enums: Vec<Enum>,
    pub build_config: [&'static str; 2],
}

impl ExtensionApi {
    /// O(n) search time, often leads to O(n^2), but less than 40 builtins total.
    pub fn builtin_by_original_name(&self, name: &str) -> &BuiltinVariant {
        self.builtins
            .iter()
            .find(|b| b.godot_original_name() == name)
            .unwrap_or_else(|| panic!("builtin_by_name: invalid `{}`", name))
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Builtins + classes + singletons

pub trait ClassLike {
    fn common(&self) -> &ClassCommons;

    fn name(&self) -> &TyName {
        &self.common().name
    }

    fn mod_name(&self) -> &ModName {
        &self.common().mod_name
    }
}

pub struct ClassCommons {
    pub name: TyName,
    pub mod_name: ModName,
}

pub struct BuiltinClass {
    pub(super) common: ClassCommons,
    pub(super) inner_name: TyName,
    pub methods: Vec<BuiltinMethod>,
    pub constructors: Vec<Constructor>,
    pub operators: Vec<Operator>,
    pub has_destructor: bool,
    pub enums: Vec<Enum>,
}

impl BuiltinClass {
    pub fn inner_name(&self) -> &Ident {
        &self.inner_name.rust_ty
    }
}

impl ClassLike for BuiltinClass {
    fn common(&self) -> &ClassCommons {
        &self.common
    }
}

/// All information about a builtin type, including its type (if available).
pub struct BuiltinVariant {
    pub(super) godot_original_name: String,
    pub(super) godot_shout_name: String,
    pub(super) godot_snake_name: String,
    pub(super) builtin_class: Option<BuiltinClass>,

    pub variant_type_ord: i32,
}

impl BuiltinVariant {
    /// Name in JSON for the class: `"int"` or `"PackedVector2Array"`.
    pub fn godot_original_name(&self) -> &str {
        &self.godot_original_name
    }

    /// Name in JSON: `"INT"` or `"PACKED_VECTOR2_ARRAY"`.
    pub fn godot_shout_name(&self) -> &str {
        &self.godot_shout_name
    }

    /// snake_case name like `"packed_vector2_array"`.
    pub fn snake_name(&self) -> &str {
        &self.godot_snake_name
    }

    /// Excludes variant types if:
    /// - There is no builtin class definition in the JSON. For example, `OBJECT` is a variant type but has no corresponding class.
    /// - The type is so trivial that most of its operations are directly provided by Rust, and there is no need
    ///   to expose the construct/destruct/operator methods from Godot (e.g. `int`, `bool`).
    ///
    /// See also `BuiltinClass::from_json()`
    pub fn associated_builtin_class(&self) -> Option<&BuiltinClass> {
        self.builtin_class.as_ref()
    }

    /// Returns an ident like `GDEXTENSION_VARIANT_TYPE_PACKED_VECTOR2_ARRAY`.
    pub fn sys_variant_type(&self) -> Ident {
        format_ident!("GDEXTENSION_VARIANT_TYPE_{}", self.godot_shout_name())
    }

    pub fn unsuffixed_ord_lit(&self) -> Literal {
        Literal::i32_unsuffixed(self.variant_type_ord)
    }
}

pub struct Class {
    pub(super) common: ClassCommons,
    pub is_refcounted: bool,
    pub is_instantiable: bool,
    pub inherits: Option<String>,
    pub api_level: ClassCodegenLevel,
    pub constants: Vec<ClassConstant>,
    pub enums: Vec<Enum>,
    pub methods: Vec<ClassMethod>,
}

impl ClassLike for Class {
    fn common(&self) -> &ClassCommons {
        &self.common
    }
}

pub struct NativeStructure {
    pub name: String,
    pub format: String,
}

pub struct Singleton {
    pub name: TyName,
    // Note: `type` currently has always same value as `name`, thus redundant
    // type_: String,
}

pub struct Enum {
    pub name: Ident,
    pub godot_name: String,
    pub is_bitfield: bool,
    pub enumerators: Vec<Enumerator>,
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Enumerators

pub struct Enumerator {
    pub name: Ident,

    pub godot_name: String,

    // i64 is common denominator for enum, bitfield and constant values.
    // Note that values > i64::MAX will be implicitly wrapped, see https://github.com/not-fl3/nanoserde/issues/89.
    pub value: EnumeratorValue,
}
pub enum EnumeratorValue {
    Enum(i32),
    Bitfield(u64),
}

impl EnumeratorValue {
    pub fn to_i64(&self) -> i64 {
        // Conversion is safe because i64 is used in the original JSON.
        match self {
            EnumeratorValue::Enum(i) => *i as i64,
            EnumeratorValue::Bitfield(i) => *i as i64,
        }
    }

    /// This method is needed for platform-dependent types like raw `VariantOperator`, which can be `i32` or `u32`.
    /// Do not suffix them.
    ///
    /// See also `BuiltinVariant::unsuffixed_ord_lit()`.
    pub fn unsuffixed_lit(&self) -> Literal {
        Literal::i64_unsuffixed(self.to_i64())
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Constants

trait Constant {
    fn name(&self) -> &str;
}

pub struct ClassConstant {
    pub name: String,
    pub value: ClassConstantValue,
}

pub enum ClassConstantValue {
    I32(i32),
    I64(i64),
}

/*
// Constants of builtin types have a string value like "Vector2(1, 1)", hence also a type field

pub struct BuiltinConstant {
    pub name: String,
    #[nserde(rename = "type")]
    pub type_: String,
    pub value: String,
}
*/

pub struct Operator {
    pub symbol: String,
    //pub right_type: Option<String>, // null if unary
    //pub return_type: String,
}

pub struct Constructor {
    pub index: usize,
    pub raw_parameters: Vec<JsonMethodArg>,
    // pub parameters: Vec<FnParam>,
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

/// Stuff that is in every of the "function" types.
pub struct FunctionCommon {
    pub name: String,
    pub godot_name: String,
    pub parameters: Vec<FnParam>,
    pub return_value: FnReturn,
    pub is_vararg: bool,
    pub is_private: bool,
    pub direction: FnDirection,
}

pub trait Function: fmt::Display {
    // Required:
    fn common(&self) -> &FunctionCommon;
    fn qualifier(&self) -> FnQualifier;
    fn surrounding_class(&self) -> Option<&TyName>;

    // Default:
    fn name(&self) -> &str {
        &self.common().name
    }
    fn godot_name(&self) -> &str {
        &self.common().godot_name
    }
    fn params(&self) -> &[FnParam] {
        &self.common().parameters
    }
    fn return_value(&self) -> &FnReturn {
        &self.common().return_value
    }
    fn is_vararg(&self) -> bool {
        self.common().is_vararg
    }
    fn is_private(&self) -> bool {
        self.common().is_private
    }
    fn is_virtual(&self) -> bool {
        matches!(self.direction(), FnDirection::Virtual)
    }
    fn direction(&self) -> FnDirection {
        self.common().direction
    }
}

#[deprecated]
struct FnSignature<'a> {
    function_name: &'a str,
    surrounding_class: Option<&'a TyName>, // None if global function
    is_private: bool,
    is_virtual: bool,
    is_vararg: bool,
    qualifier: FnQualifier,
    params: Vec<FnParam>,
    return_value: FnReturn,
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

pub struct UtilityFunction {
    pub(super) common: FunctionCommon,
}

impl Function for UtilityFunction {
    fn common(&self) -> &FunctionCommon {
        &self.common
    }

    fn qualifier(&self) -> FnQualifier {
        FnQualifier::Global
    }

    fn surrounding_class(&self) -> Option<&TyName> {
        None
    }
}

impl fmt::Display for UtilityFunction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "utility function `{}`", self.name())
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

pub struct BuiltinMethod {
    // variant_type:
    pub(super) common: FunctionCommon,
    pub(super) qualifier: FnQualifier,
    pub(super) surrounding_class: TyName,
}

impl BuiltinMethod {
    pub fn hash(&self) -> i64 {
        match self.direction() {
            FnDirection::Virtual => unreachable!("builtin method cannot be virtual"),
            FnDirection::Outbound { hash } => hash,
        }
    }
}

impl Function for BuiltinMethod {
    fn common(&self) -> &FunctionCommon {
        &self.common
    }

    fn qualifier(&self) -> FnQualifier {
        self.qualifier
    }

    fn surrounding_class(&self) -> Option<&TyName> {
        Some(&self.surrounding_class)
    }
}

impl fmt::Display for BuiltinMethod {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "builtin method `{}::{}`",
            self.surrounding_class.rust_ty,
            self.name(),
        )
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

pub struct ClassMethod {
    pub(super) common: FunctionCommon,
    pub(super) qualifier: FnQualifier,
    pub(super) surrounding_class: TyName,
}

impl ClassMethod {}

impl Function for ClassMethod {
    fn common(&self) -> &FunctionCommon {
        &self.common
    }

    fn qualifier(&self) -> FnQualifier {
        self.qualifier
    }

    fn surrounding_class(&self) -> Option<&TyName> {
        Some(&self.surrounding_class)
    }
}

impl fmt::Display for ClassMethod {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "class method `{}::{}`",
            self.surrounding_class.rust_ty,
            self.name(),
        )
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

#[derive(Copy, Clone, Debug)]
pub enum FnDirection {
    /// Godot -> Rust.
    Virtual,

    /// Rust -> Godot.
    Outbound { hash: i64 },
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum FnQualifier {
    Mut,    // &mut self
    Const,  // &self
    Static, // Self
    Global, // (nothing) // TODO remove?
}

impl FnQualifier {
    pub fn from_const_static(is_const: bool, is_static: bool) -> FnQualifier {
        if is_static {
            assert!(
                !is_const,
                "const and static qualifiers are mutually exclusive"
            );
            FnQualifier::Static
        } else if is_const {
            FnQualifier::Const
        } else {
            FnQualifier::Mut
        }
    }

    pub fn is_static_or_global(&self) -> bool {
        matches!(self, Self::Static | Self::Global)
    }
}
// ----------------------------------------------------------------------------------------------------------------------------------------------

pub struct FnParam {
    pub name: Ident,
    pub type_: RustTy,
    pub default_value: Option<TokenStream>,
}

impl FnParam {
    pub fn new_range(method_args: &Option<Vec<JsonMethodArg>>, ctx: &mut Context) -> Vec<FnParam> {
        option_as_slice(method_args)
            .iter()
            .map(|arg| Self::new(arg, ctx))
            .collect()
    }

    pub fn new_range_no_defaults(
        method_args: &Option<Vec<JsonMethodArg>>,
        ctx: &mut Context,
    ) -> Vec<FnParam> {
        option_as_slice(method_args)
            .iter()
            .map(|arg| Self::new_no_defaults(arg, ctx))
            .collect()
    }

    pub fn new(method_arg: &JsonMethodArg, ctx: &mut Context) -> FnParam {
        let name = safe_ident(&method_arg.name);
        let type_ = conv::to_rust_type(&method_arg.type_, method_arg.meta.as_ref(), ctx);
        let default_value = method_arg
            .default_value
            .as_ref()
            .map(|v| conv::to_rust_expr(v, &type_));

        FnParam {
            name,
            type_,
            default_value,
        }
    }

    pub fn new_no_defaults(method_arg: &JsonMethodArg, ctx: &mut Context) -> FnParam {
        FnParam {
            name: safe_ident(&method_arg.name),
            type_: conv::to_rust_type(&method_arg.type_, method_arg.meta.as_ref(), ctx),
            //type_: to_rust_type(&method_arg.type_, &method_arg.meta, ctx),
            default_value: None,
        }
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

pub struct FnReturn {
    pub decl: TokenStream,
    pub type_: Option<RustTy>,
}

impl FnReturn {
    pub fn new(return_value: &Option<JsonMethodReturn>, ctx: &mut Context) -> Self {
        if let Some(ret) = return_value {
            let ty = conv::to_rust_type(&ret.type_, ret.meta.as_ref(), ctx);

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

    pub fn type_tokens(&self) -> TokenStream {
        match &self.type_ {
            Some(RustTy::EngineClass { tokens, .. }) => {
                quote! { Option<#tokens> }
            }
            Some(ty) => {
                quote! { #ty }
            }
            _ => {
                quote! { () }
            }
        }
    }
}
