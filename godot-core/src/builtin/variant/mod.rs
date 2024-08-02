/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::builtin::{GString, StringName, VariantDispatch, VariantOperator, VariantType};
use crate::meta::error::ConvertError;
use crate::meta::{ArrayElement, FromGodot, ToGodot};
use godot_ffi as sys;
use std::{fmt, ptr};
use sys::{ffi_methods, interface_fn, GodotFfi};

mod impls;

/// Godot variant type, able to store a variety of different types.
///
/// While Godot variants do not appear very frequently in Rust due to their lack of compile-time type-safety, they are central to all sorts of
/// dynamic APIs. For example, if you want to call a method on an object based on a string, you will need variants to store arguments and return
/// value.  
///
/// # Conversions
///
/// For type conversions, please read the [`godot::meta` module docs][crate::meta].
///
/// # Godot docs
///
/// [`Variant` (stable)](https://docs.godotengine.org/en/stable/classes/class_variant.html)
// We rely on the layout of `Variant` being the same as Godot's layout in `borrow_slice` and `borrow_slice_mut`.
#[repr(transparent)]
pub struct Variant {
    _opaque: sys::types::OpaqueVariant,
}

impl Variant {
    /// Create an empty variant (`null` value in GDScript).
    ///
    /// If a Godot engine API accepts object (not variant) parameters and you'd like to pass `null`, use
    /// [`Gd::null_arg()`][crate::obj::Gd::null_arg] instead.
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
        self.get_type() == VariantType::NIL
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
                    converter(type_ptr, sys::SysPtr::force_mut(self.var_sys()));
                })
            };

            object_ptr.is_null()
        } else {
            false
        };

        if is_null_object {
            VariantType::NIL
        } else {
            VariantType::from_sys(sys_type)
        }
    }

    /// ⚠️ Calls the specified `method` with the given `args`.
    ///
    /// Supports `Object` as well as built-ins with methods (e.g. `Array`, `Vector3`, `GString`, etc.).
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
        let args_sys: Vec<_> = args.iter().map(|v| v.var_sys()).collect();
        let mut error = sys::default_call_error();

        let result = unsafe {
            Variant::new_with_var_uninit(|variant_ptr| {
                interface_fn!(variant_call)(
                    sys::SysPtr::force_mut(self.var_sys()),
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
        use crate::obj::EngineEnum;

        let op_sys = op.ord() as sys::GDExtensionVariantOperator;
        let mut is_valid = false as u8;

        let result = unsafe {
            Self::new_with_var_uninit(|variant_ptr| {
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
            interface_fn!(variant_stringify)(self.var_sys(), result.string_sys_mut());
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

    // Conversions from/to Godot C++ `Variant*` pointers
    ffi_methods! {
        type sys::GDExtensionVariantPtr = *mut Self;

        fn new_from_var_sys = new_from_sys;
        fn new_with_var_uninit = new_with_uninit;
        fn new_with_var_init = new_with_init;
        fn var_sys = sys;
        fn var_sys_mut = sys_mut;
    }
}

// All manually implemented unsafe functions on `Variant`.
// Deny `unsafe_op_in_unsafe_fn` so we don't forget to check safety invariants.
#[doc(hidden)]
#[deny(unsafe_op_in_unsafe_fn)]
impl Variant {
    /// Moves this variant into a variant sys pointer. This is the same as using [`GodotFfi::move_return_ptr`].
    ///
    /// # Safety
    ///
    /// `dst` must be a valid variant pointer.
    pub(crate) unsafe fn move_into_var_ptr(self, dst: sys::GDExtensionVariantPtr) {
        let dst: sys::GDExtensionTypePtr = dst.cast();
        // SAFETY: `dst` is a valid Variant pointer. Additionally `Variant` doesn't behave differently for `Standard` and `Virtual`
        // pointer calls.
        unsafe {
            self.move_return_ptr(dst, sys::PtrcallType::Standard);
        }
    }

    /// Fallible construction of a `Variant` using a fallible initialization function.
    ///
    /// # Safety
    ///
    /// If `init_fn` returns `Ok(())`, then it must have initialized the pointer passed to it in accordance with [`GodotFfi::new_with_uninit`].
    #[doc(hidden)]
    pub unsafe fn new_with_var_uninit_result<E>(
        init_fn: impl FnOnce(sys::GDExtensionUninitializedVariantPtr) -> Result<(), E>,
    ) -> Result<Self, E> {
        // Relies on current macro expansion of from_var_sys_init() having a certain implementation.

        let mut raw = std::mem::MaybeUninit::<Variant>::uninit();

        let var_uninit_ptr =
            raw.as_mut_ptr() as <sys::GDExtensionVariantPtr as sys::SysPtr>::Uninit;

        // SAFETY: `map` only runs the provided closure for the `Ok(())` variant, in which case `raw` has definitely been initialized.
        init_fn(var_uninit_ptr).map(|_success| unsafe { raw.assume_init() })
    }

    /// Convert a `Variant` sys pointer to a reference to a `Variant`.
    ///
    /// # Safety
    ///
    /// `ptr` must point to a live `Variant` for the duration of `'a`.
    pub(crate) unsafe fn borrow_var_sys<'a>(ptr: sys::GDExtensionConstVariantPtr) -> &'a Variant {
        sys::static_assert_eq_size_align!(Variant, sys::types::OpaqueVariant);

        // SAFETY: `ptr` is a pointer to a live `Variant` for the duration of `'a`.
        unsafe { &*(ptr.cast::<Variant>()) }
    }

    /// Convert an array of `Variant` sys pointers to a slice of `Variant` references all with unbounded lifetimes.
    ///
    /// # Safety
    ///
    /// Either `variant_ptr_array` is null, or it must be safe to call [`std::slice::from_raw_parts`] with
    /// `variant_ptr_array` cast to `*const &'a Variant` and `length`.
    pub(crate) unsafe fn borrow_ref_slice<'a>(
        variant_ptr_array: *const sys::GDExtensionConstVariantPtr,
        length: usize,
    ) -> &'a [&'a Variant] {
        sys::static_assert_eq_size_align!(Variant, sys::types::OpaqueVariant);
        // Godot may pass null to signal "no arguments" (e.g. in custom callables).
        if variant_ptr_array.is_null() {
            debug_assert_eq!(
                length, 0,
                "Variant::unbounded_refs_from_sys(): pointer is null but length is not 0"
            );
            return &[];
        }

        // Note: Raw pointers and references have the same memory layout.
        // See https://doc.rust-lang.org/reference/type-layout.html#pointers-and-references-layout.
        let variant_ptr_array = variant_ptr_array.cast::<&Variant>();

        // SAFETY: `variant_ptr_array` isn't null so it is safe to call `from_raw_parts` on the pointer cast to `*const &Variant`.
        unsafe { std::slice::from_raw_parts(variant_ptr_array, length) }
    }

    /// Convert an array of `Variant` sys pointers to a slice with unbounded lifetime.
    ///
    /// # Safety
    ///
    /// Either `variant_array` is null, or it must be safe to call [`std::slice::from_raw_parts`] with
    /// `variant_array` cast to `*const Variant` and `length`.
    pub(crate) unsafe fn borrow_slice<'a>(
        variant_array: sys::GDExtensionConstVariantPtr,
        length: usize,
    ) -> &'a [Variant] {
        sys::static_assert_eq_size_align!(Variant, sys::types::OpaqueVariant);

        // Godot may pass null to signal "no arguments" (e.g. in custom callables).
        if variant_array.is_null() {
            debug_assert_eq!(
                length, 0,
                "Variant::unbounded_refs_from_sys(): pointer is null but length is not 0"
            );
            return &[];
        }

        let variant_array = variant_array.cast::<Variant>();

        // SAFETY: `variant_array` isn't null so it is safe to call `from_raw_parts` on the pointer cast to `*const Variant`.
        unsafe { std::slice::from_raw_parts(variant_array, length) }
    }

    /// Convert an array of `Variant` sys pointers to a mutable slice with unbounded lifetime.
    ///
    /// # Safety
    ///
    /// Either `variant_array` is null, or it must be safe to call [`std::slice::from_raw_parts_mut`] with
    /// `variant_array` cast to `*mut Variant` and `length`.
    pub(crate) unsafe fn borrow_slice_mut<'a>(
        variant_array: sys::GDExtensionVariantPtr,
        length: usize,
    ) -> &'a mut [Variant] {
        sys::static_assert_eq_size_align!(Variant, sys::types::OpaqueVariant);

        // Godot may pass null to signal "no arguments" (e.g. in custom callables).
        if variant_array.is_null() {
            debug_assert_eq!(
                length, 0,
                "Variant::unbounded_refs_from_sys(): pointer is null but length is not 0"
            );
            return &mut [];
        }

        let variant_array = variant_array.cast::<Variant>();

        // SAFETY: `variant_array` isn't null so it is safe to call `from_raw_parts_mut` on the pointer cast to `*mut Variant`.
        unsafe { std::slice::from_raw_parts_mut(variant_array, length) }
    }

    /// Consumes self and turns it into a sys-ptr, should be used together with [`from_owned_var_sys`](Self::from_owned_var_sys).
    ///
    /// This will leak memory unless `from_owned_var_sys` is called on the returned pointer.
    pub(crate) fn into_owned_var_sys(self) -> sys::GDExtensionVariantPtr {
        sys::static_assert_eq_size_align!(Variant, sys::types::OpaqueVariant);

        let leaked = Box::into_raw(Box::new(self));
        leaked.cast()
    }

    /// Creates a `Variant` from a sys-ptr without incrementing the refcount.
    ///
    /// # Safety
    ///
    /// * Must only be used on a pointer returned from a call to [`into_owned_var_sys`](Self::into_owned_var_sys).
    /// * Must not be called more than once on the same pointer.
    #[deny(unsafe_op_in_unsafe_fn)]
    pub(crate) unsafe fn from_owned_var_sys(ptr: sys::GDExtensionVariantPtr) -> Self {
        sys::static_assert_eq_size_align!(Variant, sys::types::OpaqueVariant);

        let ptr = ptr.cast::<Self>();

        // SAFETY: `ptr` was returned from a call to `into_owned_var_sys`, which means it was created by a call to
        // `Box::into_raw`, thus we can use `Box::from_raw` here. Additionally, this is only called once on this pointer.
        let boxed = unsafe { Box::from_raw(ptr) };
        *boxed
    }
}

