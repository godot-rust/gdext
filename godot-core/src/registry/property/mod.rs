/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! Registration support for property types.

use std::fmt::Display;

use godot_ffi as sys;
use godot_ffi::{GodotNullableFfi, VariantType};

use crate::classes;
use crate::global::PropertyHint;
use crate::meta::{ClassName, FromGodot, GodotConvert, GodotType, PropertyHintInfo, ToGodot};
use crate::obj::{EngineEnum, GodotClass};

mod phantom_var;

pub use phantom_var::PhantomVar;

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Trait definitions

// Note: HTML link for #[var] works if this symbol is inside prelude, but not in register::property.
/// Trait implemented for types that can be used as [`#[var]`](../register/derive.GodotClass.html#properties-and-exports) fields.
///
/// This creates a copy of the value, according to copy semantics provided by `Clone`. For example, `Array`, `Dictionary` and `Gd` are
/// returned by shared reference instead of copying the actual data.
///
/// This does not require [`FromGodot`] or [`ToGodot`], so that something can be used as a property even if it can't be used in function
/// arguments/return types.
///
/// See also [`Export`], a specialization of this trait for properties exported to the editor UI.
///
/// For enums, this trait can be derived using the [`#[derive(Var)]`](../derive.Var.html) macro.
#[doc(alias = "property")]
//
// on_unimplemented: we also mention #[export] here, because we can't control the order of error messages.
// Missing Export often also means missing Var trait, and so the Var error message appears first.
#[diagnostic::on_unimplemented(
    message = "`#[var]` properties require `Var` trait; #[export] ones require `Export` trait",
    label = "type cannot be used as a property",
    note = "see also: https://godot-rust.github.io/book/register/properties.html"
)]
pub trait Var: GodotConvert {
    fn get_property(&self) -> Self::Via;

    fn set_property(&mut self, value: Self::Via);

    /// Specific property hints, only override if they deviate from [`GodotType::property_info`], e.g. for enums/newtypes.
    fn var_hint() -> PropertyHintInfo {
        Self::Via::property_hint_info()
    }
}

// Note: HTML link for #[export] works if this symbol is inside prelude, but not in register::property.
/// Trait implemented for types that can be used as [`#[export]`](../register/derive.GodotClass.html#properties-and-exports) fields.
///
/// To export objects, see the [_Exporting_ section of `Gd<T>`](../obj/struct.Gd.html#exporting).
///
/// For enums, this trait can be derived using the [`#[derive(Export)]`](../derive.Export.html) macro.
#[doc(alias = "property")]
//
// on_unimplemented: mentioning both Var + Export; see above.
#[diagnostic::on_unimplemented(
    message = "`#[var]` properties require `Var` trait; #[export] ones require `Export` trait",
    label = "type cannot be used as a property",
    note = "see also: https://godot-rust.github.io/book/register/properties.html",
    note = "`Gd` and `DynGd` cannot be exported directly; wrap them in `Option<...>` or `OnEditor<...>`."
)]
pub trait Export: Var {
    /// The export info to use for an exported field of this type, if no other export info is specified.
    fn export_hint() -> PropertyHintInfo {
        <Self as Var>::var_hint()
    }

    /// If this is a class inheriting `Node`, returns the `ClassName`; otherwise `None`.
    ///
    /// Only overridden for `Gd<T>`, to detect erroneous exports of `Node` inside a `Resource` class.
    #[allow(clippy::wrong_self_convention)]
    #[doc(hidden)]
    fn as_node_class() -> Option<ClassName> {
        None
    }
}

/// Marker trait to identify `GodotType`s that can be directly used with an `#[export]`.
///
/// Implemented pretty much for all the [`GodotTypes`][GodotType] that are not [`GodotClass`].
/// Provides a few blanket implementations and, by itself, has no implications
/// for the [`Var`] or [`Export`] traits.
///
/// Types which don't implement the `BuiltinExport` trait can't be used directly as an `#[export]`
/// and must be handled using associated algebraic types, such as:
/// * [`Option<T>`], which represents optional value that can be null when used.
/// * [`OnEditor<T>`][crate::obj::OnEditor], which represents value that must not be null when used.
// Some Godot Types which are inherently non-nullable (e.g., `Gd<T>`),
// might have their value set to null by the editor. Additionally, Godot must generate
// initial, default value for such properties, causing memory leaks.
// Such `GodotType`s don't implement `BuiltinExport`.
//
// Note: This marker trait is required to create a blanket implementation
// for `OnEditor<T>` where `T` is anything other than `GodotClass`.
// An alternative approach would involve introducing an extra associated type
// to `GodotType` trait. However, this would not be ideal — `GodotType` is used
// in contexts unrelated to `#[export]`, and adding unnecessary complexity
// should be avoided. Since Rust does not yet support specialization (i.e. negative trait bounds),
// this `MarkerTrait` serves as the intended solution to recognize aforementioned types.
pub trait BuiltinExport {}

