/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! Static type descriptors for Rust types towards Godot.
//!
//! The symbols in this module and primarily [`GodotShape`] are used to describe the _shape_ of a Rust type, which combines information about:
//! - To which Godot type it maps.
//! - Metadata relevant for properties (var/export).
//! - Metadata relevant for method signatures (parameters and return values).
//!
//! These shapes are then transformed to lower-level GDExtension descriptors, located in [`register::info`][crate::registry::info].
//! Godot then accepts those for registration.

use std::borrow::Cow;
use std::fmt::Display;

use godot_ffi as sys;

use crate::builtin::{CowStr, GString, StringName, VariantType};
use crate::global::godot_str;
use crate::meta::{ClassId, GodotConvert};
use crate::obj::EngineEnum as _;
use crate::registry::info::{PropertyHint, PropertyHintInfo, PropertyInfo, PropertyUsageFlags};

/// The "shape" of a Godot type: whether it's a builtin, a class, an enum/bitfield, etc.
///
/// Describes a _static_ (compile-time) type as it should be registered with Godot; returned by [`GodotConvert::godot_shape()`].
/// This is distinct from runtime introspection APIs such [`AnyArray::element_type()`].
///
/// Usually you need to deal with `GodotShape` only if you define custom types through manual `GodotConvert` impls.
///
/// # Information provided by the shape
/// A shape description is used for three purposes:
/// - Property registrations (`#[var]`) so that Godot has static type information of your type.
///   - See [`to_var_property()`] and [`var_hint()`].
/// - Exported properties (`#[export]`) so that properties show up correctly in the editor's inspector UI.
///   - See [`to_export_property()`] and [`export_hint()`].
/// - Method signatures (`#[func]`), so that Godot has the static type information for parameters and return values.
///   - See [`to_method_signature_property()`].
///
/// # Property registration
/// During registration of class properties, the runtime resolves hints and usage flags from the shape:
///
/// - For `#[var]`, it calls [`var_hint()`] and uses `NONE` as base usage.
/// - For `#[export]`, it calls [`export_hint()`] and uses `DEFAULT` as base usage.
/// - If the user specifies explicit overrides (e.g. `#[var(hint = ...)]` or `#[export(range = ...)]`), those replace hints from the shape.
///
/// The shape also contributes structural metadata -- variant type, class name, and additional usage flags (via
/// [`usage_flags()`]). These are combined with the hint and base usage into a [`PropertyInfo`] for the Godot FFI call.
///
/// [`PropertyInfo`]: PropertyInfo
/// [`GodotConvert::godot_shape()`]: GodotConvert::godot_shape
/// [`AnyArray::element_type()`]: crate::builtin::AnyArray::element_type
/// [`to_var_property()`]: Self::to_var_property
/// [`to_export_property()`]: Self::to_export_property
/// [`to_method_signature_property()`]: Self::to_method_signature_property
/// [`var_hint()`]: Self::var_hint
/// [`export_hint()`]: Self::export_hint
/// [`usage_flags()`]: Self::usage_flags
#[non_exhaustive]
#[derive(Clone, Debug)]
pub enum GodotShape {
    /// The general [`Variant`][crate::builtin::Variant] type. Can hold any Godot value.
    ///
    /// Distinct from `Builtin { variant_type: NIL }`, which represents the `()` unit type (`void` in GDScript).
    Variant,

    /// A built-in Godot type (`int`, `String`, `Vector3`, `PackedByteArray`, etc.).
    ///
    /// The variant type used here must match the one from [`GodotConvert::Via`].
    ///
    /// Packed arrays, untyped arrays and untyped dictionaries are also represented as `Builtin`.  \
    /// Typed arrays, typed dictionaries, objects and variants have their own shape representation.
    Builtin {
        /// Godot variant type (e.g. `INT`, `FLOAT`, `STRING`, `VECTOR3`, `PACKED_BYTE_ARRAY`). Never `OBJECT`.
        variant_type: VariantType,
    },

    /// A Godot object type (`Gd<T>`, `DynGd<T, D>`, `Option<Gd<T>>`, `OnReady<Gd<T>>`, etc.).
    ///
    /// Always has `VariantType::OBJECT`.
    Class {
        /// The Godot class of this object type (e.g. `Node`, `Resource`).
        class_id: ClassId,

        /// Whether this inherits from `Resource`, `Node`, or other object class.
        heritage: ClassHeritage,
    },

    /// An [`Array<T>`][crate::builtin::Array] where `T` is not `Variant`.
    ///
    /// Untyped arrays are represented as `Builtin { variant_type: VariantType::ARRAY }`.
    TypedArray {
        /// Shape of the array element type.
        element: GodotElementShape,
    },

