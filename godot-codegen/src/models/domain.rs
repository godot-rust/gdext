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
use crate::conv;
use crate::models::json::{JsonMethodArg, JsonMethodReturn};
use crate::util::{ident, option_as_slice, safe_ident};

use proc_macro2::{Ident, Literal, TokenStream};
use quote::{format_ident, quote, ToTokens};
use std::collections::HashMap;
use std::fmt;

pub struct ExtensionApi {
    pub builtins: Vec<BuiltinVariant>,
    pub classes: Vec<Class>,
    pub singletons: Vec<Singleton>,
    pub native_structures: Vec<NativeStructure>,
    pub utility_functions: Vec<UtilityFunction>,
    pub global_enums: Vec<Enum>,
    pub godot_version: GodotApiVersion,

    /// Map `(original Godot name, build config) -> builtin size` in bytes.
    pub builtin_sizes: Vec<BuiltinSize>,
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
// View and indexing over existing ExtensionApi

pub struct ApiView<'a> {
    class_by_ty: HashMap<TyName, &'a Class>,
}

impl<'a> ApiView<'a> {
    pub fn new(api: &'a ExtensionApi) -> ApiView<'a> {
        let class_by_ty = api.classes.iter().map(|c| (c.name().clone(), c)).collect();

        Self { class_by_ty }
    }

    pub fn get_engine_class(&self, ty: &TyName) -> &'a Class {
        self.class_by_ty
            .get(ty)
            .unwrap_or_else(|| panic!("specified type `{}` is not an engine class", ty.godot_ty))
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
    pub common: ClassCommons,
    pub inner_name: TyName,
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
    pub godot_original_name: String,
    pub godot_shout_name: String,
    pub godot_snake_name: String,
    pub builtin_class: Option<BuiltinClass>,

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
    pub common: ClassCommons,
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
// Build config + version

#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub enum BuildConfiguration {
    Float32,
    Float64,
    Double32,
    Double64,
}

impl BuildConfiguration {
    #[cfg(feature = "double-precision")]
    pub fn is_applicable(self) -> bool {
        matches!(self, Self::Double32 | Self::Double64)
    }

    #[cfg(not(feature = "double-precision"))]
    pub fn is_applicable(self) -> bool {
        matches!(self, Self::Float32 | Self::Float64)
    }

    // Rewrite the above using #[cfg].
    #[cfg(feature = "double-precision")]
    pub fn all_applicable() -> [BuildConfiguration; 2] {
        [BuildConfiguration::Double32, BuildConfiguration::Double64]
    }

    #[cfg(not(feature = "double-precision"))]
    pub fn all_applicable() -> [BuildConfiguration; 2] {
        [BuildConfiguration::Float32, BuildConfiguration::Float64]
    }

    pub fn is_64bit(self) -> bool {
        matches!(self, Self::Float64 | Self::Double64)
    }
}

pub struct BuiltinSize {
    pub builtin_original_name: String,
    pub config: BuildConfiguration,
    pub size: usize,
}

/// Godot API version (from the JSON; not runtime version).
// Could be consolidated with versions in other part of codegen, e.g. the one in godot-bindings.
#[derive(Clone)]
pub struct GodotApiVersion {
    pub major: u8,
    pub minor: u8,
    pub patch: u8,

    /// Without "Godot Engine " prefix.
    pub version_string: String,
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

// ----------------------------------------------------------------------------------------------------------------------------------------------

pub struct UtilityFunction {
    pub common: FunctionCommon,
}

impl UtilityFunction {
    pub fn hash(&self) -> i64 {
        match self.direction() {
            FnDirection::Virtual => unreachable!("utility function cannot be virtual"),
            FnDirection::Outbound { hash } => hash,
        }
    }
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
    pub common: FunctionCommon,
    pub qualifier: FnQualifier,
    pub surrounding_class: TyName,
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
    pub common: FunctionCommon,
    pub qualifier: FnQualifier,
    pub surrounding_class: TyName,
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