/// This function only exists as a place to add doc-tests for the `Export` trait.
///
/// Test with export of exportable type should succeed:
/// ```no_run
/// use godot::prelude::*;
///
/// #[derive(GodotClass)]
/// #[class(init)]
/// struct Foo {
///     #[export]
///     obj: Option<Gd<Resource>>,
///     #[export]
///     array: Array<Gd<Resource>>,
/// }
/// ```
///
/// Tests with export of non-exportable type should fail:
/// ```compile_fail
/// use godot::prelude::*;
///
/// #[derive(GodotClass)]
/// #[class(init)]
/// struct Foo {
///     #[export]
///     obj: Option<Gd<Object>>,
/// }
/// ```
///
/// Neither `Gd<T>` nor `DynGd<T, D>` can be used with an `#[export]` directly:
///
/// ```compile_fail
///  use godot::prelude::*;
///
/// #[derive(GodotClass)]
/// #[class(init, base = Node)]
/// struct MyClass {
///     #[export]
///     editor_property: Gd<Resource>,
/// }
/// ```
///
/// ```compile_fail
///  use godot::prelude::*;
///
/// #[derive(GodotClass)]
/// #[class(init, base = Node)]
/// struct MyClass {
///     #[export]
///     editor_property: DynGd<Node, dyn Display>,
/// }
/// ```
///
/// ```compile_fail
/// use godot::prelude::*;
///
/// #[derive(GodotClass)]
/// #[class(init)]
/// struct Foo {
///     #[export]
///     array: Array<Gd<Object>>,
/// }
/// ```
#[allow(dead_code)]
fn export_doctests() {}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Blanket impls for Option<T>

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
    fn export_hint() -> PropertyHintInfo {
        T::export_hint()
    }
}

