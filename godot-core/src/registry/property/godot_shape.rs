/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::borrow::Cow;

use godot_ffi as sys;
use godot_ffi::VariantType;

use crate::builtin::{CowStr, GString};
use crate::global::{PropertyHint, PropertyUsageFlags, godot_str};
use crate::meta::{ClassId, GodotConvert, PropertyHintInfo};
use crate::obj::EngineEnum as _;

/// The "shape" of a Godot type: whether it's a builtin, a class, an enum/bitfield, etc.
///
/// Describes how a type should be registered as a Godot property; returned by [`GodotConvert::godot_shape()`]. godot-rust will then derive all
/// property hints, class names, usage flags and collection element metadata from this single declaration. Most types use `Builtin` (the default).
///
/// # Registration flow
///
/// The property registration pipeline works as follows:
///
/// 1. The proc macro (`#[derive(GodotClass)]`) computes hint info at compile time. For `#[export]` fields,
///    this may include type-specific export hints (e.g. resource pickers). For `#[var]` fields, it uses
///    `var_hint()` or `export_hint()` depending on context.
/// 2. At runtime, the macro calls [`register_var()`] or [`register_export()`], passing the pre-computed `hint_info`
///    and `usage` flags. These functions call [`into_property_info()`](Self::into_property_info) to combine
///    the hint with structural metadata (variant type, class ID, shape-specific usage flags) from `godot_shape()`.
/// 3. The resulting [`PropertyInfo`] is passed to the same FFI call (`classdb_register_extension_class_property`)
///    for both `#[var]` and `#[export]`.
///
/// # Var vs. export hints
///
/// Some predefined shapes produce **different** property hints depending on whether the property is registered with `#[var]` or `#[export]`.
/// For example, a `PackedByteArray` uses `NONE` hint for `#[var]` but `TYPE_STRING` with element type for `#[export]`.
/// This is handled by the two separate methods [`var_hint()`](Self::var_hint) and [`export_hint()`](Self::export_hint).
///
/// While both `#[var]` and `#[export]` go through the same FFI call, they differ in two orthogonal ways:
/// - **Hints** control editor presentation (dropdowns, sliders, resource pickers). These are type-dependent.
/// - **Usage flags** control persistence and visibility (`NONE` vs `DEFAULT`). These are context-dependent.
///
/// The [`Custom`](Self::Custom) variant stores separate var/export hints because it is the escape hatch for types
/// needing different editor presentation per context.
///
/// # Override mechanism
///
/// The `#[var]` attribute allows overriding hints via `#[var(hint = SOME_HINT, hint_string = "...")]`. When specified, these
/// **replace** the shape-derived hints entirely — they are passed directly to `register_var()`/`register_export()` instead of
/// calling `var_hint()`/`export_hint()`. There is currently no separate hint override API on `#[export]`; the `#[var]` override
/// applies in both contexts.
///
/// [`register_var()`]: crate::registry::godot_register_wrappers::register_var
/// [`register_export()`]: crate::registry::godot_register_wrappers::register_export
/// [`PropertyInfo`]: crate::meta::PropertyInfo
/// [`GodotConvert::godot_shape()`]: GodotConvert::godot_shape
#[non_exhaustive]
pub enum GodotShape {
    /// The untyped `Variant` type. Can hold any Godot value.
    ///
    /// Distinct from `Builtin { variant_type: NIL }`, which represents void (`()`).
    Variant,

    /// A builtin Godot type (int, float, String, Vector3, `PackedByteArray`, etc.). All property metadata derived from the `Via` type's defaults.
    ///
    /// Packed arrays (e.g. `PACKED_BYTE_ARRAY`) are also represented as `Builtin`; their element type for export hints is inferred via
    /// [`packed_element_variant_type()`](fn@packed_element_variant_type). Never used for Godot object types (`Gd<T>` etc.); those use
    /// [`GodotShape::Class`].
    Builtin {
        /// Godot variant type (e.g. `INT`, `FLOAT`, `STRING`, `VECTOR3`, `PACKED_BYTE_ARRAY`). Never `OBJECT`.
        variant_type: VariantType,
    },

    /// A Godot object type (`Gd<T>`, `Option<Gd<T>>`, etc.). Always has `VariantType::OBJECT`.
    Class {
        /// The Godot class of this object type (e.g. `Node`, `Resource`).
        class_id: ClassId,

        /// Whether this inherits from `Resource`, `Node`, or other object class.
        heritage: ClassHeritage,
    },

