/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::builtin::GodotString;
use godot_ffi as sys;
use godot_ffi::GodotFfi;
use std::{fmt, ptr};
use sys::types::OpaqueVariant;
use sys::{ffi_methods, interface_fn};

mod impls;
mod variant_traits;

pub use impls::*;
pub use sys::{VariantOperator, VariantType};
pub use variant_traits::*;

#[repr(C, align(8))]
pub struct Variant {
    opaque: OpaqueVariant,
}

impl Variant {
    /// Create an empty variant (`null` value in GDScript).
    pub fn nil() -> Self {
        unsafe {
            Self::from_var_sys_init(|variant_ptr| {
                interface_fn!(variant_new_nil)(variant_ptr);
            })
        }
    }

    /// Create a variant holding a non-nil value.
    ///
    /// Equivalent to `value.to_variant()`.
    pub fn from<T: ToVariant>(value: T) -> Self {
        value.to_variant()
    }

    /// Convert to type `T`, panicking on failure.
    ///
    /// Equivalent to `T::from_variant(&self)`.
    ///
    /// # Panics
    /// When this variant holds a different type.
    #[cfg(feature = "convenience")]
    pub fn to<T: FromVariant>(&self) -> T {
        T::from_variant(self)
    }

    /// Convert to type `T`, returning `Err` on failure.
    ///
    /// Equivalent to `T::try_from_variant(&self)`.
    pub fn try_to<T: FromVariant>(&self) -> Result<T, VariantConversionError> {
        T::try_from_variant(self)
    }

    /// Checks whether the variant is empty (`null` value in GDScript).
    pub fn is_nil(&self) -> bool {
        self.sys_type() == sys::GDEXTENSION_VARIANT_TYPE_NIL
    }

    // TODO test
    pub fn get_type(&self) -> VariantType {
        let ty_sys = unsafe { interface_fn!(variant_get_type)(self.var_sys()) };
        VariantType::from_sys(ty_sys)
    }

    // TODO test
    #[allow(unused_mut)]
    pub fn evaluate(&self, rhs: &Variant, op: VariantOperator) -> Option<Variant> {
        let op_sys = op.sys();
        let mut is_valid = false as u8;

        let mut result = Variant::nil();
        unsafe {
            interface_fn!(variant_evaluate)(
                op_sys,
                self.var_sys(),
                rhs.var_sys(),
                result.var_sys(),
                ptr::addr_of_mut!(is_valid),
            )
        };

        if is_valid == 1 {
            Some(result)
        } else {
            None
        }
    }

    pub(crate) fn sys_type(&self) -> sys::GDExtensionVariantType {
        unsafe {
            let ty: sys::GDExtensionVariantType = interface_fn!(variant_get_type)(self.var_sys());
            ty
        }
    }

    #[allow(unused_mut)]
    fn stringify(&self) -> GodotString {
        let mut result = GodotString::new();
        unsafe {
            interface_fn!(variant_stringify)(self.var_sys(), result.string_sys());
        }
        result
    }

    fn from_opaque(opaque: OpaqueVariant) -> Self {
        Self { opaque }
    }

    // Conversions from/to Godot C++ `Variant*` pointers
    ffi_methods! {
        type sys::GDExtensionVariantPtr = *mut Opaque;

        fn from_var_sys = from_sys;
        fn from_var_sys_init = from_sys_init;
        fn var_sys = sys;
        fn write_var_sys = write_sys;
    }

    #[doc(hidden)]
    pub fn var_sys_const(&self) -> sys::GDExtensionConstVariantPtr {
        sys::to_const_ptr(self.var_sys())
    }

    /*#[doc(hidden)]
    pub unsafe fn from_var_sys_init(init_fn: impl FnOnce(sys::GDExtensionVariantPtr)) -> Self {
        // Can't use uninitialized pointer -- Variant implementation in C++ expects that on assignment,
        // the target pointer is an initialized Variant

        #[allow(unused_mut)]
        let mut result = Self::default();
        init_fn(result.var_sys());
        result
    }*/
}

impl GodotFfi for Variant {
    ffi_methods! {
        type sys::GDExtensionTypePtr = *mut Opaque;
        fn from_sys;
        fn sys;
        fn write_sys;
    }

    unsafe fn from_sys_init(init_fn: impl FnOnce(sys::GDExtensionTypePtr)) -> Self {
        // Can't use uninitialized pointer -- Variant implementation in C++ expects that on assignment,
        // the target pointer is an initialized Variant

        let mut result = Self::default();
        init_fn(result.sys_mut());
        result
    }
}

impl Clone for Variant {
    fn clone(&self) -> Self {
        unsafe {
            Self::from_var_sys_init(|variant_ptr| {
                interface_fn!(variant_new_copy)(variant_ptr, self.var_sys());
            })
        }
    }
}

impl Drop for Variant {
    fn drop(&mut self) {
        unsafe {
            interface_fn!(variant_destroy)(self.var_sys());
        }
    }
}

impl Default for Variant {
    fn default() -> Self {
        Self::nil()
    }
}

impl PartialEq for Variant {
    fn eq(&self, other: &Self) -> bool {
        Self::evaluate(self, other, VariantOperator::Equal)
            .map(|v| v.to::<bool>())
            .unwrap_or(false) // if there is no defined conversion, then they are non-equal
    }
}

// impl Eq for Variant {}
// impl PartialEq for Variant {
//     fn eq(&self, other: &Self) -> bool {
//         unsafe { builtin_fn!(ope) }
//     }
// }

impl fmt::Display for Variant {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = self.stringify();
        write!(f, "{}", s)
    }
}

impl fmt::Debug for Variant {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // TODO include variant type name
        let s = self.stringify();
        write!(f, "Variant({})", s)
    }
}
