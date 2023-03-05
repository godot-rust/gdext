/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::builtin::{GodotString, StringName};
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
        Self::default()
    }

    /// Create a variant holding a non-nil value.
    ///
    /// Equivalent to `value.to_variant()`.
    pub fn from<T: ToVariant>(value: T) -> Self {
        value.to_variant()
    }

    /// ⚠️ Convert to type `T`, panicking on failure.
    ///
    /// Equivalent to `T::from_variant(&self)`.
    ///
    /// # Panics
    /// When this variant holds a different type.
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
    ///
    /// See also [`Self::get_type`].
    pub fn is_nil(&self) -> bool {
        // Use get_type() rather than sys_type(), to also cover nullptr OBJECT as NIL
        self.get_type() == VariantType::Nil
    }

    /// Returns the type that is currently held by this variant.
    ///
    /// If this variant holds a type `Object` but no instance (represented as a null object pointer), then `Nil` will be returned for
    /// consistency. This may deviate from Godot behavior -- for example, calling `Node::get_node_or_null()` with an invalid
    /// path returns a variant that has type `Object` but acts like `Nil` for all practical purposes.
    pub fn get_type(&self) -> VariantType {
        let sys_type = self.sys_type();

        // There is a special case when the Variant has type OBJECT, but the Object* is null.
        let is_null_object = if sys_type == sys::GDEXTENSION_VARIANT_TYPE_OBJECT {
            // SAFETY: we checked that the raw type is OBJECT, so we can interpret the type-ptr as address of an object-ptr.
            let object_ptr = unsafe {
                crate::obj::raw_object_init(|type_ptr| {
                    let converter = sys::builtin_fn!(object_from_variant);
                    converter(type_ptr, self.var_sys());
                })
            };

            object_ptr.is_null()
        } else {
            false
        };

        if is_null_object {
            VariantType::Nil
        } else {
            VariantType::from_sys(sys_type)
        }
    }

    /// ⚠️ Calls the specified `method` with the given `args`.
    ///
    /// Supports `Object` as well as built-ins with methods (e.g. `Array`, `Vector3`, `GodotString`, etc).
    ///
    /// # Panics
    /// * If `self` is not a variant type which supports method calls.
    /// * If the method does not exist or the signature is not compatible with the passed arguments.
    /// * If the call causes an error.
    #[inline]
    pub fn call(&self, method: impl Into<StringName>, args: &[Variant]) -> Variant {
        self.call_inner(method.into(), args)
    }

    fn call_inner(&self, method: StringName, args: &[Variant]) -> Variant {
        let args_sys: Vec<_> = args.iter().map(|v| v.var_sys_const()).collect();
        let mut error = sys::default_call_error();

        #[allow(unused_mut)]
        let mut result = Variant::nil();

        unsafe {
            interface_fn!(variant_call)(
                self.var_sys(),
                method.string_sys(),
                args_sys.as_ptr(),
                args_sys.len() as i64,
                result.var_sys(),
                ptr::addr_of_mut!(error),
            )
        };

        sys::panic_on_call_error(&error);
        result
    }

    pub fn evaluate(&self, rhs: &Variant, op: VariantOperator) -> Option<Variant> {
        let op_sys = op.sys();
        let mut is_valid = false as u8;

        #[allow(unused_mut)]
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
    pub(crate) fn stringify(&self) -> GodotString {
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

    pub(crate) fn ptr_from_sys(variant_ptr: sys::GDExtensionVariantPtr) -> *const Variant {
        assert!(!variant_ptr.is_null(), "ptr_from_sys: null variant pointer");
        variant_ptr as *const Variant
    }

    pub(crate) fn ptr_from_sys_mut(variant_ptr: sys::GDExtensionVariantPtr) -> *mut Variant {
        assert!(
            !variant_ptr.is_null(),
            "ptr_from_sys_mut: null variant pointer"
        );
        variant_ptr as *mut Variant
    }
}

impl GodotFfi for Variant {
    ffi_methods! { type sys::GDExtensionTypePtr = *mut Opaque; .. }

    unsafe fn from_sys_init_default(init_fn: impl FnOnce(sys::GDExtensionTypePtr)) -> Self {
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
        unsafe {
            Self::from_var_sys_init(|variant_ptr| {
                interface_fn!(variant_new_nil)(variant_ptr);
            })
        }
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
        write!(f, "{s}")
    }
}

impl fmt::Debug for Variant {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // TODO include variant type name
        let s = self.stringify();
        write!(f, "Variant({s})")
    }
}