    /// An `Array<T>` where `T` is not `Variant`.
    ///
    /// Untyped arrays are represented as `Builtin { variant_type: VariantType::ARRAY }`.
    TypedArray {
        /// Shape of the array element type.
        element_shape: Box<GodotShape>,
    },

    /// A `Dictionary<K, V>` where at least one of `K`, `V` is not `Variant` (Godot 4.4+).
    ///
    /// Untyped dictionaries are represented as `Builtin { variant_type: VariantType::DICTIONARY }`.
    TypedDictionary {
        /// Shape of the dictionary key type.
        key_shape: Box<GodotShape>,
        /// Shape of the dictionary value type.
        value_shape: Box<GodotShape>,
    },

    /// An enum or bitfield type (engine-defined or user-defined).
    Enum {
        /// Godot variant type of the underlying representation (typically `INT` for int-backed enums, `STRING` for string-backed).
        variant_type: VariantType,

        /// Display name and ordinal for each enumerator. `Borrowed` for compile-time data, `Owned` for dynamic enumerators.
        enumerators: Cow<'static, [Enumerator]>,

        /// Godot-qualified enum name. `Some("Orientation")` or `Some("Node.ProcessMode")` for engine enums; `None` for user enums.
        ///
        /// When `Some`, the framework sets `class_name` + `CLASS_IS_ENUM` in `PropertyInfo` and uses `NONE` hint for `#[var]`
        /// (Godot resolves the enum from class_name). When `None`, uses `ENUM`/`FLAGS` hint with hint_string directly.
        ///
        // TODO(v0.6): In future, user enums could set this to register constants with Godot via `classdb_register_extension_class_integer_constant`,
        // enabling GDScript to reference them by name. The `is_bitfield` field maps to that FFI method's `p_is_bitfield` parameter.
        // Decide if we should even *require* enums to be associated with Godot classes or globally -- meaning this would become non-optional.
        // There would need to be a differentiator for "inside a class" or "global" (but still registered with Godot).
        godot_name: Option<CowStr>,

        /// Whether this is a bitfield:
        /// * `true` for bitfields (`FLAGS` hint, `CLASS_IS_BITFIELD` usage).
        /// * `false` for regular enums (`ENUM` hint, `CLASS_IS_ENUM` usage).
        is_bitfield: bool,
    },

    /// Fully custom property metadata. Use only when the type doesn't fit the categories above.
    Custom {
        /// Godot variant type.
        variant_type: VariantType,
        /// Property hint for `#[var]` context.
        var_hint: PropertyHint,
        /// Property hint string for `#[var]` context.
        var_hint_string: GString,
        /// Property hint for `#[export]` context.
        export_hint: PropertyHint,
        /// Property hint string for `#[export]` context.
        export_hint_string: GString,
        /// Stored as `CowStr`; converted to `ClassId` only at registration time to avoid eager global cache allocation.
        class_name: Option<CowStr>,
        usage_flags: PropertyUsageFlags,
    },
}

impl GodotShape {
    /// Creates `GodotShape::Builtin` for a type `T` directly representable as a Godot builtin, including packed arrays.
    ///
    /// Returns either of:
    /// * Variant, if `T` is `Variant`.
    /// * `GodotShape::Builtin { variant_type: ffi_variant_type::<T>().variant_as_nil() }` for other builtins.
    ///
    /// Do not use for objects, typed arrays/dictionaries or enums; those have their own shape variants.
    pub fn of_builtin<T: GodotConvert>() -> Self {
        match crate::meta::ffi_variant_type::<T>() {
            sys::ExtVariantType::Variant => Self::Variant,
            ext => Self::Builtin {
                variant_type: ext.variant_as_nil(),
            },
        }
    }

    /// Returns the Godot `VariantType` for this shape.
    pub fn variant_type(&self) -> VariantType {
        match self {
            Self::Variant => VariantType::NIL,
            Self::Builtin { variant_type } => *variant_type,
            Self::Class { .. } => VariantType::OBJECT,
            Self::Enum { variant_type, .. } => *variant_type,
            Self::Custom { variant_type, .. } => *variant_type,
            Self::TypedArray { .. } => VariantType::ARRAY,
            Self::TypedDictionary { .. } => VariantType::DICTIONARY,
        }
    }