    /// A [`Dictionary<K, V>`][crate::builtin::Dictionary] where at least one of `K`, `V` is not `Variant` (Godot 4.4+).
    ///
    /// Untyped dictionaries are represented as `Builtin { variant_type: VariantType::DICTIONARY }`.
    TypedDictionary {
        /// Shape of the dictionary key type.
        key: GodotElementShape,

        /// Shape of the dictionary value type.
        value: GodotElementShape,
    },

    /// An enum or bitfield type (engine-defined or user-defined).
    Enum {
        /// Godot variant type of the underlying representation (typically `INT` for int-backed enums, `STRING` for string-backed).
        variant_type: VariantType,

        /// Display name and ordinal for each enumerator. `Borrowed` for compile-time data, `Owned` for dynamic enumerators.
        enumerators: Cow<'static, [EnumeratorShape]>,

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

    /// Fully custom property metadata. Use only when the type doesn't fit any predefined shape
    Custom {
        /// Godot variant type.
        variant_type: VariantType,

        /// Property hint info for `#[var]` context.
        var_hint: PropertyHintInfo,

        /// Property hint info for `#[export]` context.
        export_hint: PropertyHintInfo,

        /// Stored as `CowStr`; converted to `ClassId` only at registration time to avoid eager global cache allocation.
        class_name: Option<CowStr>,

        /// Additional usage flags.
        ///
        /// These are bit-ORed with the base usage, see [`to_var_property()`][Self::to_var_property] and
        /// [`to_export_property()`][Self::to_export_property].
        ///
        /// Typically you can use `NONE` if the shape doesn't need extra flags.
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

            Self::Custom { var_hint, .. } => var_hint.clone(),

            Self::TypedArray {
                element: element_shape,
            } => PropertyHintInfo {
                hint: PropertyHint::ARRAY_TYPE,
                hint_string: GString::from(&element_shape.element_godot_type_name()),
            },

            Self::TypedDictionary {
                key: key_shape,
                value: value_shape,
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
                            hint_string: GString::from(&format_elements_untyped(elem_vtype)),
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

            Self::TypedArray {
                element: element_shape,
            } => PropertyHintInfo {
                hint: PropertyHint::TYPE_STRING,
                hint_string: GString::from(&element_shape.element_type_string()),
            },

            Self::TypedDictionary {
                key: key_shape,
                value: value_shape,
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

            Self::Custom { export_hint, .. } => export_hint.clone(),
        }
    }

    /// Converts `godot_name`/`class_name` to a `StringName`. Only called during registration.
    ///
    /// For engine enums, returns the enum's qualified name (e.g. `"Node.ProcessMode"`).
    /// For classes, returns the class name.
    /// For other types, returns an empty `StringName`.
    pub(crate) fn class_name_or_none(&self) -> StringName {
        match self {
            Self::Variant
            | Self::Builtin { .. }
            | Self::TypedArray { .. }
            | Self::TypedDictionary { .. } => StringName::default(),
            Self::Class { class_id, .. } => class_id.to_string_name(),
            Self::Enum { godot_name, .. } => match godot_name {
                Some(name) => StringName::from(name.as_ref()),
                None => StringName::default(),
            },
            Self::Custom { class_name, .. } => match class_name {
                Some(name) => StringName::from(name.as_ref()),
                None => StringName::default(),
            },
        }
    }

    /// Shape-specific usage flags for property registration.
    ///
    /// These are combined with the base usage, which is `NONE` for `#[var]` and `DEFAULT` for `#[export]`.
    // Only engine enums (those with `godot_name`) get `CLASS_IS_ENUM`. User enums don't set this flag: `CLASS_IS_ENUM` tells Godot to resolve
    // the enum's enumerators via `class_name` in ClassDB -- but user enums aren't registered there yet. Setting the flag without a valid
    // `class_name` would cause Godot to look up a nonexistent name. Once we call `classdb_register_extension_class_integer_constant` for user
    // enums (making them visible to GDScript by name), they can set `godot_name` and get `CLASS_IS_ENUM` automatically.
    pub fn usage_flags(&self) -> PropertyUsageFlags {
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

    /// Builds the low-level Godot property info for method parameter/return type registration.
    ///
    /// Uses:
    /// - Hint and hint string: [`var_hint()`][Self::var_hint].
    /// - Usage: [`DEFAULT`][PropertyUsageFlags::DEFAULT] (required for method params; not shown in editor).
    pub fn to_method_signature_property(&self, property_name: &str) -> PropertyInfo {
        let hint_info = self.var_hint();
        self.to_property(property_name, hint_info, PropertyUsageFlags::DEFAULT)
    }

    /// Builds the low-level Godot property info for `#[var]` context.
    ///
    /// Uses:
    /// - Hint and hint string: [`var_hint()`][Self::var_hint].
    /// - Base usage: [`NONE`][PropertyUsageFlags::NONE], combined with specific [`usage_flags()`][Self::usage_flags] from this shape.
    ///   Property is accessible from GDScript, but not shown in editor or saved.
    ///
    /// See also [`PropertyInfo::new_var()`].
    pub fn to_var_property(&self, property_name: &str) -> PropertyInfo {
        let hint_info = self.var_hint();
        self.to_property(property_name, hint_info, PropertyUsageFlags::NONE)
    }

    /// Builds the low-level Godot property info for `#[export]` context.
    ///
    /// Uses:
    /// - Hint and hint string: [`export_hint()`][Self::export_hint].
    /// - Usage: [`DEFAULT`][PropertyUsageFlags::DEFAULT], combined with specific [`usage_flags()`][Self::usage_flags] from this shape.
    ///   Property is shown in editor and saved.
    ///
    /// See also [`PropertyInfo::new_export()`].
    pub fn to_export_property(&self, property_name: &str) -> PropertyInfo {
        let hint_info = self.export_hint();
        self.to_property(property_name, hint_info, PropertyUsageFlags::DEFAULT)
    }

    /// Low-level builder for [`PropertyInfo`]. Derives `class_name`, `variant_type`, and shape-specific usage flags from
    /// `self`, but takes `hint_info` and `base_usage` as parameters because they depend on context (`#[var]` vs
    /// `#[export]`) and may be overridden by the user (e.g. `#[var(hint = ..., hint_string = "...")]`).
    ///
    /// Prefer [`to_var_property()`](Self::to_var_property) or [`to_export_property()`](Self::to_export_property)
    /// when no user override is involved.
    fn to_property(
        &self,
        property_name: &str,
        hint_info: PropertyHintInfo,
        base_usage: PropertyUsageFlags,
    ) -> PropertyInfo {
        use crate::builtin::StringName;

        let class_name = self.class_name_or_none();
        let variant_type = self.variant_type();
        let usage = base_usage | self.usage_flags();

        PropertyInfo {
            variant_type,
            class_name,
            property_name: StringName::from(property_name),
            hint_info,
            usage,
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
    /// A class inheriting from [`Node`][crate::classes::Node] (uses `NODE_TYPE` hint for `#[export]`).
    Node,

    /// A class inheriting from [`Resource`][crate::classes::Resource] (uses `RESOURCE_TYPE` hint for `#[export]`).
    Resource,

    /// A `DynGd<T, D>` where `T` inherits `Resource`. Stores the concrete implementor `ClassId`s from the `#[godot_dyn]` registry.
    DynResource {
        /// Class IDs of all concrete implementors registered via `#[godot_dyn]` or [`AsDyn`][crate::obj::AsDyn] for the trait.
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
// Inner/nested shape

/// Same as [`GodotShape`], but for element types nested in typed arrays/dictionaries.
///
/// Matches the same layout as `GodotShape`, exists to avoid recursive definition (and also `Box` allocations). Also constrains the possible
/// shapes (elements cannot be typed arrays/dictionaries themselves).
///
/// In contrast to [`ElementType`][crate::meta::inspect::ElementType], this is a _static_ type description for Godot registration purposes.
///
/// Use [`into_outer()`][Self::into_outer] to convert into a full `GodotShape`.
#[non_exhaustive]
#[derive(Clone, Debug)]
pub enum GodotElementShape {
    // Inner types are not structs like Builtin(BuiltinShape), to avoid type proliferation in niche APIs. to_outer() is good enough.
    Variant,

    Builtin {
        variant_type: VariantType,
    },

    Class {
        class_id: ClassId,
        heritage: ClassHeritage,
    },

    Enum {
        variant_type: VariantType,
        enumerators: Cow<'static, [EnumeratorShape]>,
        godot_name: Option<CowStr>,
        is_bitfield: bool,
    },

    Custom {
        variant_type: VariantType,
        var_hint: PropertyHintInfo,
        export_hint: PropertyHintInfo,
        class_name: Option<CowStr>,
        usage_flags: PropertyUsageFlags,
    },
}

impl GodotElementShape {
    #[rustfmt::skip]
    pub(crate) fn new(outer: GodotShape) -> Self {
        type GShape = GodotShape;

        match outer {
             GShape::Variant
            => Self::Variant,

            GShape::Builtin { variant_type }
            => Self::Builtin { variant_type },

             GShape::Class { class_id, heritage }
            => Self::Class { class_id, heritage },

             GShape::Enum { variant_type, enumerators, godot_name, is_bitfield }
            => Self::Enum { variant_type, enumerators, godot_name, is_bitfield },

             GShape::Custom { variant_type, var_hint, export_hint, class_name, usage_flags}
            => Self::Custom { variant_type, var_hint, export_hint, class_name, usage_flags },

            GShape::TypedArray { .. } |
            GShape::TypedDictionary { .. } => panic!("nested shapes cannot be typed arrays/dictionaries")
        }
    }

    /// Converts this nested shape into a full `GodotShape`. Infallible.
    #[rustfmt::skip]
    pub fn into_outer(self) -> GodotShape {
        type G = GodotShape;

        match self {
            Self::Variant
            => G::Variant,

            Self::Builtin { variant_type }
            => G::Builtin { variant_type },

            Self::Class { class_id, heritage }
            => G::Class { class_id, heritage },

            Self::Enum { variant_type, enumerators, godot_name, is_bitfield }
            => G::Enum { variant_type, enumerators, godot_name, is_bitfield },

            Self::Custom { variant_type, var_hint, export_hint, class_name, usage_flags}
            => G::Custom { variant_type, var_hint, export_hint, class_name, usage_flags },
        }
    }

    /// Returns the Godot type name for use in `#[var]` array/dictionary type hints.
    ///
    /// Defaults to the `Via` type's name (e.g. `"int"` for `i32`). Engine enums override this to return their qualified class name
    /// (e.g. `"Node.ProcessMode"`).
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
        }
    }

    /// Returns the representation of this type as an element inside an array, e.g. `"4:"` for string, or `"24:34/MyClass"` for objects.
    ///
    /// `4` and `24` are variant type ords; `34` is `PropertyHint::NODE_TYPE` ord.
    ///
    /// See [`PropertyHint::TYPE_STRING`] and
    /// [upstream docs](https://docs.godotengine.org/en/stable/classes/class_%40globalscope.html#enum-globalscope-propertyhint).
    pub(crate) fn element_type_string(&self) -> String {
        match self {
            Self::Variant
            | Self::Builtin {
                variant_type: VariantType::NIL,
            } => {
                // Variant (or void) as element: untyped, no hint.
                format_elements_untyped(VariantType::NIL)
            }

            Self::Builtin { variant_type } => {
                if sys::GdextBuild::since_api("4.3") {
                    format_elements_untyped(*variant_type)
                } else {
                    format!("{}:{}", variant_type.ord(), variant_type.godot_type_name())
                }
            }

            Self::Class { class_id, heritage } => {
                let export_hint = heritage.export_property_hint();
                sys::strict_assert_ne!(
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
                format_elements_typed(VariantType::OBJECT, export_hint, &hint_string)
            }

            Self::Enum { .. } => {
                let outer = self.clone().into_outer(); // slightly expensive
                let variant_type = outer.variant_type();
                let info = outer.export_hint();
                format_elements_typed(variant_type, info.hint, &info.hint_string)
            }

            Self::Custom {
                variant_type,
                var_hint,
                ..
            } => format_elements_typed(*variant_type, var_hint.hint, &var_hint.hint_string),
        }
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

/// Describes a single enumerator entry: display name and ordinal value. Used in [`GodotShape::Enum`].
#[derive(Clone, Debug)]
pub struct EnumeratorShape {
    pub name: CowStr,
    /// Ordinal value. `None` for string-backed enums (hint string omits ordinals: `"Grass,Rock,Water"`).
    pub value: Option<i64>,
}

impl EnumeratorShape {
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

/// Builds the element type string used in Godot's `TYPE_STRING` hint for typed collections.
///
/// Format: `"{vtype}:"` if `hint` is `NONE`, otherwise `"{vtype}/{hint}:{hint_string}"`.
pub(crate) fn format_elements_typed(
    variant_type: VariantType,
    hint: PropertyHint,
    hint_string: impl std::fmt::Display,
) -> String {
    if hint == PropertyHint::NONE {
        format!("{}:", variant_type.ord())
    } else {
        format!("{}/{}:{}", variant_type.ord(), hint.ord(), hint_string)
    }
}

/// Formats the element type string for untyped collections (e.g. `Array` without `TYPE_STRING` hint), which only includes the variant type.
///
/// Format: `"{vtype}:"`.
fn format_elements_untyped(variant_type: VariantType) -> String {
    format!("{}:", variant_type.ord())
}

/// Returns `PropertyHintInfo` for an enum or bitfield: `ENUM`/`FLAGS` hint with formatted hint string.
fn enum_hint_info(
    enumerators: &[EnumeratorShape],
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
fn format_hint_string(enumerators: &[EnumeratorShape]) -> String {
    enumerators
        .iter()
        .map(|e| format_hint_entry(&e.name, e.value))
        .collect::<Vec<_>>()
        .join(",")
}

/// Formats a single hint as `"Name:value"` if value is `Some`, otherwise `"Name"`.
pub(crate) fn format_hint_entry(name: &str, value: Option<impl Display>) -> String {
    match value {
        Some(v) => format!("{name}:{v}"),
        None => name.to_string(),
    }
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
