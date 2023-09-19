/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

pub mod registration;

mod class_name;
mod godot_compat;
mod return_marshal;
mod signature;

pub use class_name::*;
pub use godot_compat::*;
#[doc(hidden)]
pub use return_marshal::*;
#[doc(hidden)]
pub use signature::*;

use godot_ffi as sys;
use sys::{GodotFfi, GodotNullableFfi};

use crate::builtin::*;
use crate::engine::global;
use registration::method::MethodParamOrReturnInfo;

/// Conversion of GodotFfi-types into/from [`Variant`].
pub trait GodotFfiVariant: Sized + GodotFfi {
    fn ffi_to_variant(&self) -> Variant;
    fn ffi_from_variant(variant: &Variant) -> Result<Self, VariantConversionError>;
}

mod sealed {
    // To ensure the user does not implement `GodotType` for their own types.

    use godot_ffi::GodotNullableFfi;

    use super::GodotType;
    use crate::builtin::*;
    use crate::obj::*;

    pub trait Sealed {}

    impl Sealed for Aabb {}
    impl Sealed for Basis {}
    impl Sealed for Callable {}
    impl Sealed for Vector2 {}
    impl Sealed for Vector3 {}
    impl Sealed for Vector4 {}
    impl Sealed for Vector2i {}
    impl Sealed for Vector3i {}
    impl Sealed for Vector4i {}
    impl Sealed for Quaternion {}
    impl Sealed for Color {}
    impl Sealed for GodotString {}
    impl Sealed for StringName {}
    impl Sealed for NodePath {}
    impl Sealed for PackedByteArray {}
    impl Sealed for PackedInt32Array {}
    impl Sealed for PackedInt64Array {}
    impl Sealed for PackedFloat32Array {}
    impl Sealed for PackedFloat64Array {}
    impl Sealed for PackedStringArray {}
    impl Sealed for PackedVector2Array {}
    impl Sealed for PackedVector3Array {}
    impl Sealed for PackedColorArray {}
    impl Sealed for Plane {}
    impl Sealed for Projection {}
    impl Sealed for Rid {}
    impl Sealed for Rect2 {}
    impl Sealed for Rect2i {}
    impl Sealed for Signal {}
    impl Sealed for Transform2D {}
    impl Sealed for Transform3D {}
    impl Sealed for Dictionary {}
    impl Sealed for bool {}
    impl Sealed for i64 {}
    impl Sealed for i32 {}
    impl Sealed for i16 {}
    impl Sealed for i8 {}
    impl Sealed for u64 {}
    impl Sealed for u32 {}
    impl Sealed for u16 {}
    impl Sealed for u8 {}
    impl Sealed for f64 {}
    impl Sealed for f32 {}
    impl Sealed for () {}
    impl Sealed for Variant {}
    impl<T: GodotType> Sealed for Array<T> {}
    impl<T: GodotClass> Sealed for RawGd<T> {}
    impl<T: GodotClass> Sealed for Gd<T> {}
    impl<T> Sealed for Option<T>
    where
        T: GodotType,
        T::Ffi: GodotNullableFfi,
    {
    }
}

/// Types that can represent some Godot type.
///
/// This trait cannot be implemented for custom user types, for that you should see [`GodotCompatible`]
/// instead.
///
/// Unlike [`GodotFfi`], types implementing this trait don't need to fully represent its corresponding Godot
/// type. For instance [`i32`] does not implement [`GodotFfi`] because it cannot represent all values of
/// Godot's `int` type, however it does implement `GodotType` because we can set the metadata of values with
/// this type to indicate that they are 32 bits large.
pub trait GodotType: GodotCompatible<Via = Self> + ToGodot + FromGodot + sealed::Sealed {
    type Ffi: GodotFfiVariant;

    fn to_ffi(&self) -> Self::Ffi;
    fn into_ffi(self) -> Self::Ffi;
    fn try_from_ffi(ffi: Self::Ffi) -> Option<Self>;

    fn from_ffi(ffi: Self::Ffi) -> Self {
        Self::try_from_ffi(ffi).unwrap()
    }