    /// Property hint for `#[var]` context.
    pub fn var_hint(&self) -> PropertyHintInfo {
        match self {
            Self::Variant | Self::Builtin { .. } | Self::Class { .. } => PropertyHintInfo::none(),
            Self::Enum {
                godot_name,
                enumerators,
                is_bitfield,
                ..
            } => enum_hint_info(enumerators, *is_bitfield, godot_name.is_some()),

            Self::Custom {
                var_hint,
                var_hint_string,
                ..
            } => PropertyHintInfo {
                hint: *var_hint,
                hint_string: var_hint_string.clone(),
            },

            Self::TypedArray { element_shape } => PropertyHintInfo {
                hint: PropertyHint::ARRAY_TYPE,
                hint_string: GString::from(&element_shape.element_godot_type_name()),
            },

            Self::TypedDictionary {
                key_shape,
                value_shape,
            } => {
                // PropertyHint::DICTIONARY_TYPE, only available since Godot 4.4 -- so the `if` is essentially a version check.
                if let Some(hint) = PropertyHint::try_from_ord(38) {
                    PropertyHintInfo {
                        hint,
                        hint_string: godot_str!(
                            "{};{}",
                            key_shape.element_godot_type_name(),
                            value_shape.element_godot_type_name()
                        ),
                    }
                } else {
                    let _unused = (key_shape, value_shape);
                    PropertyHintInfo::none()
                }
            }
        }
    }

    /// Property hint for `#[export]` context.
    ///
    /// For enums, always uses `ENUM`/`FLAGS` + hint_string (even engine enums need explicit hints for export).
    pub fn export_hint(&self) -> PropertyHintInfo {
        match self {
            Self::Variant => PropertyHintInfo::none(),

            Self::Builtin { variant_type } => {
                // In 4.3+, packed arrays use a TYPE_STRING hint with their element type.
                // See https://github.com/godotengine/godot/pull/82952.
                if sys::GdextBuild::since_api("4.3") {
                    if let Some(elem_vtype) = packed_element_variant_type(*variant_type) {
                        return PropertyHintInfo {
                            hint: PropertyHint::TYPE_STRING,
                            hint_string: godot_str!("{}:", elem_vtype.ord()),
                        };
                    }
                    PropertyHintInfo::none()
                } else {
                    // Pre-4.3 Godot uses the type name in hint_string even with NONE hint.
                    PropertyHintInfo {
                        hint: PropertyHint::NONE,
                        hint_string: GString::from(variant_type.godot_type_name()),
                    }
                }
            }

            Self::Class { class_id, heritage } => match heritage {
                ClassHeritage::Node => PropertyHintInfo {
                    hint: PropertyHint::NODE_TYPE,
                    hint_string: class_id.to_gstring(),
                },
                ClassHeritage::Resource => PropertyHintInfo {
                    hint: PropertyHint::RESOURCE_TYPE,
                    hint_string: class_id.to_gstring(),
                },
                ClassHeritage::DynResource { implementors } => PropertyHintInfo {
                    hint: PropertyHint::RESOURCE_TYPE,
                    hint_string: GString::from(&join_class_ids(implementors)),
                },
                ClassHeritage::Other => PropertyHintInfo::none(),
            },

            Self::TypedArray { element_shape } => PropertyHintInfo {
                hint: PropertyHint::TYPE_STRING,
                hint_string: GString::from(&element_shape.element_type_string()),
            },

            Self::TypedDictionary {
                key_shape,
                value_shape,
            } => {
                if sys::GdextBuild::since_api("4.4") {
                    PropertyHintInfo {
                        hint: PropertyHint::TYPE_STRING,
                        hint_string: godot_str!(
                            "{};{}",
                            key_shape.element_type_string(),
                            value_shape.element_type_string()
                        ),
                    }
                } else {
                    PropertyHintInfo::none()
                }
            }

            Self::Enum {
                enumerators,
                is_bitfield,
                ..
            } => enum_hint_info(enumerators, *is_bitfield, false),

            Self::Custom {
                export_hint,
                export_hint_string,
                ..
            } => PropertyHintInfo {
                hint: *export_hint,
                hint_string: export_hint_string.clone(),
            },
        }
    }