impl ArrayElement for Variant {}

// SAFETY:
// `from_opaque` properly initializes a dereferenced pointer to an `OpaqueVariant`.
// `std::mem::swap` is sufficient for returning a value.
unsafe impl GodotFfi for Variant {
    fn variant_type() -> VariantType {
        VariantType::NIL
    }

    ffi_methods! { type sys::GDExtensionTypePtr = *mut Self; .. }
}

crate::meta::impl_godot_as_self!(Variant);

impl Default for Variant {
    fn default() -> Self {
        unsafe {
            Self::new_with_var_uninit(|variant_ptr| {
                interface_fn!(variant_new_nil)(variant_ptr);
            })
        }
    }
}

impl Clone for Variant {
    fn clone(&self) -> Self {
        unsafe {
            Self::new_with_var_uninit(|variant_ptr| {
                interface_fn!(variant_new_copy)(variant_ptr, self.var_sys());
            })
        }
    }
}

impl Drop for Variant {
    fn drop(&mut self) {
        unsafe {
            interface_fn!(variant_destroy)(self.var_sys_mut());
        }
    }
}

// Variant is not Eq because it can contain floats and other types composed of floats.
impl PartialEq for Variant {
    fn eq(&self, other: &Self) -> bool {
        Self::evaluate(self, other, VariantOperator::EQUAL)
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
