/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::builtin::meta::GodotType;

use super::*;

impl<T: GodotCompatible> GodotCompatible for Option<T>
where
    Option<T::Via>: GodotType,
{
    type Via = Option<T::Via>;
}

impl<T: ToGodot> ToGodot for Option<T>
where
    Option<T::Via>: GodotType,
{
    fn to_godot(&self) -> Self::Via {
        self.as_ref().map(ToGodot::to_godot)
    }

    fn into_godot(self) -> Self::Via {
        self.map(ToGodot::into_godot)
    }
}

impl<T: FromGodot> FromGodot for Option<T>
where
    Option<T::Via>: GodotType,
{
    fn try_from_godot(via: Self::Via) -> Option<Self> {
        match via {
            Some(via) => T::try_from_godot(via).map(Some),
            None => Some(None),
        }
    }

    fn from_godot(via: Self::Via) -> Self {
        via.map(T::from_godot)
    }
}

impl GodotCompatible for sys::VariantType {
    type Via = i64;
}

impl ToGodot for sys::VariantType {
    fn to_godot(&self) -> Self::Via {
        *self as i64
    }

    fn into_godot(self) -> Self::Via {
        self as i64
    }
}

impl FromGodot for sys::VariantType {
    fn try_from_godot(via: Self::Via) -> Option<Self> {
        Some(Self::from_sys(via as sys::GDExtensionVariantType))
    }
}

impl GodotCompatible for sys::VariantOperator {
    type Via = i64;
}

impl ToGodot for sys::VariantOperator {
    fn to_godot(&self) -> Self::Via {
        *self as i64
    }

    fn into_godot(self) -> Self::Via {
        self as i64
    }
}

impl FromGodot for sys::VariantOperator {
    fn try_from_godot(via: Self::Via) -> Option<Self> {
        Some(Self::from_sys(via as sys::GDExtensionVariantOperator))
    }
}

impl<T> GodotCompatible for *mut T {
    type Via = i64;
}

impl<T> ToGodot for *mut T {
    fn to_godot(&self) -> Self::Via {
        *self as i64
    }
}

impl<T> FromGodot for *mut T {
    fn try_from_godot(via: Self::Via) -> Option<Self> {
        Some(via as Self)
    }
}

impl<T> GodotCompatible for *const T {
    type Via = i64;
}

impl<T> ToGodot for *const T {
    fn to_godot(&self) -> Self::Via {
        *self as i64
    }
}

impl<T> FromGodot for *const T {
    fn try_from_godot(via: Self::Via) -> Option<Self> {
        Some(via as Self)
    }
}

mod scalars {
    use super::{impl_godot_as_self, FromGodot, GodotCompatible, ToGodot};
    use crate::builtin::meta::GodotType;
    use godot_ffi as sys;

    macro_rules! impl_godot {
        ($T:ty as $Via:ty $(, $param_metadata:expr)?) => {
            impl GodotType for $T {
                type Ffi = $Via;

                fn to_ffi(&self) -> Self::Ffi {
                    (*self).into()
                }

                fn into_ffi(self) -> Self::Ffi {
                    self.into()
                }

                fn try_from_ffi(ffi: Self::Ffi) -> Option<Self> {
                    Self::try_from(ffi).ok()
                }

                $(
                    fn param_metadata() -> sys::GDExtensionClassMethodArgumentMetadata {
                        $param_metadata
                    }
                )?
            }

            impl GodotCompatible for $T {
                type Via = $T;
            }

            impl ToGodot for $T {
                fn to_godot(&self) -> Self::Via {
                    *self
                }
            }

            impl FromGodot for $T {
                fn try_from_godot(via: Self::Via) -> Option<Self> {
                    Some(via)
                }
            }
        };
        ($T:ty as $Via:ty $(, $param_metadata:expr)?; lossy) => {
            impl GodotType for $T {
                type Ffi = $Via;

                fn to_ffi(&self) -> Self::Ffi {
                    *self as $Via
                }

                fn into_ffi(self) -> Self::Ffi {
                    self as $Via
                }

                fn try_from_ffi(ffi: Self::Ffi) -> Option<Self> {
                    Some(ffi as $T)
                }

                $(
                    fn param_metadata() -> sys::GDExtensionClassMethodArgumentMetadata {
                        $param_metadata
                    }
                )?
            }

            impl GodotCompatible for $T {
                type Via = $T;
            }

            impl ToGodot for $T {
                fn to_godot(&self) -> Self::Via {
                    *self
                }
            }

            impl FromGodot for $T {
                fn try_from_godot(via: Self::Via) -> Option<Self> {
                    Some(via)
                }
            }
        };
    }

    impl_godot_as_self!(bool);
    impl_godot_as_self!(i64, sys::GDEXTENSION_METHOD_ARGUMENT_METADATA_INT_IS_INT64);
    impl_godot_as_self!(
        f64,
        sys::GDEXTENSION_METHOD_ARGUMENT_METADATA_REAL_IS_DOUBLE
    );
    impl_godot_as_self!(());

    impl_godot!(
        i32 as i64,
        sys::GDEXTENSION_METHOD_ARGUMENT_METADATA_INT_IS_INT32
    );
    impl_godot!(
        i16 as i64,
        sys::GDEXTENSION_METHOD_ARGUMENT_METADATA_INT_IS_INT16
    );
    impl_godot!(
        i8 as i64,
        sys::GDEXTENSION_METHOD_ARGUMENT_METADATA_INT_IS_INT8
    );
    impl_godot!(
        u32 as i64,
        sys::GDEXTENSION_METHOD_ARGUMENT_METADATA_INT_IS_UINT32
    );
    impl_godot!(
        u16 as i64,
        sys::GDEXTENSION_METHOD_ARGUMENT_METADATA_INT_IS_UINT16
    );
    impl_godot!(
        u8 as i64,
        sys::GDEXTENSION_METHOD_ARGUMENT_METADATA_INT_IS_UINT8
    );

    impl_godot!(
        u64 as i64,
        sys::GDEXTENSION_METHOD_ARGUMENT_METADATA_INT_IS_UINT64;
        lossy
    );
    impl_godot!(
        f32 as f64,
        sys::GDEXTENSION_METHOD_ARGUMENT_METADATA_REAL_IS_FLOAT;
        lossy
    );
}