    /// Converts `godot_name`/`class_name` to `ClassId`. Only called during registration.
    //
    // For engine enums, this inserts the enum's qualified name (e.g. `"Node.ProcessMode"`) into the global `ClassId` cache.
    // This is conceptually wrong — enum names aren't class names — but practically harmless: the cache is an append-only
    // string-intern table, and enum names (containing `.`) never collide with real class names. The proper fix is to change
    // `PropertyInfo::class_id` from `ClassId` to `StringName`, avoiding the cache entirely for non-class names.
    // TODO(v0.5): change PropertyInfo::class_id to StringName to avoid this.
    pub(crate) fn class_name_or_none(&self) -> ClassId {
        match self {
            Self::Variant
            | Self::Builtin { .. }
            | Self::TypedArray { .. }
            | Self::TypedDictionary { .. } => ClassId::none(),
            Self::Class { class_id, .. } => *class_id,
            Self::Enum { godot_name, .. } => match godot_name {
                Some(name) => ClassId::new_dynamic(name.clone()),
                None => ClassId::none(),
            },
            Self::Custom { class_name, .. } => match class_name {
                Some(name) => ClassId::new_dynamic(name.clone()),
                None => ClassId::none(),
            },
        }
    }

    /// Additional usage flags for property registration.
    ///
    /// Only engine enums (those with `godot_name`) get `CLASS_IS_ENUM`. User enums don't set this flag: `CLASS_IS_ENUM` tells Godot to resolve
    /// the enum's enumerators via `class_name` in ClassDB -- but user enums aren't registered there yet. Setting the flag without a valid
    /// `class_name` would cause Godot to look up a nonexistent name. Once we call `classdb_register_extension_class_integer_constant` for user
    /// enums (making them visible to GDScript by name), they can set `godot_name` and get `CLASS_IS_ENUM` automatically.
    pub(crate) fn usage_flags(&self) -> PropertyUsageFlags {
        match self {
            Self::Variant => PropertyUsageFlags::NIL_IS_VARIANT,

            Self::Builtin { .. }
            | Self::Class { .. }
            | Self::TypedArray { .. }
            | Self::TypedDictionary { .. } => PropertyUsageFlags::NONE,

            Self::Enum {
                godot_name,
                is_bitfield,
                ..
            } => match (godot_name, *is_bitfield) {
                (Some(_), true) => PropertyUsageFlags::CLASS_IS_BITFIELD,
                (Some(_), false) => PropertyUsageFlags::CLASS_IS_ENUM,
                (None, _) => PropertyUsageFlags::NONE, // User enums are currently not yet registered.
            },

            Self::Custom { usage_flags, .. } => *usage_flags,
        }
    }

    /// Builds the low-level Godot property info for `#[var]` context.
    ///
    /// Uses [`var_hint()`](Self::var_hint) and [`NONE`](PropertyUsageFlags::NONE) base usage (property is accessible from GDScript,
    /// but not shown in editor or saved).
    pub fn to_var_property(self, property_name: &str) -> crate::meta::PropertyInfo {
        let hint_info = self.var_hint();
        self.into_property_info(property_name, hint_info, PropertyUsageFlags::NONE)
    }

    /// Builds the low-level Godot property info for `#[export]` context.
    ///
    /// Uses [`export_hint()`](Self::export_hint) and [`DEFAULT`](PropertyUsageFlags::DEFAULT) base usage (property is shown in editor and saved).
    pub fn to_export_property(self, property_name: &str) -> crate::meta::PropertyInfo {
        let hint_info = self.export_hint();
        self.into_property_info(property_name, hint_info, PropertyUsageFlags::DEFAULT)
    }

    /// Low-level builder for [`PropertyInfo`]. Derives `class_id`, `variant_type`, and shape-specific usage flags from `self`,
    /// but takes `hint_info` and `base_usage` as parameters because they depend on context (`#[var]` vs `#[export]`) and may
    /// be overridden by the user (e.g. `#[var(hint = ..., hint_string = "...")]`).
    ///
    /// Prefer [`to_var_property_info()`](Self::to_var_property) or [`to_export_property_info()`](Self::to_export_property)
    /// when no user override is involved. This method is used directly by [`register_var`](crate::registry::godot_register_wrappers::register_var)
    /// and [`register_export`](crate::registry::godot_register_wrappers::register_export), which receive pre-computed hints from the macro.
    pub(crate) fn into_property_info(
        self,
        property_name: &str,
        hint_info: PropertyHintInfo,
        base_usage: PropertyUsageFlags,
    ) -> crate::meta::PropertyInfo {
        use crate::builtin::StringName;

        let class_id = self.class_name_or_none();
        let variant_type = self.variant_type();
        let usage = base_usage | self.usage_flags();

        crate::meta::PropertyInfo {
            variant_type,
            class_id,
            property_name: StringName::from(property_name),
            hint_info,
            usage,
        }
    }

