/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::builtin::GodotString;
use crate::engine::global::PropertyHint;

/// Trait implemented for types that can be used as `#[var]` fields. This creates a copy of the
/// value, for some type-specific definition of "copy". For example, `Array`, `Dictionary` and `Gd` are
/// returned via `Share::share()` instead of copying the actual data.
pub trait Property {
    type Intermediate;

    fn get_property(&self) -> Self::Intermediate;
    fn set_property(&mut self, value: Self::Intermediate);
}

/// Trait implemented for types that can be used as `#[export]` fields.
pub trait Export: Property {
    /// The export info to use for an exported field of this type, if no other export info is specified.
    fn default_export_info() -> ExportInfo;
}

/// Info needed for godot to understand how to export a type to the editor.
#[derive(Clone, Eq, PartialEq, Debug)]
pub struct ExportInfo {
    pub hint: PropertyHint,
    pub hint_string: GodotString,
}

impl ExportInfo {
    /// Create a new `ExportInfo` with a property hint of
    /// [`PROPERTY_HINT_NONE`](PropertyHint::PROPERTY_HINT_NONE).
    pub fn with_hint_none() -> Self {
        Self {
            hint: PropertyHint::PROPERTY_HINT_NONE,
            hint_string: GodotString::new(),
        }
    }
}

impl<T: TypeStringHint> TypeStringHint for Option<T> {
    fn type_string() -> String {
        T::type_string()
    }
}

impl<T> Property for Option<T>
where
    T: Property + From<<T as Property>::Intermediate>,
{
    type Intermediate = Option<T::Intermediate>;

    fn get_property(&self) -> Self::Intermediate {
        self.as_ref().map(Property::get_property)
    }

    fn set_property(&mut self, value: Self::Intermediate) {
        match value {
            Some(value) => {
                if let Some(current_value) = self {
                    current_value.set_property(value)
                } else {
                    *self = Some(T::from(value))
                }
            }
            None => *self = None,
        }
    }
}

