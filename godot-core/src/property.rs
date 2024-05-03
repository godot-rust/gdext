/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! Registration support for property types.

use crate::builtin::meta::{FromGodot, GodotConvert, ToGodot};
use crate::builtin::GString;
use crate::engine::global::PropertyHint;

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Trait definitions

/// Trait implemented for types that can be used as `#[var]` fields.
///
/// This creates a copy of the value, according to copy semantics provided by `Clone`. For example, `Array`, `Dictionary` and `Gd` are
/// returned by shared reference instead of copying the actual data.
///
/// This does not require [`FromGodot`] or [`ToGodot`], so that something can be used as a property even if it can't be used in function
/// arguments/return types.
pub trait Var: GodotConvert {
    fn get_property(&self) -> Self::Via;
    fn set_property(&mut self, value: Self::Via);

    fn property_hint() -> PropertyHintInfo {
        PropertyHintInfo::with_hint_none("")
    }
}

/// Trait implemented for types that can be used as `#[export]` fields.
pub trait Export: Var {
    /// The export info to use for an exported field of this type, if no other export info is specified.
    fn default_export_info() -> PropertyHintInfo;
}

/// Marks types that are registered via "type string hint" in Godot.
///
/// See [`PropertyHint::TYPE_STRING`] and [upstream docs].
///
/// [upstream docs]: https://docs.godotengine.org/en/stable/classes/class_%40globalscope.html#enum-globalscope-propertyhint
pub trait TypeStringHint {
    /// Returns the representation of this type as a type string.
    ///
    /// See [`PropertyHint.PROPERTY_HINT_TYPE_STRING`](
    ///     https://docs.godotengine.org/en/stable/classes/class_%40globalscope.html#enum-globalscope-propertyhint
    /// ).
    fn type_string() -> String;
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Blanket impls for Option<T>

impl<T: TypeStringHint> TypeStringHint for Option<T> {
    fn type_string() -> String {
        T::type_string()
    }
}

impl<T> Var for Option<T>
where
    T: Var + FromGodot,
    Option<T>: GodotConvert<Via = Option<T::Via>>,
{
    fn get_property(&self) -> Self::Via {
        self.as_ref().map(Var::get_property)
    }

    fn set_property(&mut self, value: Self::Via) {
        match value {
            Some(value) => {
                if let Some(current_value) = self {
                    current_value.set_property(value)
                } else {
                    *self = Some(FromGodot::from_godot(value))
                }
            }
            None => *self = None,
        }
    }
}

impl<T> Export for Option<T>
where
    T: Export,
    Option<T>: Var,
{
    fn default_export_info() -> PropertyHintInfo {
        T::default_export_info()
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Export machinery

/// Info needed by Godot, for how to export a type to the editor.
#[derive(Clone, Eq, PartialEq, Debug)]
pub struct PropertyHintInfo {
    pub hint: PropertyHint,
    pub hint_string: GString,
}

impl PropertyHintInfo {
    /// Create a new `PropertyHintInfo` with a property hint of
    /// [`PROPERTY_HINT_NONE`](PropertyHint::NONE).
    ///
    /// Usually Godot expects this to be combined with a `hint_string` containing the name of the type.
    pub fn with_hint_none<S: Into<GString>>(type_name: S) -> Self {
        Self {
            hint: PropertyHint::NONE,
            hint_string: type_name.into(),
        }
    }
}

/// Functions used to translate user-provided arguments into export hints.
pub mod export_info_functions {
    use crate::builtin::GString;
    use crate::engine::global::PropertyHint;

    use super::PropertyHintInfo;

    /// Turn a list of variables into a comma separated string containing only the identifiers corresponding
    /// to a true boolean variable.
    macro_rules! comma_separate_boolean_idents {
        ($( $ident:ident),* $(,)?) => {
            {
                let mut strings = Vec::new();

                $(
                    if $ident {
                        strings.push(stringify!($ident));
                    }
                )*

                strings.join(",")
            }
        };
    }

    // We want this to match the options available on `@export_range(..)`
    #[allow(clippy::too_many_arguments)]
    pub fn export_range(
        min: f64,
        max: f64,
        step: Option<f64>,
        or_greater: bool,
        or_less: bool,
        exp: bool,
        radians: bool,
        degrees: bool,
        hide_slider: bool,
    ) -> PropertyHintInfo {
        let hint_beginning = if let Some(step) = step {
            format!("{min},{max},{step}")
        } else {
            format!("{min},{max}")
        };
        let rest =
            comma_separate_boolean_idents!(or_greater, or_less, exp, radians, degrees, hide_slider);

        let hint_string = if rest.is_empty() {
            hint_beginning
        } else {
            format!("{hint_beginning},{rest}")
        };

        PropertyHintInfo {
            hint: PropertyHint::RANGE,
            hint_string: hint_string.into(),
        }
    }

    pub struct ExportValueWithKey<T> {
        variant: String,
        key: Option<T>,
    }

    impl<T: std::fmt::Display> ExportValueWithKey<T> {
        fn as_hint_string(&self) -> String {
            let Self { variant, key } = self;

            match key {
                Some(key) => format!("{variant}:{key}"),
                None => variant.clone(),
            }
        }

        fn slice_as_hint_string<V>(values: &[V]) -> String
        where
            for<'a> &'a V: Into<Self>,
        {
            let values = values
                .iter()
                .map(|v| v.into().as_hint_string())
                .collect::<Vec<_>>();

            values.join(",")
        }
    }

    impl<T, S> From<&(S, Option<T>)> for ExportValueWithKey<T>
    where
        T: Clone,
        S: AsRef<str>,
    {
        fn from((variant, key): &(S, Option<T>)) -> Self {
            Self {
                variant: variant.as_ref().into(),
                key: key.clone(),
            }
        }
    }

    type EnumVariant = ExportValueWithKey<i64>;

    pub fn export_enum<T>(variants: &[T]) -> PropertyHintInfo
    where
        for<'a> &'a T: Into<EnumVariant>,
    {
        let hint_string: String = EnumVariant::slice_as_hint_string(variants);

        PropertyHintInfo {
            hint: PropertyHint::ENUM,
            hint_string: hint_string.into(),
        }
    }

    pub fn export_exp_easing(attenuation: bool, positive_only: bool) -> PropertyHintInfo {
        let hint_string = comma_separate_boolean_idents!(attenuation, positive_only);

        PropertyHintInfo {
            hint: PropertyHint::EXP_EASING,
            hint_string: hint_string.into(),
        }
    }

    type BitFlag = ExportValueWithKey<u32>;

    pub fn export_flags<T>(bits: &[T]) -> PropertyHintInfo
    where
        for<'a> &'a T: Into<BitFlag>,
    {
        let hint_string = BitFlag::slice_as_hint_string(bits);

        PropertyHintInfo {
            hint: PropertyHint::FLAGS,
            hint_string: hint_string.into(),
        }
    }

    pub fn export_file<S: AsRef<str>>(filter: S) -> PropertyHintInfo {
        export_file_inner(false, filter)
    }

    pub fn export_global_file<S: AsRef<str>>(filter: S) -> PropertyHintInfo {
        export_file_inner(true, filter)
    }

    pub fn export_file_inner<S: AsRef<str>>(global: bool, filter: S) -> PropertyHintInfo {
        let hint = if global {
            PropertyHint::GLOBAL_FILE
        } else {
            PropertyHint::FILE
        };

        PropertyHintInfo {
            hint,
            hint_string: filter.as_ref().into(),
        }
    }

    pub fn export_placeholder<S: AsRef<str>>(placeholder: S) -> PropertyHintInfo {
        PropertyHintInfo {
            hint: PropertyHint::PLACEHOLDER_TEXT,
            hint_string: placeholder.as_ref().into(),
        }
    }

    macro_rules! default_export_funcs {
        (
            $( $function_name:ident => $property_hint:ident, )*
        ) => {
            $(
                pub fn $function_name() -> PropertyHintInfo {
                    PropertyHintInfo {
                        hint: PropertyHint::$property_hint,
                        hint_string: GString::new()
                    }
                }
            )*
        };
    }

    // The left side of these declarations follows the export annotation provided by GDScript, whereas the
    // right side are the corresponding property hint. Godot is not always consistent between the two, such
    // as `export_multiline` being `PROPERTY_HINT_MULTILINE_TEXT`.
    default_export_funcs!(
        export_flags_2d_physics => LAYERS_2D_PHYSICS,
        export_flags_2d_render => LAYERS_2D_RENDER,
        export_flags_2d_navigation => LAYERS_2D_NAVIGATION,
        export_flags_3d_physics => LAYERS_3D_PHYSICS,
        export_flags_3d_render => LAYERS_3D_RENDER,
        export_flags_3d_navigation => LAYERS_3D_NAVIGATION,
        export_dir => DIR,
        export_global_dir => GLOBAL_DIR,
        export_multiline => MULTILINE_TEXT,
        export_color_no_alpha => COLOR_NO_ALPHA,
    );
}

mod export_impls {
    use super::*;
    use crate::builtin::*;
    use godot_ffi as sys;

    macro_rules! impl_property_by_godot_convert {
        ($Ty:ty, no_export) => {
            impl_property_by_godot_convert!(@property $Ty);
            impl_property_by_godot_convert!(@type_string_hint $Ty);
        };

        ($Ty:ty) => {
            impl_property_by_godot_convert!(@property $Ty);
            impl_property_by_godot_convert!(@export $Ty);
            impl_property_by_godot_convert!(@type_string_hint $Ty);
        };

        (@property $Ty:ty) => {
            impl Var for $Ty {
                fn get_property(&self) -> Self::Via {
                    self.to_godot()
                }

                fn set_property(&mut self, value: Self::Via) {
                    *self = FromGodot::from_godot(value);
                }
            }
        };

        (@export $Ty:ty) => {
            impl Export for $Ty {
                fn default_export_info() -> PropertyHintInfo {
                    PropertyHintInfo::with_hint_none(<$Ty as $crate::builtin::meta::GodotType>::godot_type_name())
                }
            }
        };

        (@type_string_hint $Ty:ty) => {
            impl TypeStringHint for $Ty {
                fn type_string() -> String {
                    use sys::GodotFfi;
                    let variant_type = <$Ty as $crate::builtin::meta::GodotType>::Ffi::variant_type();
                    let type_name = <$Ty as $crate::builtin::meta::GodotType>::godot_type_name();
                    format!("{}:{}", variant_type as i32, type_name)
                }
            }
        }
    }

    // Bounding Boxes
    impl_property_by_godot_convert!(Aabb);
    impl_property_by_godot_convert!(Rect2);
    impl_property_by_godot_convert!(Rect2i);

    // Matrices
    impl_property_by_godot_convert!(Basis);
    impl_property_by_godot_convert!(Transform2D);
    impl_property_by_godot_convert!(Transform3D);
    impl_property_by_godot_convert!(Projection);

    // Vectors
    impl_property_by_godot_convert!(Vector2);
    impl_property_by_godot_convert!(Vector2i);
    impl_property_by_godot_convert!(Vector3);
    impl_property_by_godot_convert!(Vector3i);
    impl_property_by_godot_convert!(Vector4);
    impl_property_by_godot_convert!(Vector4i);

    // Misc Math
    impl_property_by_godot_convert!(Quaternion);
    impl_property_by_godot_convert!(Plane);

    // Stringy Types
    impl_property_by_godot_convert!(GString);
    impl_property_by_godot_convert!(StringName);
    impl_property_by_godot_convert!(NodePath);

    impl_property_by_godot_convert!(Color);

    // Arrays
    // We manually implement `Export`.
    impl_property_by_godot_convert!(PackedByteArray, no_export);
    impl_property_by_godot_convert!(PackedInt32Array, no_export);
    impl_property_by_godot_convert!(PackedInt64Array, no_export);
    impl_property_by_godot_convert!(PackedFloat32Array, no_export);
    impl_property_by_godot_convert!(PackedFloat64Array, no_export);
    impl_property_by_godot_convert!(PackedStringArray, no_export);
    impl_property_by_godot_convert!(PackedVector2Array, no_export);
    impl_property_by_godot_convert!(PackedVector3Array, no_export);
    impl_property_by_godot_convert!(PackedVector4Array, no_export);
    impl_property_by_godot_convert!(PackedColorArray, no_export);

    // Primitives
    impl_property_by_godot_convert!(f64);
    impl_property_by_godot_convert!(i64);
    impl_property_by_godot_convert!(bool);

    // Godot uses f64 internally for floats, and if Godot tries to pass an invalid f32 into a rust property
    // then the property will just round the value or become inf.
    impl_property_by_godot_convert!(f32);

    // Godot uses i64 internally for integers, and if Godot tries to pass an invalid integer into a property
    // accepting one of the below values then rust will panic. In the editor this will appear as the property
    // failing to be set to a value and an error printed in the console. During runtime this will crash the
    // program and print the panic from rust stating that the property cannot store the value.
    impl_property_by_godot_convert!(i32);
    impl_property_by_godot_convert!(i16);
    impl_property_by_godot_convert!(i8);
    impl_property_by_godot_convert!(u32);
    impl_property_by_godot_convert!(u16);
    impl_property_by_godot_convert!(u8);

    // Callables and Signals are useless when exported to the editor, so we only need to make them available as
    // properties.
    impl_property_by_godot_convert!(Callable, no_export);
    impl_property_by_godot_convert!(Signal, no_export);

    // RIDs when exported act slightly weird. They are largely read-only, however you can reset them to their
    // default value. This seems to me very unintuitive. Since if we are storing an RID we would likely not
    // want that RID to be spuriously resettable. And if used for debugging purposes we can use another
    // mechanism than exporting the RID to the editor. Such as exporting a string containing the RID.
    //
    // Additionally, RIDs aren't persistent, and can sometimes behave a bit weirdly when passed from the
    // editor to the runtime.
    impl_property_by_godot_convert!(Rid, no_export);

    // impl_property_by_godot_convert!(Signal);
}
