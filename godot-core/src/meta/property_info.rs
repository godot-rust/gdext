/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::builtin::{GString, StringName, VariantType};
use crate::global::{PropertyHint, PropertyUsageFlags};
use crate::meta::{ClassId, Element};
use crate::obj::{Bounds, EngineBitfield, EngineEnum, GodotClass, bounds};
use crate::registry::property::{Export, Var};
use crate::{classes, sys};

/// Describes a property's type, name and metadata for Godot.
///
/// `PropertyInfo` is used throughout the Godot binding to describe properties, method parameters and return types.
///
/// This the Rust representation of the FFI type `sys::GDExtensionPropertyInfo`, still relatively low level. Unlike the FFI version which
/// only stores pointers, `PropertyInfo` owns its data, ensuring it remains valid for the lifetime of the struct.
/// For a high-level representation of properties, see [`GodotShape`][crate::registry::property::GodotShape].
///
/// See also [`MethodInfo`](crate::meta::MethodInfo) for describing method signatures and [`ClassId`] for type-IDs of Godot classes.
///
/// # Construction
/// For most use cases, prefer the convenience constructors:
/// - [`new_var::<T>()`](Self::new_var) -- creates property info for a `#[var]` attribute.
/// - [`new_export::<T>()`](Self::new_export) -- for an `#[export]` attribute.
/// - [`new_group()`](Self::new_group) / [`new_subgroup()`](Self::new_subgroup) -- for editor groups.
///
/// # Example
/// ```no_run
/// use godot::meta::{PropertyInfo, PropertyHintInfo};
/// use godot::builtin::{StringName, VariantType};
/// use godot::global::PropertyUsageFlags;
///
/// // Integer property without a specific class
/// let count_property = PropertyInfo {
///     variant_type: VariantType::INT,
///     class_name: StringName::default(),  // Only OBJECT types and enums need a real class/enum name.
///     property_name: StringName::from("count"),
///     hint_info: PropertyHintInfo::none(),
///     usage: PropertyUsageFlags::DEFAULT,
/// };
/// ```
///
/// For OBJECT types, `class_name` should be set to the class name (e.g., `"Node3D"`). You can use [`GodotClass::class_id()`] +
/// [`ClassId::to_string_name()`] to keep this type-safe across class renames.
///
/// If the property refers to an enum, `class_name` should be set to the enum name (e.g., `"Node.ProcessMode"` or `"GlobalEnum"`).
/// User-defined enums can also be registered with the empty string (they're a loose list of enumerators in that case).
#[derive(Clone, Debug)]
// Note: is not #[non_exhaustive], so adding fields is a breaking change. Mostly used internally at the moment though.
// Note: There was an idea of a high-level representation of the following, but it's likely easier and more efficient to use introspection
// APIs like `is_array_of_elem()`, unless there's a real user-facing need.
// pub(crate) enum SimplePropertyType {
//     Variant { ty: VariantType },
//     Array { elem_ty: VariantType },
//     Object { class_id: ClassId },
// }
pub struct PropertyInfo {
    /// Type of the property.
    ///
    /// For objects, this should be set to [`VariantType::OBJECT`] and use the `class_name` field to specify the actual class.  \
    /// For enums, this should be set to [`VariantType::INT`] and use the `class_name` field to specify the enum name.  \
    /// For generic [`Variant`](crate::builtin::Variant) properties, use [`VariantType::NIL`].
    pub variant_type: VariantType,

