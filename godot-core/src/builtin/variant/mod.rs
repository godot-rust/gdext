/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::builtin::meta::{impl_godot_as_self, ConvertError, FromGodot, ToGodot};
use crate::builtin::{GString, StringName};
use crate::gen::central::VariantDispatch;
use godot_ffi as sys;
use std::{fmt, ptr};
use sys::types::OpaqueVariant;
use sys::{ffi_methods, interface_fn, GodotFfi};

mod impls;

pub use sys::{VariantOperator, VariantType};

/// Godot variant type, able to store a variety of different types.
///
/// While Godot variants do not appear very frequently in Rust due to their lack of compile-time type-safety, they are central to all sorts of
/// dynamic APIs. For example, if you want to call a method on an object based on a string, you will need variants to store arguments and return
/// value.  
///
/// See also [Godot documentation for `Variant`](https://docs.godotengine.org/en/stable/classes/class_variant.html).
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
    pub fn from<T: ToGodot>(value: T) -> Self {
        value.to_variant()
    }

    /// ⚠️ Convert to type `T`, panicking on failure.
    ///
    /// Equivalent to `T::from_variant(&self)`.
    ///
    /// # Panics
    /// When this variant holds a different type.
    pub fn to<T: FromGodot>(&self) -> T {
        T::from_variant(self)
    }

    /// Convert to type `T`, returning `Err` on failure.
    ///
    /// Equivalent to `T::try_from_variant(&self)`.
    pub fn try_to<T: FromGodot>(&self) -> Result<T, ConvertError> {
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
    /// Supports `Object` as well as built-ins with methods (e.g. `Array`, `Vector3`, `GString`, etc).
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

        let result = unsafe {
            Variant::from_var_sys_init_or_init_default(|variant_ptr| {
                interface_fn!(variant_call)(
                    self.var_sys(),
                    method.string_sys(),
                    args_sys.as_ptr(),
                    args_sys.len() as i64,
                    variant_ptr,
                    ptr::addr_of_mut!(error),
                )
            })
        };

        if error.error != sys::GDEXTENSION_CALL_OK {
            let arg_types: Vec<_> = args.iter().map(Variant::get_type).collect();
            sys::panic_call_error(&error, "call", &arg_types);
        }
        result
    }

    /// Evaluates an expression using a GDScript operator.
    ///
    /// Returns the result of the operation, or `None` if the operation is not defined for the given operand types.
    ///
    /// Recommended to be used with fully-qualified call syntax.
    /// For example, `Variant::evaluate(&a, &b, VariantOperator::Add)` is equivalent to `a + b` in GDScript.
    pub fn evaluate(&self, rhs: &Variant, op: VariantOperator) -> Option<Variant> {
        let op_sys = op.sys();
        let mut is_valid = false as u8;

        let result = unsafe {
            Self::from_var_sys_init_or_init_default(|variant_ptr| {
                interface_fn!(variant_evaluate)(
                    op_sys,
                    self.var_sys(),
                    rhs.var_sys(),
                    variant_ptr,
                    ptr::addr_of_mut!(is_valid),
                )
            })
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

    /// Return Godot's string representation of the variant.
    ///
    /// See also `Display` impl.
    #[allow(unused_mut)] // result
    pub fn stringify(&self) -> GString {
        let mut result = GString::new();
        unsafe {
            interface_fn!(variant_stringify)(self.var_sys(), result.string_sys());
        }
        result
    }

    /// Return Godot's hash value for the variant.
    ///
    /// _Godot equivalent : `@GlobalScope.hash()`_
    pub fn hash(&self) -> i64 {
        unsafe { interface_fn!(variant_hash)(self.var_sys()) }
    }

    /// Interpret the `Variant` as `bool`.
    ///
    /// Returns `false` only if the variant's current value is the default value for its type. For example:
    /// - `nil` for the nil type
    /// - `false` for bool
    /// - zero for numeric types
    /// - empty string
    /// - empty container (array, packed array, dictionary)
    /// - default-constructed other builtins (e.g. zero vector, degenerate plane, zero RID, etc...)
    pub fn booleanize(&self) -> bool {
        // See Variant::is_zero(), roughly https://github.com/godotengine/godot/blob/master/core/variant/variant.cpp#L859.

        unsafe { interface_fn!(variant_booleanize)(self.var_sys()) != 0 }
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
    }

    #[doc(hidden)]
    pub unsafe fn from_var_sys_init_default(
        init_fn: impl FnOnce(sys::GDExtensionVariantPtr),
    ) -> Self {
        #[allow(unused_mut)]
        let mut variant = Variant::nil();
        init_fn(variant.var_sys());
        variant
    }

    /// # Safety
    /// See [`GodotFfi::from_sys_init`] and [`GodotFfi::from_sys_init_default`].
    #[cfg(before_api = "4.1")]
    pub unsafe fn from_var_sys_init_or_init_default(
        init_fn: impl FnOnce(sys::GDExtensionVariantPtr),
    ) -> Self {
        Self::from_var_sys_init_default(init_fn)
    }

    /// # Safety
    /// See [`GodotFfi::from_sys_init`] and [`GodotFfi::from_sys_init_default`].
    #[cfg(since_api = "4.1")]
    #[doc(hidden)]
    pub unsafe fn from_var_sys_init_or_init_default(
        init_fn: impl FnOnce(sys::GDExtensionUninitializedVariantPtr),
    ) -> Self {
        Self::from_var_sys_init(init_fn)
    }

    /// # Safety
    /// See [`GodotFfi::from_sys_init`].
    #[doc(hidden)]
    pub unsafe fn from_var_sys_init_result<E>(
        init_fn: impl FnOnce(sys::GDExtensionUninitializedVariantPtr) -> Result<(), E>,
    ) -> Result<Self, E> {
        // Relies on current macro expansion of from_var_sys_init() having a certain implementation.

        let mut raw = std::mem::MaybeUninit::<OpaqueVariant>::uninit();

        let var_uninit_ptr =
            raw.as_mut_ptr() as <sys::GDExtensionVariantPtr as ::godot_ffi::AsUninit>::Ptr;

        init_fn(var_uninit_ptr).map(|_err| Self::from_opaque(raw.assume_init()))
    }

    #[doc(hidden)]
    pub fn var_sys_const(&self) -> sys::GDExtensionConstVariantPtr {
        sys::to_const_ptr(self.var_sys())
    }

    /// Converts to variant pointer; can be a null pointer.
    pub(crate) fn ptr_from_sys(variant_ptr: sys::GDExtensionConstVariantPtr) -> *const Variant {
        variant_ptr as *const Variant
    }

    /// # Safety
    /// `variant_ptr_array` must be a valid pointer to an array of `length` variant pointers.
    /// The caller is responsible of keeping the backing storage alive while the unbounded references exist.
    pub(crate) unsafe fn unbounded_refs_from_sys<'a>(
        variant_ptr_array: *const sys::GDExtensionConstVariantPtr,
        length: usize,
    ) -> &'a [&'a Variant] {
        // Godot may pass null to signal "no arguments" (e.g. in custom callables).
        if variant_ptr_array.is_null() {
            debug_assert_eq!(
                length, 0,
                "Variant::unbounded_refs_from_sys(): pointer is null but length is not 0"
            );
            return &[];
        }

        let variant_ptr_array: &'a [sys::GDExtensionConstVariantPtr] =
            std::slice::from_raw_parts(variant_ptr_array, length);

        // SAFETY: raw pointers and references have the same memory layout.
        // See https://doc.rust-lang.org/reference/type-layout.html#pointers-and-references-layout.
        unsafe { std::mem::transmute(variant_ptr_array) }
    }

    /// Converts to variant mut pointer; can be a null pointer.
    pub(crate) fn ptr_from_sys_mut(variant_ptr: sys::GDExtensionVariantPtr) -> *mut Variant {
        variant_ptr as *mut Variant
    }

    /// Move `self` into a system pointer. This transfers ownership and thus does not call the destructor.
    ///
    /// # Safety
    /// `dst` must be a pointer to a [`Variant`] which is suitable for ffi with Godot.
    pub(crate) unsafe fn move_var_ptr(self, dst: sys::GDExtensionVariantPtr) {
        self.move_return_ptr(dst as *mut _, sys::PtrcallType::Standard);
    }
}