impl<T> Export for Option<T>
where
    T: Export + From<<T as Property>::Intermediate>,
{
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

/// To export properties to Godot, you must have an impl-block with the `#[godot_api]` attribute, even if
/// it is empty.
///
/// This trait is automatically implemented when such an impl-block is present. If Rust complains that it is
/// not implemented, then you can usually fix this by adding:
///
/// ```ignore
/// #[godot_api]
/// impl MyClass {}
/// ```
///
/// Where you replace `MyClass` with the name of your class.
#[allow(non_camel_case_types)]
pub trait Cannot_export_without_godot_api_impl {
    const EXISTS: () = ();
}

/// Functions used to translate user-provided arguments into export hints.
pub mod export_info_functions {
    use crate::builtin::GodotString;
    use crate::engine::global::PropertyHint;

    use super::ExportInfo;

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
        or_greater: bool,
        or_less: bool,
        exp: bool,
        radians: bool,
        degrees: bool,
        hide_slider: bool,
    ) -> ExportInfo {
        let min_max = format!("{},{}", min, max);

        let rest =
            comma_separate_boolean_idents!(or_greater, or_less, exp, radians, degrees, hide_slider);

        let hint_string = if rest.is_empty() {
            min_max
        } else {
            format!("{min_max},{rest}")
        };

        ExportInfo {
            hint: PropertyHint::PROPERTY_HINT_RANGE,
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

    pub fn export_enum<T>(variants: &[T]) -> ExportInfo
    where
        for<'a> &'a T: Into<EnumVariant>,
    {
        let hint_string: String = EnumVariant::slice_as_hint_string(variants);

        ExportInfo {
            hint: PropertyHint::PROPERTY_HINT_ENUM,
            hint_string: hint_string.into(),
        }
    }

    pub fn export_exp_easing(attenuation: bool, positive_only: bool) -> ExportInfo {
        let hint_string = comma_separate_boolean_idents!(attenuation, positive_only);

        ExportInfo {
            hint: PropertyHint::PROPERTY_HINT_EXP_EASING,
            hint_string: hint_string.into(),
        }
    }

    type BitFlag = ExportValueWithKey<u32>;

    pub fn export_flags<T>(bits: &[T]) -> ExportInfo
    where
        for<'a> &'a T: Into<BitFlag>,
    {
        let hint_string = BitFlag::slice_as_hint_string(bits);

        ExportInfo {
            hint: PropertyHint::PROPERTY_HINT_FLAGS,
            hint_string: hint_string.into(),
        }
    }

    pub fn export_file<S: AsRef<str>>(filter: S) -> ExportInfo {
        export_file_inner(false, filter)
    }

    pub fn export_global_file<S: AsRef<str>>(filter: S) -> ExportInfo {
        export_file_inner(true, filter)
    }

    pub fn export_file_inner<S: AsRef<str>>(global: bool, filter: S) -> ExportInfo {
        let hint = if global {
            PropertyHint::PROPERTY_HINT_FILE
        } else {
            PropertyHint::PROPERTY_HINT_GLOBAL_FILE
        };

        ExportInfo {
            hint,
            hint_string: filter.as_ref().into(),
        }
    }

    pub fn export_placeholder<S: AsRef<str>>(placeholder: S) -> ExportInfo {
        ExportInfo {
            hint: PropertyHint::PROPERTY_HINT_PLACEHOLDER_TEXT,
            hint_string: placeholder.as_ref().into(),
        }
    }

    macro_rules! default_export_funcs {
        (
            $( $function_name:ident => $property_hint:ident, )*
        ) => {
            $(
                pub fn $function_name() -> ExportInfo {
                    ExportInfo {
                        hint: PropertyHint::$property_hint,
                        hint_string: GodotString::new()
                    }
                }
            )*
        };
    }

    // The left side of these declarations follows the export annotation provided by GDScript, whereas the
    // right side are the corresponding property hint. Godot is not always consistent between the two, such
    // as `export_multiline` being `PROPERTY_HINT_MULTILINE_TEXT`.
    default_export_funcs!(
        export_flags_2d_physics => PROPERTY_HINT_LAYERS_2D_PHYSICS,
        export_flags_2d_render => PROPERTY_HINT_LAYERS_2D_RENDER,
        export_flags_2d_navigation => PROPERTY_HINT_LAYERS_2D_NAVIGATION,
        export_flags_3d_physics => PROPERTY_HINT_LAYERS_3D_PHYSICS,
        export_flags_3d_render => PROPERTY_HINT_LAYERS_3D_RENDER,
        export_flags_3d_navigation => PROPERTY_HINT_LAYERS_3D_NAVIGATION,
        export_dir => PROPERTY_HINT_DIR,
        export_global_dir => PROPERTY_HINT_GLOBAL_DIR,
        export_multiline => PROPERTY_HINT_MULTILINE_TEXT,
        export_color_no_alpha => PROPERTY_HINT_COLOR_NO_ALPHA,
    );
}

mod export_impls {
    use super::*;
    use crate::builtin::*;

    macro_rules! impl_property_by_clone {
        ($Ty:ty => $variant_type:ident, no_export) => {
            impl_property_by_clone!(@property $Ty => $variant_type);
        };

        ($Ty:ty => $variant_type:ident) => {
            impl_property_by_clone!(@property $Ty => $variant_type);
            impl_property_by_clone!(@export $Ty);
        };

        (@property $Ty:ty => $variant_type:ident) => {
            impl Property for $Ty {
                type Intermediate = Self;

                fn get_property(&self) -> Self {
                    self.clone()
                }

                fn set_property(&mut self, value: Self) {
                    *self = value;
                }
            }
        };

        (@export $Ty:ty) => {
            impl Export for $Ty {
                fn default_export_info() -> ExportInfo {
                    ExportInfo::with_hint_none()
                }
            }
        };
    }

    // Bounding Boxes
    impl_property_by_clone!(Aabb => Aabb);
    impl_property_by_clone!(Rect2 => Rect2);
    impl_property_by_clone!(Rect2i => Rect2i);

    // Matrices
    impl_property_by_clone!(Basis => Basis);
    impl_property_by_clone!(Transform2D => Transform2D);
    impl_property_by_clone!(Transform3D => Transform3D);
    impl_property_by_clone!(Projection => Projection);

    // Vectors
    impl_property_by_clone!(Vector2 => Vector2);
    impl_property_by_clone!(Vector2i => Vector2i);
    impl_property_by_clone!(Vector3 => Vector3);
    impl_property_by_clone!(Vector3i => Vector3i);
    impl_property_by_clone!(Vector4 => Vector4);
    impl_property_by_clone!(Vector4i => Vector4i);

    // Misc Math
    impl_property_by_clone!(Quaternion => Quaternion);
    impl_property_by_clone!(Plane => Plane);

    // Stringy Types
    impl_property_by_clone!(GodotString => String);
    impl_property_by_clone!(StringName => StringName);
    impl_property_by_clone!(NodePath => NodePath);

    impl_property_by_clone!(Color => Color);

    // Arrays
    impl_property_by_clone!(PackedByteArray => PackedByteArray);
    impl_property_by_clone!(PackedInt32Array => PackedInt32Array);
    impl_property_by_clone!(PackedInt64Array => PackedInt64Array);
    impl_property_by_clone!(PackedFloat32Array => PackedFloat32Array);
    impl_property_by_clone!(PackedFloat64Array => PackedFloat64Array);
    impl_property_by_clone!(PackedStringArray => PackedStringArray);
    impl_property_by_clone!(PackedVector2Array => PackedVector2Array);
    impl_property_by_clone!(PackedVector3Array => PackedVector3Array);
    impl_property_by_clone!(PackedColorArray => PackedColorArray);

    // Primitives
    impl_property_by_clone!(f64 => Float);
    impl_property_by_clone!(i64 => Int);
    impl_property_by_clone!(bool => Bool);

    // Godot uses f64 internally for floats, and if Godot tries to pass an invalid f32 into a rust property
    // then the property will just round the value or become inf.
    impl_property_by_clone!(f32 => Float);

    // Godot uses i64 internally for integers, and if Godot tries to pass an invalid integer into a property
    // accepting one of the below values then rust will panic. In the editor this will appear as the property
    // failing to be set to a value and an error printed in the console. During runtime this will crash the
    // program and print the panic from rust stating that the property cannot store the value.
    impl_property_by_clone!(i32 => Int);
    impl_property_by_clone!(i16 => Int);
    impl_property_by_clone!(i8 => Int);
    impl_property_by_clone!(u32 => Int);
    impl_property_by_clone!(u16 => Int);
    impl_property_by_clone!(u8 => Int);

    // Callables are useless when exported to the editor, so we only need to make them available as
    // properties.
    impl_property_by_clone!(Callable => Callable, no_export);

    // RIDs when exported act slightly weird. They are largely read-only, however you can reset them to their
    // default value. This seems to me very unintuitive. Since if we are storing an RID we would likely not
    // want that RID to be spuriously resettable. And if used for debugging purposes we can use another
    // mechanism than exporting the RID to the editor. Such as exporting a string containing the RID.
    //
    // Additionally, RIDs aren't persistent, and can sometimes behave a bit weirdly when passed from the
    // editor to the runtime.
    impl_property_by_clone!(Rid => Rid, no_export);

    // impl_property_by_clone!(Signal => Signal);
}
