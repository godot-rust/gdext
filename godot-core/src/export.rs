/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::builtin::meta::{ClassName, PropertyInfo};
use crate::builtin::{GodotString, StringName};
use crate::engine::global::{PropertyHint, PropertyUsageFlags};
use crate::obj::GodotClass;

use godot_ffi as sys;
use sys::VariantType;

/// Trait implemented for types that can be used as `#[export]` fields. This creates a copy of the
/// value, for some type-specific definition of "copy". For example, `Array` and `Gd` are returned
/// via `Share::share()` instead of copying the actual data.
pub trait Export {
    /// Creates a copy to be returned from a getter.
    fn export(&self) -> Self;

    /// The export info to use for an exported field of this type, if no other export info is specified.
    fn default_export_info() -> ExportInfo;
}

/// Info needed for godot to understand how to export a type to the editor.
#[derive(Clone, Eq, PartialEq, Debug)]
pub struct ExportInfo {
    pub variant_type: VariantType,
    pub hint: PropertyHint,
    pub hint_string: GodotString,
}

impl ExportInfo {
    /// Create a new `ExportInfo` with a property hint of
    /// [`PROPERTY_HINT_NONE`](PropertyHint::PROPERTY_HINT_NONE).
    pub fn with_hint_none(variant_type: VariantType) -> Self {
        Self {
            variant_type,
            hint: PropertyHint::PROPERTY_HINT_NONE,
            hint_string: GodotString::new(),
        }
    }

    /// Create a `PropertyInfo` from this export info, using the given property_name and usage, as well as the class name of `C`.
    pub fn to_property_info<C: GodotClass>(
        self,
        property_name: StringName,
        usage: PropertyUsageFlags,
    ) -> PropertyInfo {
        let Self {
            variant_type,
            hint,
            hint_string,
        } = self;

        PropertyInfo {
            variant_type,
            class_name: ClassName::of::<C>(),
            property_name,
            hint,
            hint_string,
            usage,
        }
    }
}

impl<T: Export> Export for Option<T> {
    fn export(&self) -> Self {
        self.as_ref().map(Export::export)
    }

    fn default_export_info() -> ExportInfo {
        T::default_export_info()
    }
}

/// Trait for types that can be represented as a type string for use with
/// [`PropertyHint::PROPERTY_HINT_TYPE_STRING`].
pub trait TypeStringHint {
    /// Returns the representation of this type as a type string.
    ///
    /// See [`PropertyHint.PROPERTY_HINT_TYPE_STRING`](
    ///     https://docs.godotengine.org/en/stable/classes/class_%40globalscope.html#enum-globalscope-propertyhint
    /// ).
    fn type_string() -> String;
}

mod export_impls {
    use super::*;
    use crate::builtin::meta::VariantMetadata;
    use crate::builtin::*;

    macro_rules! impl_export_by_clone {
        ($Ty:ty => $variant_type:ident) => {
            impl Export for $Ty {
                fn export(&self) -> Self {
                    // If `Self` does not implement `Clone`, this gives a clearer error message
                    // than simply `self.clone()`.
                    Clone::clone(self)
                }

                fn default_export_info() -> ExportInfo {
                    ExportInfo::with_hint_none(Self::variant_type())
                }
            }

            impl TypeStringHint for $Ty {
                fn type_string() -> String {
                    format!("{}:", sys::VariantType::$variant_type as i32)
                }
            }
        };
    }

    impl_export_by_clone!(Aabb => Aabb);
    impl_export_by_clone!(bool => Bool);
    impl_export_by_clone!(Basis => Basis);
    impl_export_by_clone!(Vector2 => Vector2);
    impl_export_by_clone!(Vector3 => Vector3);
    impl_export_by_clone!(Vector4 => Vector4);
    impl_export_by_clone!(Vector2i => Vector2i);
    impl_export_by_clone!(Vector3i => Vector3i);
    impl_export_by_clone!(Quaternion => Quaternion);
    impl_export_by_clone!(Color => Color);
    impl_export_by_clone!(GodotString => String);
    impl_export_by_clone!(StringName => StringName);
    impl_export_by_clone!(NodePath => NodePath);
    impl_export_by_clone!(PackedByteArray => PackedByteArray);
    impl_export_by_clone!(PackedInt32Array => PackedInt32Array);
    impl_export_by_clone!(PackedInt64Array => PackedInt64Array);
    impl_export_by_clone!(PackedFloat32Array => PackedFloat32Array);
    impl_export_by_clone!(PackedFloat64Array => PackedFloat64Array);
    impl_export_by_clone!(PackedStringArray => PackedStringArray);
    impl_export_by_clone!(PackedVector2Array => PackedVector2Array);
    impl_export_by_clone!(PackedVector3Array => PackedVector3Array);
    impl_export_by_clone!(PackedColorArray => PackedColorArray);
    impl_export_by_clone!(Plane => Plane);
    impl_export_by_clone!(Projection => Projection);
    impl_export_by_clone!(Rid => Rid);
    impl_export_by_clone!(Rect2 => Rect2);
    impl_export_by_clone!(Rect2i => Rect2i);
    impl_export_by_clone!(Transform2D => Transform2D);
    impl_export_by_clone!(Transform3D => Transform3D);
    impl_export_by_clone!(f64 => Float);
    impl_export_by_clone!(i64 => Int);

    // Godot uses f64 internally for floats, and if Godot tries to pass an invalid f32 into a rust property
    // then the property will just round the value or become inf.
    impl_export_by_clone!(f32 => Float);

    // Godot uses i64 internally for integers, and if Godot tries to pass an invalid integer into a property
    // accepting one of the below values then rust will panic. In the editor this will appear as the property
    // failing to be set to a value and an error printed in the console. During runtime this will crash the
    // program and print the panic from rust stating that the property cannot store the value.
    impl_export_by_clone!(i32 => Int);
    impl_export_by_clone!(i16 => Int);
    impl_export_by_clone!(i8 => Int);
    impl_export_by_clone!(u32 => Int);
    impl_export_by_clone!(u16 => Int);
    impl_export_by_clone!(u8 => Int);

    // Callables can be exported, however you can't do anything with them in the editor.
    // But we do need to be able to export them since we can't make something a property without exporting.
    // And it should be possible to access Callables by property from for instance GDScript.
    // TODO:
    // Remove export impl when we can create properties without exporting them.
    impl_export_by_clone!(Callable => Callable);

    // impl_export_by_clone!(Signal => Signal);
}