// SAFETY:
// `from_opaque` properly initializes a dereferenced pointer to an `OpaqueVariant`.
// `std::mem::swap` is sufficient for returning a value.
unsafe impl GodotFfi for Variant {
    fn variant_type() -> sys::VariantType {
        sys::VariantType::Nil
    }

    ffi_methods! { type sys::GDExtensionTypePtr = *mut Opaque; .. }

    unsafe fn from_sys_init_default(init_fn: impl FnOnce(sys::GDExtensionTypePtr)) -> Self {
        let mut result = Self::default();
        init_fn(result.sys_mut());
        result
    }
}

impl_godot_as_self!(Variant);

impl Default for Variant {
    fn default() -> Self {
        unsafe {
            Self::from_var_sys_init(|variant_ptr| {
                interface_fn!(variant_new_nil)(variant_ptr);
            })
        }
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

// Variant is not Eq because it can contain floats and other types composed of floats.
impl PartialEq for Variant {
    fn eq(&self, other: &Self) -> bool {
        Self::evaluate(self, other, VariantOperator::Equal)
            .map(|v| v.to::<bool>())
            .unwrap_or(false) // if there is no defined conversion, then they are non-equal
    }
}

impl fmt::Display for Variant {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = self.stringify();
        write!(f, "{s}")
    }
}

impl fmt::Debug for Variant {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        VariantDispatch::from_variant(self).fmt(f)
    }
}
