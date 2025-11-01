/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

#![allow(dead_code)] // TODO remove when mapped

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Domain models

use std::collections::HashMap;
use std::fmt;

use proc_macro2::{Ident, Literal, TokenStream};
use quote::{format_ident, quote, ToTokens};

use crate::context::Context;
use crate::conv;
use crate::models::json::{JsonMethodArg, JsonMethodReturn};
use crate::util::{ident, option_as_slice, safe_ident};

mod enums;

pub use enums::{Enum, Enumerator, EnumeratorValue};

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
            .unwrap_or_else(|| panic!("builtin_by_name: invalid `{name}`"))
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
    pub is_experimental: bool,
    pub is_final: bool,
    pub base_class: Option<TyName>,
    pub api_level: ClassCodegenLevel,
    pub constants: Vec<ClassConstant>,
    pub enums: Vec<Enum>,
    pub methods: Vec<ClassMethod>,
    pub signals: Vec<ClassSignal>,
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
    pub is_virtual_required: bool,
    /// Whether raw pointers appear in signature. Affects safety, and in case of virtual methods, the name.
    pub is_unsafe: bool,
    pub direction: FnDirection,
}

pub trait Function: fmt::Display {
    // Required:
    fn common(&self) -> &FunctionCommon;
    fn qualifier(&self) -> FnQualifier;
    fn surrounding_class(&self) -> Option<&TyName>;

    // Default:
    /// Rust name as string slice.
    fn name(&self) -> &str {
        &self.common().name
    }

    /// Rust name as `Ident`. Might be cached in future.
    fn name_ident(&self) -> Ident {
        safe_ident(self.name())
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
        matches!(self.direction(), FnDirection::Virtual { .. })
    }

    fn is_generic(&self) -> bool {
        matches!(self.return_value().type_, Some(RustTy::GenericArray))
    }

    fn direction(&self) -> FnDirection {
        self.common().direction
    }