    /// The specific class or enum name for object-typed and enum properties in Godot.
    ///
    /// Assign the following value:
    /// - For objects (`variant_type == OBJECT`), this should be set to the class name (e.g., `"Node3D"`, `"RefCounted"`).
    /// - For enums (commonly `variant_type == INT`), this should be set to the enum name (e.g., `"Node.ProcessMode"` or `"GlobalEnum"`).
    ///   Rust-side enums that aren't registered with Godot can also use the empty string -- in that case it's a loose list of enumerators.
    /// - For other types, this should be left empty, i.e. `StringName::default()`.
    ///
    /// # Example
    /// ```no_run
    /// use godot::builtin::StringName;
    /// use godot::classes::Node3D;
    /// use godot::meta::ClassId;
    /// use godot::obj::GodotClass; // Trait method ::class_id().
    ///
    /// let none_id = ClassId::none();                      // For built-ins (not classes).
    /// let static_id = Node3D::class_id();                 // For classes with a Rust type.
    /// let dynamic_id = ClassId::new_dynamic("MyScript");  // For runtime class names.
    ///
    /// // Convert to StringName for this field:
    /// let class_name = static_id.to_string_name();
    ///
    /// // Or directly, without caching the class name globally (recommended anyway for enums):
    /// let class_name = StringName::from("MyScript");
    /// ```
    pub class_name: StringName,

    /// The name of this property as it appears in Godot's object system.
    pub property_name: StringName,

    /// Additional type information and validation constraints for this property.
    ///
    /// Use functions from [`export_info_functions`](crate::registry::property::export_info_functions) to create common hints,
    /// or [`PropertyHintInfo::none()`] for no hints.
    ///
    /// See [`PropertyHintInfo`] struct in Rust, as well as [`PropertyHint`] in the official Godot documentation.
    ///
    /// [`PropertyHint`]: https://docs.godotengine.org/en/latest/classes/class_%40globalscope.html#enum-globalscope-propertyhint
    pub hint_info: PropertyHintInfo,

    /// Flags controlling how this property should be used and displayed by the Godot engine.
    ///
    /// Common values:
    /// - [`PropertyUsageFlags::DEFAULT`] -- standard property (readable, writable, saved, appears in editor).
    /// - [`PropertyUsageFlags::STORAGE`] -- persisted, but not shown in editor.
    /// - [`PropertyUsageFlags::EDITOR`] -- shown in editor, but not persisted.
    ///
    /// See also [`PropertyUsageFlags`] in the official Godot documentation for a complete list of flags.
    ///
    /// [`PropertyUsageFlags`]: https://docs.godotengine.org/en/latest/classes/class_%40globalscope.html#enum-globalscope-propertyusageflags
    pub usage: PropertyUsageFlags,
}

impl PropertyInfo {
    /// Create a new `PropertyInfo` representing a property named `property_name` with type `T` automatically.
    ///
    /// This will generate property info equivalent to what a `#[var]` attribute would produce: the property is accessible
    /// from GDScript but **not** shown in the editor and **not** saved. Uses [`PropertyUsageFlags::NONE`] as base usage.
    ///
    /// For editor-visible + saved properties, use [`new_export()`](Self::new_export).
    pub fn new_var<T: Var>(property_name: &str) -> Self {
        T::godot_shape().to_var_property(property_name)
    }

    /// Create a new `PropertyInfo` for an exported property named `property_name` with type `T` automatically.
    ///
    /// This will generate property info equivalent to what an `#[export]` attribute would produce: the property is shown
    /// in the editor and saved. Uses [`PropertyUsageFlags::DEFAULT`] as base usage.
    pub fn new_export<T: Export>(property_name: &str) -> Self {
        T::godot_shape().to_export_property(property_name)
    }

    /// Change the `hint` and `hint_string` to be the given `hint_info`.
    ///
    /// See [`export_info_functions`](crate::registry::property::export_info_functions) for functions that return appropriate `PropertyHintInfo`s for
    /// various Godot annotations.
    ///
    /// # Example
    /// Creating an `@export_range` property.
    ///
    // TODO: Make this nicer to use.
    /// ```no_run
    /// use godot::register::property::export_info_functions;
    /// use godot::meta::PropertyInfo;
    ///
    /// let property = PropertyInfo::new_export::<f64>("my_range_property")
    ///     .with_hint_info(export_info_functions::export_range(
    ///         0.0,
    ///         10.0,
    ///         Some(0.1),
    ///         false,
    ///         false,
    ///         false,
    ///         false,
    ///         false,
    ///         false,
    ///         Some("mm".to_string()),
    ///     ));
    /// ```
    pub fn with_hint_info(self, hint_info: PropertyHintInfo) -> Self {
        Self { hint_info, ..self }
    }