    pub fn call_result_decl(&self) -> TokenStream {
        let ret = self.type_tokens();
        quote! { -> Result<#ret, crate::builtin::meta::CallError> }
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Godot type

#[derive(Clone, Eq, PartialEq, Hash, Debug)]
pub struct GodotTy {
    pub ty: String,
    pub meta: Option<String>,
}

// impl GodotTy {
//     fn new<'a>(ty: &'a String, meta: &'a Option<String>) -> Self {
//         Self {
//             ty: ty.clone(),
//             meta: meta.clone(),
//         }
//     }
// }

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Rust type

#[derive(Clone, Debug)]
pub enum RustTy {
    /// `bool`, `Vector3i`
    BuiltinIdent(Ident),

    /// `Array<i32>`
    BuiltinArray(TokenStream),

    /// C-style raw pointer to a `RustTy`.
    RawPointer { inner: Box<RustTy>, is_const: bool },

    /// `Array<Gd<PhysicsBody3D>>`
    EngineArray {
        tokens: TokenStream,
        #[allow(dead_code)] // only read in minimal config
        elem_class: String,
    },

    /// `module::Enum`
    EngineEnum {
        tokens: TokenStream,
        /// `None` for globals
        #[allow(dead_code)] // only read in minimal config
        surrounding_class: Option<String>,
    },

    /// `module::Bitfield`
    EngineBitfield {
        tokens: TokenStream,
        /// `None` for globals
        #[allow(dead_code)] // only read in minimal config
        surrounding_class: Option<String>,
    },

    /// `Gd<Node>`
    EngineClass {
        /// Tokens with full `Gd<T>`
        tokens: TokenStream,
        /// only inner `T`
        #[allow(dead_code)] // only read in minimal config
        inner_class: Ident,
    },
}

impl RustTy {
    pub fn return_decl(&self) -> TokenStream {
        match self {
            Self::EngineClass { tokens, .. } => quote! { -> Option<#tokens> },
            other => quote! { -> #other },
        }
    }
}

impl ToTokens for RustTy {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            RustTy::BuiltinIdent(ident) => ident.to_tokens(tokens),
            RustTy::BuiltinArray(path) => path.to_tokens(tokens),
            RustTy::RawPointer {
                inner,
                is_const: true,
            } => quote! { *const #inner }.to_tokens(tokens),
            RustTy::RawPointer {
                inner,
                is_const: false,
            } => quote! { *mut #inner }.to_tokens(tokens),
            RustTy::EngineArray { tokens: path, .. } => path.to_tokens(tokens),
            RustTy::EngineEnum { tokens: path, .. } => path.to_tokens(tokens),
            RustTy::EngineBitfield { tokens: path, .. } => path.to_tokens(tokens),
            RustTy::EngineClass { tokens: path, .. } => path.to_tokens(tokens),
        }
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

/// Contains multiple naming conventions for types (classes, builtin classes, enums).
#[derive(Clone, Eq, PartialEq, Hash)]
pub struct TyName {
    pub godot_ty: String,
    pub rust_ty: Ident,
}

impl TyName {
    pub fn from_godot(godot_ty: &str) -> Self {
        Self {
            godot_ty: godot_ty.to_owned(),
            rust_ty: ident(&conv::to_pascal_case(godot_ty)),
        }
    }

    pub fn description(&self) -> String {
        if self.rust_ty == self.godot_ty {
            self.godot_ty.clone()
        } else {
            format!("{}  [renamed {}]", self.godot_ty, self.rust_ty)
        }
    }

    pub fn virtual_trait_name(&self) -> String {
        format!("I{}", self.rust_ty)
    }
}

impl ToTokens for TyName {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.rust_ty.to_tokens(tokens)
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

/// Contains naming conventions for modules.
#[derive(Clone)]
pub struct ModName {
    // godot_mod: String,
    pub rust_mod: Ident,
}

impl ModName {
    pub fn from_godot(godot_ty: &str) -> Self {
        Self {
            // godot_mod: godot_ty.to_owned(),
            rust_mod: ident(&conv::to_snake_case(godot_ty)),
        }
    }
}

impl ToTokens for ModName {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.rust_mod.to_tokens(tokens)
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

/// At which stage a class function pointer is loaded.
#[derive(Copy, Clone, Eq, PartialEq, Hash)]
pub enum ClassCodegenLevel {
    Servers,
    Scene,
    Editor,
}

impl ClassCodegenLevel {
    pub fn with_tables() -> [Self; 3] {
        [Self::Servers, Self::Scene, Self::Editor]
    }

    pub fn table_global_getter(self) -> Ident {
        format_ident!("class_{}_api", self.lower())
    }

    pub fn table_file(self) -> String {
        format!("table_{}_classes.rs", self.lower())
    }

    pub fn table_struct(self) -> Ident {
        format_ident!("Class{}MethodTable", self.upper())
    }

    pub fn lower(self) -> &'static str {
        match self {
            Self::Servers => "servers",
            Self::Scene => "scene",
            Self::Editor => "editor",
        }
    }

    fn upper(self) -> &'static str {
        match self {
            Self::Servers => "Servers",
            Self::Scene => "Scene",
            Self::Editor => "Editor",
        }
    }

    pub fn to_init_level(self) -> TokenStream {
        match self {
            Self::Servers => quote! { crate::init::InitLevel::Servers },
            Self::Scene => quote! { crate::init::InitLevel::Scene },
            Self::Editor => quote! { crate::init::InitLevel::Editor },
        }
    }
}