    fn is_virtual_required(&self) -> bool {
        self.common().is_virtual_required
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

pub struct UtilityFunction {
    pub common: FunctionCommon,
}

impl UtilityFunction {
    pub fn hash(&self) -> i64 {
        match self.direction() {
            FnDirection::Virtual { .. } => unreachable!("utility function cannot be virtual"),
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
    /// Whether the method is directly exposed in the public-facing API, instead of the `Inner*` private struct.
    pub is_exposed_in_outer: bool,
}

impl BuiltinMethod {
    pub fn hash(&self) -> i64 {
        match self.direction() {
            FnDirection::Virtual { .. } => unreachable!("builtin method cannot be virtual"),
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

pub struct ClassSignal {
    pub name: String,
    pub parameters: Vec<FnParam>,
    pub surrounding_class: TyName,
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

#[derive(Copy, Clone, Debug)]
pub enum FnDirection {
    /// Godot -> Rust.
    Virtual {
        // Since PR https://github.com/godotengine/godot/pull/100674, virtual methods have a compat hash, too.
        #[cfg(since_api = "4.4")]
        hash: u32,
    },

    /// Rust -> Godot.
    Outbound { hash: i64 },
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
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

    /// Type, as it appears in `type CallSig` tuple definition.
    pub type_: RustTy,

    /// Rust expression for default value, if available.
    pub default_value: Option<TokenStream>,
}

impl FnParam {
    /// Creates a new parameter builder for constructing function parameters with configurable options.
    pub fn builder() -> FnParamBuilder {
        FnParamBuilder::new()
    }
}

/// Builder for constructing `FnParam` instances with configurable enum replacements and default value handling.
pub struct FnParamBuilder {
    replacements: EnumReplacements,
    no_defaults: bool,
}

impl FnParamBuilder {
    /// Creates a new parameter builder with default settings (no replacements, defaults enabled).
    pub fn new() -> Self {
        Self {
            replacements: &[],
            no_defaults: false,
        }
    }

    /// Configures the builder to use specific enum replacements.
    pub fn enum_replacements(mut self, replacements: EnumReplacements) -> Self {
        self.replacements = replacements;
        self
    }

    /// Configures the builder to exclude default values from generated parameters.
    pub fn no_defaults(mut self) -> Self {
        self.no_defaults = true;
        self
    }

    /// Builds a single function parameter from the provided JSON method argument.
    pub fn build_single(self, method_arg: &JsonMethodArg, ctx: &mut Context) -> FnParam {
        self.build_single_impl(method_arg, ctx)
    }

    /// Builds a vector of function parameters from the provided JSON method arguments.
    pub fn build_many(
        self,
        method_args: &Option<Vec<JsonMethodArg>>,
        ctx: &mut Context,
    ) -> Vec<FnParam> {
        option_as_slice(method_args)
            .iter()
            .map(|arg| self.build_single_impl(arg, ctx))
            .collect()
    }

    /// Core implementation for processing a single JSON method argument into a `FnParam`.
    fn build_single_impl(&self, method_arg: &JsonMethodArg, ctx: &mut Context) -> FnParam {
        let name = safe_ident(&method_arg.name);
        let type_ = conv::to_rust_type(&method_arg.type_, method_arg.meta.as_ref(), ctx);

        // Apply enum replacement if one exists for this parameter
        let matching_replacement = self
            .replacements
            .iter()
            .find(|(p, ..)| *p == method_arg.name);
        let type_ = if let Some((_, enum_name, is_bitfield)) = matching_replacement {
            if !type_.is_integer() {
                panic!(
                    "Parameter `{}` is of type {}, but can only replace int with enum",
                    method_arg.name, type_
                );
            }
            conv::to_enum_type_uncached(enum_name, *is_bitfield)
        } else {
            type_
        };

        let default_value = if self.no_defaults {
            None
        } else {
            method_arg
                .default_value
                .as_ref()
                .map(|v| conv::to_rust_expr(v, &type_))
        };

        FnParam {
            name,
            type_,
            default_value,
        }
    }
}

impl fmt::Debug for FnParam {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let def_val = self
            .default_value
            .as_ref()
            .map_or(String::new(), |v| format!(" (default {v})"));

        write!(f, "{}: {}{}", self.name, self.type_, def_val)
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

pub struct FnReturn {
    pub decl: TokenStream,
    pub type_: Option<RustTy>,
}

impl FnReturn {
    pub fn new(return_value: &Option<JsonMethodReturn>, ctx: &mut Context) -> Self {
        Self::with_enum_replacements(return_value, &[], ctx)
    }

    pub fn with_generic_builtin(generic_type: RustTy) -> Self {
        Self {
            decl: generic_type.return_decl(),
            type_: Some(generic_type),
        }
    }

    pub fn with_enum_replacements(
        return_value: &Option<JsonMethodReturn>,
        replacements: EnumReplacements,
        ctx: &mut Context,
    ) -> Self {
        if let Some(ret) = return_value {
            let ty = conv::to_rust_type(&ret.type_, ret.meta.as_ref(), ctx);

            // Apply enum replacement if one exists for return type (indicated by empty string)
            let matching_replacement = replacements.iter().find(|(p, ..)| p.is_empty());
            let ty = if let Some((_, enum_name, is_bitfield)) = matching_replacement {
                if !ty.is_integer() {
                    panic!(
                        "Return type is of type {}, but can only replace int with enum",
                        ty
                    );
                }
                conv::to_enum_type_uncached(enum_name, *is_bitfield)
            } else {
                ty
            };

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
            Some(ty) => ty.to_token_stream(),
            _ => quote! { () },
        }
    }

    pub fn generic_params(&self) -> Option<TokenStream> {
        self.type_.as_ref()?.generic_params()
    }

    pub fn where_clause(&self) -> Option<TokenStream> {
        self.type_.as_ref()?.where_clause()
    }

    pub fn call_result_decl(&self) -> TokenStream {
        let ret = self.type_tokens();
        quote! { -> Result<#ret, crate::meta::error::CallError> }
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Int->enum replacements

/// Replacement of int->enum in engine APIs; each tuple being `(param_name, enum_type, is_bitfield)`.
///
/// Empty string `""` is used as `param_name` to denote return type replacements.
pub type EnumReplacements = &'static [(&'static str, &'static str, bool)];

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Godot type

#[derive(Clone, Eq, PartialEq, Hash, Debug)]
pub struct GodotTy {
    pub ty: String,
    pub meta: Option<String>,
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Rust type

#[derive(Clone, Debug)]
pub enum RustTy {
    /// `bool`, `Vector3i`, `Array`, `GString`
    BuiltinIdent { ty: Ident, arg_passing: ArgPassing },

    /// Pointers declared in `gdextension_interface` such as `sys::GDExtensionInitializationFunction`
    /// used as parameters in some APIs.
    SysPointerType { tokens: TokenStream },

    /// `Array<i32>`
    ///
    /// Note that untyped arrays are mapped as `BuiltinIdent("Array")`.
    BuiltinArray { elem_type: TokenStream },

    /// Will be included as `Array<T>` in the generated source.
    ///
    /// Set by [`builtin_method_generic_ret`](crate::special_cases::builtin_method_generic_ret)
    GenericArray,

    /// C-style raw pointer to a `RustTy`.
    RawPointer { inner: Box<RustTy>, is_const: bool },

    /// `Array<Gd<PhysicsBody3D>>`. Never contains `Option` elements.
    EngineArray {
        tokens: TokenStream,

        #[allow(dead_code)] // Only read in minimal config.
        elem_class: String,
    },

    /// `module::Enum` or `module::Bitfield`
    EngineEnum {
        tokens: TokenStream,

        /// `None` for globals.
        #[allow(dead_code)] // Only read in minimal config.
        surrounding_class: Option<String>,

        is_bitfield: bool,
    },

    /// `Gd<Node>`
    EngineClass {
        /// Tokens with full `Gd<T>`, never `Option<Gd<T>>`.
        gd_tokens: TokenStream,

        /// Signature declaration with `impl AsArg<Gd<T>>` or `impl AsArg<Option<Gd<T>>>`.
        impl_as_object_arg: TokenStream,

        /// Only inner `Node`.
        inner_class: Ident,

        /// Whether this object parameter/return is nullable in the GDExtension API.
        ///
        /// Defaults to true (nullable). Only false when meta="required".
        is_nullable: bool,
    },

    /// Receiver type of default parameters extender constructor.
    ExtenderReceiver { tokens: TokenStream },
}

impl RustTy {
    pub fn param_decl(&self) -> TokenStream {
        match self {
            RustTy::EngineClass {
                impl_as_object_arg, ..
            } => impl_as_object_arg.clone(),
            other => other.to_token_stream(),
        }
    }

    pub fn return_decl(&self) -> TokenStream {
        match self {
            Self::GenericArray => quote! { -> Array<Ret> },
            _ => quote! { -> #self },
        }
    }

    /// Returns tokens without `Option<T>` wrapper, even for nullable engine classes.
    ///
    /// For `EngineClass`, always returns `Gd<T>` regardless of nullability. For other types, behaves the same as `ToTokens`.
    // Might also be useful to directly extract inner `gd_tokens` field.
    pub fn tokens_non_null(&self) -> TokenStream {
        match self {
            Self::EngineClass { gd_tokens, .. } => gd_tokens.clone(),
            other => other.to_token_stream(),
        }
    }

    pub fn generic_params(&self) -> Option<TokenStream> {
        if matches!(self, Self::GenericArray) {
            Some(quote! { < Ret > })
        } else {
            None
        }
    }

    pub fn where_clause(&self) -> Option<TokenStream> {
        if matches!(self, Self::GenericArray) {
            Some(quote! {
                where
                    Ret: crate::meta::ArrayElement,
            })
        } else {
            None
        }
    }

    pub fn is_integer(&self) -> bool {
        let RustTy::BuiltinIdent { ty, .. } = self else {
            return false;
        };

        // isize/usize currently not supported (2025-09), but this is more future-proof.
        matches!(
            ty.to_string().as_str(),
            "i8" | "i16" | "i32" | "i64" | "u8" | "u16" | "u32" | "u64" | "isize" | "usize"
        )
    }

    pub fn is_sys_pointer(&self) -> bool {
        let RustTy::RawPointer { inner, .. } = self else {
            return false;
        };
        matches!(**inner, RustTy::SysPointerType { .. })
    }
}

impl ToTokens for RustTy {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            RustTy::BuiltinIdent { ty: ident, .. } => ident.to_tokens(tokens),
            RustTy::BuiltinArray { elem_type } => elem_type.to_tokens(tokens),
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
            RustTy::EngineClass {
                is_nullable,
                gd_tokens: path,
                ..
            } => {
                // Return nullable-aware type: Option<Gd<T>> if nullable, else Gd<T>.
                if *is_nullable {
                    quote! { Option<#path> }.to_tokens(tokens)
                } else {
                    path.to_tokens(tokens)
                }
            }
            RustTy::ExtenderReceiver { tokens: path } => path.to_tokens(tokens),
            RustTy::GenericArray => quote! { Array<Ret> }.to_tokens(tokens),
            RustTy::SysPointerType { tokens: path } => path.to_tokens(tokens),
        }
    }
}

impl fmt::Display for RustTy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_token_stream().to_string().replace(" ", ""))
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum ArgPassing {
    ByValue,
    ByRef,
    ImplAsArg,
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

/// Behavior of virtual methods in derived classes.
pub enum VirtualMethodPresence {
    /// Preserve default behavior of base class (required or optional).
    Inherit,

    /// Virtual method is now required/optional according to `is_required`, independent of base method declaration.
    Override { is_required: bool },

    /// Virtual method is removed in derived classes (no longer appearing in their interface trait).
    Remove,
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

    /// Get name of virtual interface trait.
    ///
    /// This is also valid if the outer class generates no traits (e.g. to explicitly mention absence in docs).
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
    Core,
    Servers,
    Scene,
    Editor,
}

impl ClassCodegenLevel {
    pub fn with_tables() -> [Self; 4] {
        [Self::Core, Self::Servers, Self::Scene, Self::Editor]
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
            Self::Core => "core",
            Self::Servers => "servers",
            Self::Scene => "scene",
            Self::Editor => "editor",
        }
    }

    fn upper(self) -> &'static str {
        match self {
            Self::Core => "Core",
            Self::Servers => "Servers",
            Self::Scene => "Scene",
            Self::Editor => "Editor",
        }
    }

    pub fn to_init_level(self) -> TokenStream {
        match self {
            Self::Core => quote! { crate::init::InitLevel::Core },
            Self::Servers => quote! { crate::init::InitLevel::Servers },
            Self::Scene => quote! { crate::init::InitLevel::Scene },
            Self::Editor => quote! { crate::init::InitLevel::Editor },
        }
    }
}