    fn param_metadata() -> sys::GDExtensionClassMethodArgumentMetadata {
        Self::Ffi::default_param_metadata()
    }

    fn class_name() -> ClassName {
        // If we use `ClassName::of::<()>()` then this type shows up as `(no base)` in documentation.
        ClassName::none()
    }

    fn property_info(property_name: &str) -> PropertyInfo {
        PropertyInfo {
            variant_type: Self::Ffi::variant_type(),
            class_name: Self::class_name(),
            property_name: StringName::from(property_name),
            hint: global::PropertyHint::PROPERTY_HINT_NONE,
            hint_string: GodotString::new(),
            usage: global::PropertyUsageFlags::PROPERTY_USAGE_DEFAULT,
        }
    }

    fn argument_info(property_name: &str) -> MethodParamOrReturnInfo {
        MethodParamOrReturnInfo::new(Self::property_info(property_name), Self::param_metadata())
    }

    fn return_info() -> Option<MethodParamOrReturnInfo> {
        Some(MethodParamOrReturnInfo::new(
            Self::property_info(""),
            Self::param_metadata(),
        ))
    }
}

impl<T> GodotType for Option<T>
where
    T: GodotType,
    T::Ffi: GodotNullableFfi,
{
    type Ffi = T::Ffi;

    fn to_ffi(&self) -> Self::Ffi {
        GodotNullableFfi::flatten_option(self.as_ref().map(|t| t.to_ffi()))
    }

    fn into_ffi(self) -> Self::Ffi {
        GodotNullableFfi::flatten_option(self.map(|t| t.into_ffi()))
    }

    fn try_from_ffi(ffi: Self::Ffi) -> Option<Self> {
        if ffi.is_null() {
            return Some(None);
        }

        let t = GodotType::try_from_ffi(ffi);
        t.map(Some)
    }

    fn from_ffi(ffi: Self::Ffi) -> Self {
        if ffi.is_null() {
            return None;
        }

        Some(GodotType::from_ffi(ffi))
    }
}

/// Stores meta-information about registered types or properties.
///
/// Filling this information properly is important so that Godot can use ptrcalls instead of varcalls
/// (requires typed GDScript + sufficient information from the extension side)

// ----------------------------------------------------------------------------------------------------------------------------------------------

/// Rusty abstraction of `sys::GDExtensionPropertyInfo`.
///
/// Keeps the actual allocated values (the `sys` equivalent only keeps pointers, which fall out of scope).
#[derive(Debug)]
// Note: is not #[non_exhaustive], so adding fields is a breaking change. Mostly used internally at the moment though.
pub struct PropertyInfo {
    pub variant_type: VariantType,
    pub class_name: ClassName,
    pub property_name: StringName,
    pub hint: global::PropertyHint,
    pub hint_string: GodotString,
    pub usage: global::PropertyUsageFlags,
}

impl PropertyInfo {
    /// Converts to the FFI type. Keep this object allocated while using that!
    pub fn property_sys(&self) -> sys::GDExtensionPropertyInfo {
        use crate::obj::EngineEnum as _;

        sys::GDExtensionPropertyInfo {
            type_: self.variant_type.sys(),
            name: self.property_name.string_sys(),
            class_name: self.class_name.string_sys(),
            hint: u32::try_from(self.hint.ord()).expect("hint.ord()"),
            hint_string: self.hint_string.string_sys(),
            usage: u32::try_from(self.usage.ord()).expect("usage.ord()"),
        }
    }

    pub fn empty_sys() -> sys::GDExtensionPropertyInfo {
        use crate::obj::EngineEnum as _;

        sys::GDExtensionPropertyInfo {
            type_: VariantType::Nil.sys(),
            name: std::ptr::null_mut(),
            class_name: std::ptr::null_mut(),
            hint: global::PropertyHint::PROPERTY_HINT_NONE.ord() as u32,
            hint_string: std::ptr::null_mut(),
            usage: global::PropertyUsageFlags::PROPERTY_USAGE_NONE.ord() as u32,
        }
    }
}