    /// Builds `"{vtype}/{hint}:{hint_string}"` for typed collections (e.g. `Array<MyEnum>`).
    pub(crate) fn element_type_string(&self) -> String {
        match self {
            Self::Variant
            | Self::Builtin {
                variant_type: VariantType::NIL,
            } => {
                // Variant (or void) as element: untyped, no hint.
                format!("{}:", VariantType::NIL.ord())
            }

            Self::Builtin { variant_type } => {
                if sys::GdextBuild::since_api("4.3") {
                    format!("{}:", variant_type.ord())
                } else {
                    format!("{}:{}", variant_type.ord(), variant_type.godot_type_name())
                }
            }

            Self::Class { class_id, heritage } => {
                let export_hint = heritage.export_property_hint();
                assert_ne!(
                    export_hint,
                    PropertyHint::NONE,
                    "element_type_string() should only be called for exportable object classes (Resource or Node), \
                     but got ClassAncestor::Other for class `{}`",
                    class_id.to_cow_str()
                );

                let hint_string = match heritage {
                    ClassHeritage::DynResource { implementors } => join_class_ids(implementors),
                    _ => class_id.to_cow_str().to_string(),
                };
                format!(
                    "{}/{}:{}",
                    VariantType::OBJECT.ord(),
                    export_hint.ord(),
                    hint_string
                )
            }

            Self::TypedArray { .. } | Self::TypedDictionary { .. } | Self::Enum { .. } => {
                let variant_type = self.variant_type();
                let info = self.export_hint();
                if info.hint == PropertyHint::NONE {
                    format!("{}:", variant_type.ord())
                } else {
                    format!(
                        "{}/{}:{}",
                        variant_type.ord(),
                        info.hint.ord(),
                        info.hint_string
                    )
                }
            }

            Self::Custom {
                variant_type,
                var_hint,
                var_hint_string,
                ..
            } => {
                if *var_hint == PropertyHint::NONE {
                    format!("{}:", variant_type.ord())
                } else {
                    format!(
                        "{}/{}:{}",
                        variant_type.ord(),
                        var_hint.ord(),
                        var_hint_string
                    )
                }
            }
        }
    }