    /// Create a new `PropertyInfo` representing a group in Godot.
    ///
    /// See [`EditorInspector`](https://docs.godotengine.org/en/latest/classes/class_editorinspector.html#class-editorinspector) in Godot for
    /// more information.
    pub fn new_group(group_name: &str, group_prefix: &str) -> Self {
        Self {
            variant_type: VariantType::NIL,
            class_name: StringName::default(),
            property_name: group_name.into(),
            hint_info: PropertyHintInfo {
                hint: PropertyHint::NONE,
                hint_string: group_prefix.into(),
            },
            usage: PropertyUsageFlags::GROUP,
        }
    }

    /// Create a new `PropertyInfo` representing a subgroup in Godot.
    ///
    /// See [`EditorInspector`](https://docs.godotengine.org/en/latest/classes/class_editorinspector.html#class-editorinspector) in Godot for
    /// more information.
    pub fn new_subgroup(subgroup_name: &str, subgroup_prefix: &str) -> Self {
        Self {
            variant_type: VariantType::NIL,
            class_name: StringName::default(),
            property_name: subgroup_name.into(),
            hint_info: PropertyHintInfo {
                hint: PropertyHint::NONE,
                hint_string: subgroup_prefix.into(),
            },
            usage: PropertyUsageFlags::SUBGROUP,
        }
    }

    // ------------------------------------------------------------------------------------------------------------------------------------------
    // Introspection API -- could be made public in the future

    pub(crate) fn is_array_of_elem<T>(&self) -> bool
    where
        T: Element,
    {
        self.variant_type == VariantType::ARRAY
            && self.hint_info.hint == PropertyHint::ARRAY_TYPE
            && self.hint_info.hint_string == T::godot_shape().godot_type_name().as_ref()
    }

    // ------------------------------------------------------------------------------------------------------------------------------------------
    // FFI conversion functions

    /// Converts to the FFI type. Keep this object allocated while using that!
    #[doc(hidden)]
    pub fn property_sys(&self) -> sys::GDExtensionPropertyInfo {
        use crate::obj::{EngineBitfield as _, EngineEnum as _};

        sys::GDExtensionPropertyInfo {
            type_: self.variant_type.sys(),
            name: sys::SysPtr::force_mut(self.property_name.string_sys()),
            class_name: sys::SysPtr::force_mut(self.class_name.string_sys()),
            hint: u32::try_from(self.hint_info.hint.ord()).expect("hint.ord()"),
            hint_string: sys::SysPtr::force_mut(self.hint_info.hint_string.string_sys()),
            usage: u32::try_from(self.usage.ord()).expect("usage.ord()"),
        }
    }

    #[doc(hidden)]
    pub fn empty_sys() -> sys::GDExtensionPropertyInfo {
        use crate::obj::{EngineBitfield as _, EngineEnum as _};

        sys::GDExtensionPropertyInfo {
            type_: VariantType::NIL.sys(),
            name: std::ptr::null_mut(),
            class_name: std::ptr::null_mut(),
            hint: PropertyHint::NONE.ord() as u32,
            hint_string: std::ptr::null_mut(),
            usage: PropertyUsageFlags::NONE.ord() as u32,
        }
    }

    /// Consumes self and turns it into a `sys::GDExtensionPropertyInfo`, should be used together with
    /// [`free_owned_property_sys`](Self::free_owned_property_sys).
    ///
    /// This will leak memory unless used together with `free_owned_property_sys`.
    pub(crate) fn into_owned_property_sys(self) -> sys::GDExtensionPropertyInfo {
        use crate::obj::{EngineBitfield as _, EngineEnum as _};

        sys::GDExtensionPropertyInfo {
            type_: self.variant_type.sys(),
            name: self.property_name.into_owned_string_sys(),
            class_name: self.class_name.into_owned_string_sys(),
            hint: u32::try_from(self.hint_info.hint.ord()).expect("hint.ord()"),
            hint_string: self.hint_info.hint_string.into_owned_string_sys(),
            usage: u32::try_from(self.usage.ord()).expect("usage.ord()"),
        }
    }