impl<T> BuiltinExport for Option<T>
where
    T: GodotType,
    T::Ffi: GodotNullableFfi,
{
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Export machinery

/// Functions used to translate user-provided arguments into export hints.
///
/// You are not supposed to use these functions directly. They are used by the `#[export]` macro to generate the correct export hint.
///
/// Each function is named the same as the equivalent Godot annotation.  
/// For instance, `@export_range` in Godot is `fn export_range` here.
pub mod export_info_functions {
    use godot_ffi::VariantType;

    use crate::builtin::GString;
    use crate::global::PropertyHint;
    use crate::meta::{GodotType, PropertyHintInfo, PropertyInfo};
    use crate::obj::EngineEnum;
    use crate::registry::property::Export;

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
    /// Mark an exported numerical value to use the editor's range UI.
    ///
    /// You'll never call this function itself, but will instead use the macro `#[export(range=(...))]`, as below.  The syntax is
    /// very similar to Godot's [`@export_range`](https://docs.godotengine.org/en/stable/classes/class_%40gdscript.html#class-gdscript-annotation-export-range).
    /// `min`, `max`, and `step` are `f32` positional arguments, with `step` being optional and defaulting to `1.0`.  The rest of
    /// the arguments can be written in any order.  The symbols of type `bool` just need to have those symbols written, and those of type `Option<T>` will be written as `{KEY}={VALUE}`, e.g. `suffix="px"`.
    ///
    /// ```
    /// # use godot::prelude::*;
    /// #[derive(GodotClass)]
    /// #[class(init, base=Node)]
    /// struct MyClassWithRangedValues {
    ///     #[export(range=(0.0, 400.0, 1.0, or_greater, suffix="px"))]
    ///     icon_width: i32,
    ///     #[export(range=(-180.0, 180.0, degrees))]
    ///     angle: f32,
    /// }
    /// ```
    #[allow(clippy::too_many_arguments)]
    pub fn export_range(
        min: f64,
        max: f64,
        step: Option<f64>,
        or_greater: bool,
        or_less: bool,
        exp: bool,
        radians_as_degrees: bool,
        degrees: bool,
        hide_slider: bool,
        suffix: Option<String>,
    ) -> PropertyHintInfo {
        // From Godot 4.4, GDScript uses `.0` for integral floats, see https://github.com/godotengine/godot/pull/47502.
        // We still register them the old way, to test compatibility. See also property_template_test.rs.

        let hint_beginning = if let Some(step) = step {
            format!("{min},{max},{step}")
        } else {
            format!("{min},{max}")
        };

        let rest = comma_separate_boolean_idents!(
            or_greater,
            or_less,
            exp,
            radians_as_degrees,
            degrees,
            hide_slider
        );

        let mut hint_string = hint_beginning;
        if !rest.is_empty() {
            hint_string.push_str(&format!(",{rest}"));
        }
        if let Some(suffix) = suffix {
            hint_string.push_str(&format!(",suffix:{suffix}"));
        }

        PropertyHintInfo {
            hint: PropertyHint::RANGE,
            hint_string: hint_string.into(),
        }
    }

    #[doc(hidden)]
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

    /// Equivalent to `@export_enum` in Godot.
    ///
    /// A name without a key would be represented as `(name, None)`, and a name with a key as `(name, Some(key))`.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use godot::register::property::export_info_functions::export_enum;
    /// export_enum(&[("a", None), ("b", Some(10))]);
    /// ```
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

    /// Equivalent to `@export_flags` in Godot.
    ///
    /// A flag without a key would be represented as `(flag, None)`, and a flag with a key as `(flag, Some(key))`.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use godot::register::property::export_info_functions::export_flags;
    /// export_flags(&[("a", None), ("b", Some(10))]);
    /// ```
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

    /// Handles `@export_file`, `@export_global_file`, `@export_dir` and `@export_global_dir`.
    pub fn export_file_or_dir<T: Export>(
        is_file: bool,
        is_global: bool,
        filter: impl AsRef<str>,
    ) -> PropertyHintInfo {
        let field_ty = T::Via::property_info("");
        let filter = filter.as_ref();
        debug_assert!(is_file || filter.is_empty()); // Dir never has filter.

        export_file_or_dir_inner(&field_ty, is_file, is_global, filter)
    }

    pub fn export_file_or_dir_inner(
        field_ty: &PropertyInfo,
        is_file: bool,
        is_global: bool,
        filter: &str,
    ) -> PropertyHintInfo {
        let hint = match (is_file, is_global) {
            (true, true) => PropertyHint::GLOBAL_FILE,
            (true, false) => PropertyHint::FILE,
            (false, true) => PropertyHint::GLOBAL_DIR,
            (false, false) => PropertyHint::DIR,
        };

        // Returned value depends on field type.
        match field_ty.variant_type {
            // GString field:
            // { "type": 4, "hint": 13, "hint_string": "*.png" }
            VariantType::STRING => PropertyHintInfo {
                hint,
                hint_string: GString::from(filter),
            },

            // Array<GString> or PackedStringArray field:
            // { "type": 28, "hint": 23, "hint_string": "4/13:*.png" }
            #[cfg(since_api = "4.3")]
            VariantType::PACKED_STRING_ARRAY => to_string_array_hint(hint, filter),
            #[cfg(since_api = "4.3")]
            VariantType::ARRAY if field_ty.is_array_of_elem::<GString>() => {
                to_string_array_hint(hint, filter)
            }

            _ => {
                // E.g. `global_file`.
                let attribute_name = hint.as_str().to_lowercase();

                // TODO nicer error handling.
                // Compile time may be difficult (at least without extra traits... maybe const fn?). But at least more context info, field name etc.
                #[cfg(since_api = "4.3")]
                panic!(
                    "#[export({attribute_name})] only supports GString, Array<String> or PackedStringArray field types\n\
                    encountered: {field_ty:?}"
                );

                #[cfg(before_api = "4.3")]
                panic!(
                    "#[export({attribute_name})] only supports GString type prior to Godot 4.3\n\
                    encountered: {field_ty:?}"
                );
            }
        }
    }

    /// For `Array<GString>` and `PackedStringArray` fields using one of the `@export[_global]_{file|dir}` annotations.
    ///
    /// Formats: `"4/13:"`, `"4/15:*.png"`, ...
    fn to_string_array_hint(hint: PropertyHint, filter: &str) -> PropertyHintInfo {
        let variant_ord = VariantType::STRING.ord(); // "4"
        let hint_ord = hint.ord();
        let hint_string = format!("{variant_ord}/{hint_ord}");

        PropertyHintInfo {
            hint: PropertyHint::TYPE_STRING,
            hint_string: format!("{hint_string}:{filter}").into(),
        }
    }

    pub fn export_placeholder<S: AsRef<str>>(placeholder: S) -> PropertyHintInfo {
        PropertyHintInfo {
            hint: PropertyHint::PLACEHOLDER_TEXT,
            hint_string: GString::from(placeholder.as_ref()),
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
        export_storage => NONE, // Storage exports don't display in the editor.
        export_flags_2d_physics => LAYERS_2D_PHYSICS,
        export_flags_2d_render => LAYERS_2D_RENDER,
        export_flags_2d_navigation => LAYERS_2D_NAVIGATION,
        export_flags_3d_physics => LAYERS_3D_PHYSICS,
        export_flags_3d_render => LAYERS_3D_RENDER,
        export_flags_3d_navigation => LAYERS_3D_NAVIGATION,
        export_multiline => MULTILINE_TEXT,
        export_color_no_alpha => COLOR_NO_ALPHA,
    );
}

mod export_impls {
    use super::*;
    use crate::builtin::*;

    macro_rules! impl_property_by_godot_convert {
        ($Ty:ty, no_export) => {
            impl_property_by_godot_convert!(@property $Ty);
        };

        ($Ty:ty) => {
            impl_property_by_godot_convert!(@property $Ty);
            impl_property_by_godot_convert!(@export $Ty);
            impl_property_by_godot_convert!(@builtin $Ty);
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
                fn export_hint() -> PropertyHintInfo {
                    PropertyHintInfo::type_name::<$Ty>()
                }
            }
        };

        (@builtin $Ty:ty) => {
            impl BuiltinExport for $Ty {}
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

    // Dictionary: will need to be done manually once they become typed.
    impl_property_by_godot_convert!(Dictionary);
    impl_property_by_godot_convert!(Variant);

    // Packed arrays: we manually implement `Export`.
    impl_property_by_godot_convert!(PackedByteArray, no_export);
    impl_property_by_godot_convert!(PackedInt32Array, no_export);
    impl_property_by_godot_convert!(PackedInt64Array, no_export);
    impl_property_by_godot_convert!(PackedFloat32Array, no_export);
    impl_property_by_godot_convert!(PackedFloat64Array, no_export);
    impl_property_by_godot_convert!(PackedStringArray, no_export);
    impl_property_by_godot_convert!(PackedVector2Array, no_export);
    impl_property_by_godot_convert!(PackedVector3Array, no_export);
    #[cfg(since_api = "4.3")]
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

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Crate-local utilities

pub(crate) fn builtin_type_string<T: GodotType>() -> String {
    use sys::GodotFfi as _;

    let variant_type = T::Ffi::VARIANT_TYPE.variant_as_nil();

    // Godot 4.3 changed representation for type hints, see https://github.com/godotengine/godot/pull/90716.
    if sys::GdextBuild::since_api("4.3") {
        format!("{}:", variant_type.ord())
    } else {
        format!("{}:{}", variant_type.ord(), T::godot_type_name())
    }
}

/// Creates `hint_string` to be used for given `GodotClass` when used as an `ArrayElement`.
pub(crate) fn object_export_element_type_string<T>(class_hint: impl Display) -> String
where
    T: GodotClass,
{
    let hint = if T::inherits::<classes::Resource>() {
        Some(PropertyHint::RESOURCE_TYPE)
    } else if T::inherits::<classes::Node>() {
        Some(PropertyHint::NODE_TYPE)
    } else {
        None
    };

    // Exportable classes (Resource/Node based) include the {RESOURCE|NODE}_TYPE hint + the class name.
    if let Some(export_hint) = hint {
        format!(
            "{variant}/{hint}:{class}",
            variant = VariantType::OBJECT.ord(),
            hint = export_hint.ord(),
            class = class_hint
        )
    } else {
        // Previous impl: format!("{variant}:", variant = VariantType::OBJECT.ord())
        unreachable!("element_type_string() should only be invoked for exportable classes")
    }
}