    /// Returns the Godot type name for use in `#[var]` array/dictionary type hints.
    pub(crate) fn element_godot_type_name(&self) -> String {
        match self {
            Self::Variant => VariantType::NIL.godot_type_name().to_string(),
            Self::Builtin { variant_type } => variant_type.godot_type_name().to_string(),
            Self::Class { class_id, .. } => class_id.to_cow_str().to_string(),
            Self::Enum {
                godot_name,
                variant_type,
                ..
            } => match godot_name {
                Some(name) => name.to_string(),
                None => variant_type.godot_type_name().to_string(),
            },
            Self::Custom { variant_type, .. } => variant_type.godot_type_name().to_string(),
            Self::TypedArray { .. } => VariantType::ARRAY.godot_type_name().to_string(),
            Self::TypedDictionary { .. } => VariantType::DICTIONARY.godot_type_name().to_string(),
        }
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// ClassAncestor

/// Which tree in the Godot hierarchy a class belongs to; determines how it appears in property hints.
///
/// Used inside [`GodotShape::Class`].
#[derive(Clone, Debug)]
pub enum ClassHeritage {
    /// A class inheriting from `Node` (uses `NODE_TYPE` hint for `#[export]`).
    Node,

    /// A class inheriting from `Resource` (uses `RESOURCE_TYPE` hint for `#[export]`).
    Resource,

    /// A `DynGd<T, D>` where `T` inherits `Resource`. Stores the concrete implementor `ClassId`s from the `#[godot_dyn]` registry.
    DynResource {
        /// Class IDs of all concrete implementors registered via `#[godot_dyn]` or `AsDyn` for the trait.
        implementors: Vec<ClassId>,
    },

    /// Any other class that doesn't inherit from `Node` or `Resource`. No special hint for `#[export]`.
    Other,
}

impl ClassHeritage {
    /// Returns the `PropertyHint` for `#[export]` context.
    pub fn export_property_hint(&self) -> PropertyHint {
        match self {
            Self::Resource | Self::DynResource { .. } => PropertyHint::RESOURCE_TYPE,
            Self::Node => PropertyHint::NODE_TYPE,
            Self::Other => PropertyHint::NONE,
        }
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

/// A single enumerator entry: display name and ordinal value. Used in [`GodotShape::Enum`].
#[derive(Clone, Debug)]
pub struct Enumerator {
    pub name: CowStr,
    /// Ordinal value. `None` for string-backed enums (hint string omits ordinals: `"Grass,Rock,Water"`).
    pub value: Option<i64>,
}

impl Enumerator {
    /// Creates a new int-backed enumerator.
    pub const fn new_int(name: &'static str, value: i64) -> Self {
        Self {
            name: Cow::Borrowed(name),
            value: Some(value),
        }
    }

    /// Creates a new string-backed enumerator (no ordinal in hint string).
    pub const fn new_string(name: &'static str) -> Self {
        Self {
            name: Cow::Borrowed(name),
            value: None,
        }
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Global helper functions

/// Returns `PropertyHintInfo` for an enum or bitfield: `ENUM`/`FLAGS` hint with formatted hint string.
fn enum_hint_info(
    enumerators: &[Enumerator],
    is_bitfield: bool,
    is_engine_enum: bool,
) -> PropertyHintInfo {
    // For engine enums used as `#[var]`: before Godot 4.7, GDScript registers these with `NONE` hint, relying on
    // `class_name` + `CLASS_IS_ENUM` usage flag. From 4.7 onward, GDScript provides the full `ENUM` hint with enumerator list.
    if is_engine_enum && sys::GdextBuild::before_api("4.7") {
        return PropertyHintInfo::none();
    }

    let hint = if is_bitfield {
        PropertyHint::FLAGS
    } else {
        PropertyHint::ENUM
    };

    PropertyHintInfo {
        hint,
        hint_string: GString::from(&format_hint_string(enumerators)),
    }
}

/// Builds `"Name:0,Name2:1"` or `"Name,Name2"` hint string from enumerators.
fn format_hint_string(enumerators: &[Enumerator]) -> String {
    enumerators
        .iter()
        .map(|e| super::format_hint_entry(&e.name, e.value))
        .collect::<Vec<_>>()
        .join(",")
}

/// Maps a packed array's variant type to its element's variant type, if applicable.
///
/// Returns `Some(element_type)` for packed array variant types (e.g. `PACKED_BYTE_ARRAY` → `INT`,
/// `PACKED_STRING_ARRAY` → `STRING`), or `None` for all other variant types.
#[rustfmt::skip]
pub(crate) fn packed_element_variant_type(packed_vtype: VariantType) -> Option<VariantType> {
    match packed_vtype {
        | VariantType::PACKED_BYTE_ARRAY
        | VariantType::PACKED_INT32_ARRAY
        | VariantType::PACKED_INT64_ARRAY   => Some(VariantType::INT),
        | VariantType::PACKED_FLOAT32_ARRAY
        | VariantType::PACKED_FLOAT64_ARRAY => Some(VariantType::FLOAT),
        | VariantType::PACKED_STRING_ARRAY  => Some(VariantType::STRING),
        | VariantType::PACKED_VECTOR2_ARRAY => Some(VariantType::VECTOR2),
        | VariantType::PACKED_VECTOR3_ARRAY => Some(VariantType::VECTOR3),
        #[cfg(since_api = "4.3")]
        | VariantType::PACKED_VECTOR4_ARRAY => Some(VariantType::VECTOR4),
        | VariantType::PACKED_COLOR_ARRAY   => Some(VariantType::COLOR),
        _                                   => None,
    }
}

/// Joins class IDs into a comma-separated string for use in DynGd property hints.
fn join_class_ids(class_ids: &[ClassId]) -> String {
    class_ids
        .iter()
        .map(|id| id.to_cow_str().to_string())
        .collect::<Vec<_>>()
        .join(",")
}