    /// Properly frees a `sys::GDExtensionPropertyInfo` created by [`into_owned_property_sys`](Self::into_owned_property_sys).
    ///
    /// # Safety
    ///
    /// * Must only be used on a struct returned from a call to `into_owned_property_sys`, without modification.
    /// * Must not be called more than once on a `sys::GDExtensionPropertyInfo` struct.
    pub(crate) unsafe fn free_owned_property_sys(info: sys::GDExtensionPropertyInfo) {
        // SAFETY: This function was called on a pointer returned from `into_owned_property_sys`, thus both `info.name` and
        // `info.hint_string` were created from calls to `into_owned_string_sys` on their respective types.
        // Additionally, this function isn't called more than once on a struct containing the same `name` or `hint_string` pointers.
        unsafe {
            let _name = StringName::from_owned_string_sys(info.name);
            let _class_name = StringName::from_owned_string_sys(info.class_name);
            let _hint_string = GString::from_owned_string_sys(info.hint_string);
        }
    }

    /// Moves its values into given `GDExtensionPropertyInfo`, dropping previous values if necessary.
    ///
    /// # Safety
    ///
    /// * `property_info_ptr` must be valid.
    ///
    pub(crate) unsafe fn move_into_property_info_ptr(
        self,
        property_info_ptr: *mut sys::GDExtensionPropertyInfo,
    ) {
        unsafe {
            let ptr = &mut *property_info_ptr;

            ptr.usage = u32::try_from(self.usage.ord()).expect("usage.ord()");
            ptr.hint = u32::try_from(self.hint_info.hint.ord()).expect("hint.ord()");
            ptr.type_ = self.variant_type.sys();

            *StringName::borrow_string_sys_mut(ptr.name) = self.property_name;
            *GString::borrow_string_sys_mut(ptr.hint_string) = self.hint_info.hint_string;
            *StringName::borrow_string_sys_mut(ptr.class_name) = self.class_name;
        }
    }

    /// Creates copy of given `sys::GDExtensionPropertyInfo`.
    ///
    /// # Safety
    ///
    /// * `property_info_ptr` must be valid.
    pub(crate) unsafe fn new_from_sys(
        property_info_ptr: *mut sys::GDExtensionPropertyInfo,
    ) -> Self {
        unsafe {
            let ptr = *property_info_ptr;

            Self {
                variant_type: VariantType::from_sys(ptr.type_),
                class_name: StringName::new_from_string_sys(ptr.class_name),
                property_name: StringName::new_from_string_sys(ptr.name),
                hint_info: PropertyHintInfo {
                    hint: PropertyHint::from_ord(ptr.hint.to_owned() as i32),
                    hint_string: GString::new_from_string_sys(ptr.hint_string),
                },
                usage: PropertyUsageFlags::from_ord(ptr.usage as u64),
            }
        }
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

/// Info needed by Godot, for how to export a type to the editor.
///
/// Property hints provide extra metadata about the property, such as:
/// - Range constraints for numeric values.
/// - Enum value lists.
/// - File/directory paths.
/// - Resource types.
/// - Array element types.
#[derive(Clone, Eq, PartialEq, Debug)]
pub struct PropertyHintInfo {
    pub hint: PropertyHint,
    pub hint_string: GString,
}

impl PropertyHintInfo {
    /// Create a new `PropertyHintInfo` with a property hint of [`PROPERTY_HINT_NONE`](PropertyHint::NONE), and no hint string.
    pub fn none() -> Self {
        Self {
            hint: PropertyHint::NONE,
            hint_string: GString::new(),
        }
    }

    #[doc(hidden)]
    pub fn object_as_node_class<T>() -> Option<ClassId>
    where
        T: GodotClass + Bounds<Exportable = bounds::Yes>,
    {
        T::inherits::<classes::Node>().then(|| T::class_id())
    }
}
