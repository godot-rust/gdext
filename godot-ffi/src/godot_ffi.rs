/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::marker::PhantomData;

use crate as sys;
use crate::VariantType;

/// Types that can directly and fully represent some Godot type.
///
/// Adds methods to convert from and to Godot FFI pointers.
/// See [crate::ffi_methods] for ergonomic implementation.
///
/// # Safety
///
/// - [`from_arg_ptr`](GodotFfi::from_arg_ptr) and [`move_return_ptr`](GodotFfi::move_return_ptr)
/// must properly initialize and clean up values given the [`PtrcallType`] provided by the caller.
///
/// - [`new_with_uninit`](GodotFfi::new_with_uninit) must call `init_fn` with a pointer to a *new*
/// [allocated object](https://doc.rust-lang.org/std/ptr/index.html#safety).
///
/// - [`new_with_init`](GodotFfi::new_with_init) must call `init_fn` with a reference to a *new* value.
#[doc(hidden)] // shows up in implementors otherwise
pub unsafe trait GodotFfi {
    #[doc(hidden)]
    const VARIANT_TYPE: ExtVariantType;

    #[doc(hidden)]
    fn default_param_metadata() -> sys::GDExtensionClassMethodArgumentMetadata {
        sys::GDEXTENSION_METHOD_ARGUMENT_METADATA_NONE
    }

    /// Construct from Godot opaque pointer.
    ///
    /// This will increment reference counts if the type is reference counted. If you need to avoid this, then a `borrow_sys` associated
    /// function should usually be used. That function that takes a sys-pointer and returns it as a `&Self` reference. This must be manually
    /// implemented for each relevant type, as not all types can be borrowed like this.
    ///
    /// # Safety
    /// `ptr` must be a valid _type ptr_: it must follow Godot's convention to encode `Self`,
    /// which is different depending on the type.
    #[doc(hidden)]
    unsafe fn new_from_sys(ptr: sys::GDExtensionConstTypePtr) -> Self;

    /// Construct uninitialized opaque data, then initialize it with `init_fn` function.
    ///
    /// # Safety
    /// `init_fn` must be a function that correctly handles a (possibly-uninitialized) _type ptr_.
    #[doc(hidden)]
    unsafe fn new_with_uninit(init_fn: impl FnOnce(sys::GDExtensionUninitializedTypePtr)) -> Self;

    /// Like [`new_with_uninit`](GodotFfi::new_with_uninit), but pre-initializes the sys pointer to a default instance (usually
    /// [`Default::default()`]) before calling `init_fn`.
    ///
    /// Some FFI functions in Godot expect a pre-existing instance at the destination pointer, e.g. CoW/ref-counted
    /// builtin types like `Array`, `Dictionary`, `String`, `StringName`.
    ///
    /// # Note
    ///
    /// This does call `init_fn` with a `&mut Self` reference, but in some cases initializing the reference to a more appropriate
    /// value may involve violating the value's safety invariant. In those cases it is important to ensure that this violation isn't
    /// leaked to user-code.
    #[doc(hidden)]
    unsafe fn new_with_init(init_fn: impl FnOnce(sys::GDExtensionTypePtr)) -> Self;

    /// Return Godot opaque pointer, for an immutable operation.
    #[doc(hidden)]
    fn sys(&self) -> sys::GDExtensionConstTypePtr;

    /// Return Godot opaque pointer, for a mutable operation.
    #[doc(hidden)]
    fn sys_mut(&mut self) -> sys::GDExtensionTypePtr;

    #[doc(hidden)]
    fn as_arg_ptr(&self) -> sys::GDExtensionConstTypePtr {
        self.sys()
    }

    /// Construct from a pointer to an argument in a call.
    ///
    /// # Safety
    /// * `ptr` must be a valid _type ptr_: it must follow Godot's convention to encode `Self`,
    ///   which is different depending on the type.
    ///
    /// * `ptr` must encode `Self` according to the given `call_type`'s encoding of argument values.
    #[doc(hidden)]
    unsafe fn from_arg_ptr(ptr: sys::GDExtensionTypePtr, call_type: PtrcallType) -> Self;

    /// Move self into the pointer in pointer `dst`, dropping what is already in `dst.
    ///
    /// # Safety
    /// * `dst` must be a valid _type ptr_: it must follow Godot's convention to encode `Self`,
    ///    which is different depending on the type.
    ///
    /// * `dst` must be able to accept a value of type `Self` encoded according to the given
    ///   `call_type`'s encoding of return values.
    #[doc(hidden)]
    unsafe fn move_return_ptr(self, dst: sys::GDExtensionTypePtr, call_type: PtrcallType);
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

/// Types that can represent null-values.
///
/// Used to blanket implement various conversions over `Option<T>`.
///
/// This is currently only implemented for `RawGd`.
// TODO: Consider implementing for `Variant`.
pub trait GodotNullableFfi: Sized + GodotFfi {
    fn null() -> Self;

    fn is_null(&self) -> bool;

    fn flatten_option(opt: Option<Self>) -> Self {
        opt.unwrap_or_else(Self::null)
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

/// Variant type that differentiates between `Variant` and `NIL` types.
#[doc(hidden)]
#[derive(Copy, Clone, Eq, PartialEq)]
pub enum ExtVariantType {
    /// The type `Variant` itself.
    Variant,

    /// A Godot built-in type. `NIL` means actually nil (unit type `()` in Rust), not `Variant`.
    Concrete(VariantType),
}

impl ExtVariantType {
    /// Returns the variant type as Godot `VariantType`, using `NIL` for the `Variant` case.
    pub const fn variant_as_nil(&self) -> VariantType {
        match self {
            ExtVariantType::Variant => VariantType::NIL,
            ExtVariantType::Concrete(variant_type) => *variant_type,
        }
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

/// An indication of what type of pointer call is being made.
#[derive(Default, Copy, Clone, Eq, PartialEq, Debug)]
#[doc(hidden)]
pub enum PtrcallType {
    /// Standard pointer call.
    ///
    /// In a standard ptrcall, every argument is passed in as a pointer to a value of that type, and the
    /// return value must be moved into the return pointer.
    #[default]
    Standard,

    /// Virtual pointer call.
    ///
    /// A virtual call behaves like [`PtrcallType::Standard`], except for Objects.
    ///
    /// Objects that do not inherit from `RefCounted` are passed in as `Object**`
    /// (`*mut GDExtensionObjectPtr` in GDExtension terms), and objects that inherit from
    /// `RefCounted` are passed in as `Ref<T>*` (`GDExtensionRefPtr` in GDExtension
    /// terms) and returned as `Ref<T>` objects in Godot.
    ///
    /// To get a `GDExtensionObjectPtr` from a `GDExtensionRefPtr`, you must use `ref_get_object`, and to
    /// set a `GDExtensionRefPtr` to some object, you must use `ref_set_object`.
    ///
    /// See also <https://github.com/godotengine/godot-cpp/issues/954>.
    Virtual,
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Macros to choose a certain implementation of `GodotFfi` trait for GDExtensionTypePtr;
// or a freestanding `impl` for concrete sys pointers such as GDExtensionObjectPtr.
// See doc comment of `ffi_methods!` for information

// TODO: explicitly document safety invariants.
#[macro_export]
#[doc(hidden)]
macro_rules! ffi_methods_one {
    // type $Ptr = *mut Opaque
    (OpaquePtr $Ptr:ty; $( #[$attr:meta] )? $vis:vis $new_from_sys:ident = new_from_sys) => {
        $( #[$attr] )? $vis
        unsafe fn $new_from_sys(ptr: <$Ptr as $crate::SysPtr>::Const) -> Self {
            // TODO: Directly use copy constructors here?
            let opaque = std::ptr::read(ptr.cast());
            let new = Self::from_opaque(opaque);
            std::mem::forget(new.clone());
            new
        }
    };
    (OpaquePtr $Ptr:ty; $( #[$attr:meta] )? $vis:vis $new_with_uninit:ident = new_with_uninit) => {
        $( #[$attr] )? $vis
        unsafe fn $new_with_uninit(init: impl FnOnce(<$Ptr as $crate::SysPtr>::Uninit)) -> Self {
            let mut raw = std::mem::MaybeUninit::uninit();
            init(raw.as_mut_ptr() as *mut _);

            Self::from_opaque(raw.assume_init())
        }
    };
    (OpaquePtr $Ptr:ty; $( #[$attr:meta] )? $vis:vis $new_with_init:ident = new_with_init) => {
        $( #[$attr] )? $vis
        unsafe fn $new_with_init(init: impl FnOnce($Ptr)) -> Self {
            let mut default = Self::default();
            init(default.sys_mut().cast());
            default
        }
    };
    (OpaquePtr $Ptr:ty; $( #[$attr:meta] )? $vis:vis $sys:ident = sys) => {
        $( #[$attr] )? $vis
        fn $sys(&self) -> <$Ptr as $crate::SysPtr>::Const {
            std::ptr::from_ref(&self.opaque).cast()
        }
    };
    (OpaquePtr $Ptr:ty; $( #[$attr:meta] )? $vis:vis $sys_mut:ident = sys_mut) => {
        $( #[$attr] )? $vis
        fn $sys_mut(&mut self) -> $Ptr {
            std::ptr::from_mut(&mut self.opaque).cast()
        }
    };
    (OpaquePtr $Ptr:ty; $( #[$attr:meta] )? $vis:vis $from_arg_ptr:ident = from_arg_ptr) => {
        $( #[$attr] )? $vis
        unsafe fn $from_arg_ptr(ptr: $Ptr, _call_type: $crate::PtrcallType) -> Self {
            Self::new_from_sys(ptr.cast())
        }
    };
    (OpaquePtr $Ptr:ty; $( #[$attr:meta] )? $vis:vis $move_return_ptr:ident = move_return_ptr) => {
        $( #[$attr] )? $vis
        unsafe fn $move_return_ptr(mut self, dst: $Ptr, _call_type: $crate::PtrcallType) {
            std::ptr::swap(dst.cast(), std::ptr::addr_of_mut!(self.opaque))
        }
    };

    // type $Ptr = *mut Self
    (SelfPtr $Ptr:ty; $( #[$attr:meta] )? $vis:vis $new_from_sys:ident = new_from_sys) => {
        $( #[$attr] )? $vis
        unsafe fn $new_from_sys(ptr: <$Ptr as $crate::SysPtr>::Const) -> Self {
            let borrowed = &*ptr.cast::<Self>();
            borrowed.clone()
        }
    };
    (SelfPtr $Ptr:ty; $( #[$attr:meta] )? $vis:vis $new_with_uninit:ident = new_with_uninit) => {
        $( #[$attr] )? $vis
        unsafe fn $new_with_uninit(init: impl FnOnce(<$Ptr as $crate::SysPtr>::Uninit)) -> Self {
            let mut raw = std::mem::MaybeUninit::<Self>::uninit();
            init(raw.as_mut_ptr().cast());

            raw.assume_init()
        }
    };
    (SelfPtr $Ptr:ty; $( #[$attr:meta] )? $vis:vis $new_with_init:ident = new_with_init) => {
        $( #[$attr] )? $vis
        unsafe fn $new_with_init(init: impl FnOnce($Ptr)) -> Self {
            let mut default = Self::default();
            init(default.sys_mut().cast());
            default
        }
    };
    (SelfPtr $Ptr:ty; $( #[$attr:meta] )? $vis:vis $sys:ident = sys) => {
        $( #[$attr] )? $vis
        fn $sys(&self) -> <$Ptr as $crate::SysPtr>::Const {
            std::ptr::from_ref(self).cast()
        }
    };
    (SelfPtr $Ptr:ty; $( #[$attr:meta] )? $vis:vis $sys_mut:ident = sys_mut) => {
        $( #[$attr] )? $vis
        fn $sys_mut(&mut self) -> $Ptr {
            std::ptr::from_mut(self).cast()
        }
    };
    (SelfPtr $Ptr:ty; $( #[$attr:meta] )? $vis:vis $from_arg_ptr:ident = from_arg_ptr) => {
        $( #[$attr] )? $vis
        unsafe fn $from_arg_ptr(ptr: $Ptr, _call_type: $crate::PtrcallType) -> Self {
            Self::new_from_sys(ptr.cast())
        }
    };
    (SelfPtr $Ptr:ty; $( #[$attr:meta] )? $vis:vis $move_return_ptr:ident = move_return_ptr) => {
        $( #[$attr] )? $vis
        unsafe fn $move_return_ptr(self, dst: $Ptr, _call_type: $crate::PtrcallType) {
            *(dst.cast::<Self>()) = self
        }
    };
}

#[macro_export]
#[doc(hidden)]
macro_rules! ffi_methods_rest {
    ( // impl T: each method has a custom name and is annotated with 'pub'
        $Impl:ident $Ptr:ty; $( fn $user_fn:ident = $sys_fn:ident; )*
    ) => {
        $( $crate::ffi_methods_one!($Impl $Ptr; #[doc(hidden)] pub $user_fn = $sys_fn); )*
    };

    ( // impl GodotFfi for T: methods have given names, no 'pub' needed
        $Impl:ident $Ptr:ty; $( fn $sys_fn:ident; )*
    ) => {
        $( $crate::ffi_methods_one!($Impl $Ptr; $sys_fn = $sys_fn); )*
    };

    ( // impl GodotFfi for T (default all 5)
        $Impl:ident $Ptr:ty; ..
    ) => {
        $crate::ffi_methods_one!($Impl $Ptr; new_from_sys = new_from_sys);
        $crate::ffi_methods_one!($Impl $Ptr; new_with_uninit = new_with_uninit);
        $crate::ffi_methods_one!($Impl $Ptr; new_with_init = new_with_init);
        $crate::ffi_methods_one!($Impl $Ptr; sys = sys);
        $crate::ffi_methods_one!($Impl $Ptr; sys_mut = sys_mut);
        $crate::ffi_methods_one!($Impl $Ptr; from_arg_ptr = from_arg_ptr);
        $crate::ffi_methods_one!($Impl $Ptr; move_return_ptr = move_return_ptr);
    };
}

/// Provides "sys" style methods for FFI and ptrcall integration with Godot.
/// The generated implementations follow one of three patterns:
///
/// * `*mut Opaque`<br>
///   Implements FFI methods for a type with `Opaque` data that stores a value type (e.g. Vector2).
///   The **address of** the `Opaque` field is used as the sys pointer.
///   Expects a `from_opaque()` constructor and a `opaque` field.
///
/// * `*mut Self`<br>
///   Implements FFI methods for a type implemented with standard Rust fields (not opaque).
///   The address of `Self` is directly reinterpreted as the sys pointer.
///   The size of the corresponding sys type (the `N` in `Opaque*<N>`) must not be bigger than `size_of::<Self>()`.
///   This cannot be checked easily, because Self cannot be used in size_of(). There would of course be workarounds.
///
/// Using this macro as a complete implementation for [`GodotFfi`] is sound only when:
///
/// ## Using `*mut Opaque`
///
/// Turning ptrcall arguments into a value is simply calling `from_opaque` on the
/// dereferenced argument pointer.
/// Returning a value from a pointer call is simply calling [`std::ptr::swap`] on the return pointer
/// and the address to the `opaque` field.
///  
/// ## Using `*mut Self`
///
/// Turning ptrcall arguments into a value is a dereferencing.
/// Returning a value from a pointer call is `*ret_ptr = value`.
#[macro_export]
macro_rules! ffi_methods {
    ( // Sys pointer = address of opaque
        type $Ptr:ty = *mut Opaque;
        $( $rest:tt )*
    ) => {
        $crate::ffi_methods_rest!(OpaquePtr $Ptr; $($rest)*);
    };

    ( // Sys pointer = address of self
        type $Ptr:ty = *mut Self;
        $( $rest:tt )*
    ) => {
        $crate::ffi_methods_rest!(SelfPtr $Ptr; $($rest)*);
    };
}

/// An error representing a failure to convert some value of type `From` into the type `Into`.
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub struct PrimitiveConversionError<From, Into> {
    from: From,
    into_ty: PhantomData<Into>,
}

impl<From, Into> PrimitiveConversionError<From, Into> {
    pub fn new(from: From) -> Self {
        Self {
            from,
            into_ty: PhantomData,
        }
    }
}

impl<From, Into> std::fmt::Display for PrimitiveConversionError<From, Into>
where
    From: std::fmt::Display,
    Into: std::fmt::Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "could not convert {} to type {}",
            self.from,
            std::any::type_name::<Into>()
        )
    }
}

impl<From, Into> std::error::Error for PrimitiveConversionError<From, Into>
where
    From: std::fmt::Display + std::fmt::Debug,
    Into: std::fmt::Display + std::fmt::Debug,
{
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Implementation for common types (needs to be this crate due to orphan rule)
mod scalars {
    use super::{ExtVariantType, GodotFfi};
    use crate as sys;

    /*
    macro_rules! impl_godot_marshalling {
        ($T:ty) => {
            // SAFETY:
            // This type is represented as `Self` in Godot, so `*mut Self` is sound.
            unsafe impl GodotFfi for $T {
                ffi_methods! { type sys::GDExtensionTypePtr = *mut Self; .. }
            }
        };

        ($T:ty as $Via:ty) => {
            // implicit bounds:
            //    T: TryFrom<Via>, Copy
            //    Via: TryFrom<T>, GodotFfi
            impl GodotFuncMarshal for $T {
                type Via = $Via;
                type FromViaError = PrimitiveConversionError<$Via, Self>;
                type IntoViaError = PrimitiveConversionError<Self, $Via>;

                fn try_from_via(via: Self::Via) -> Result<Self, Self::FromViaError> {
                    Self::try_from(via).map_err(|_| PrimitiveConversionError::new(via))
                }

                fn try_into_via(self) -> Result<Self::Via, Self::IntoViaError> {
                    <$Via>::try_from(self).map_err(|_| PrimitiveConversionError::new(self))
                }
            }
        };

        ($T:ty as $Via:ty; lossy) => {
            // implicit bounds:
            //    T: TryFrom<Via>, Copy
            //    Via: TryFrom<T>, GodotFfi
            impl GodotFuncMarshal for $T {
                type Via = $Via;
                type FromViaError = Infallible;
                type IntoViaError = Infallible;

                #[inline]
                fn try_from_via(via: Self::Via) -> Result<Self, Self::FromViaError> {
                    Ok(via as Self)
                }

                #[inline]
                fn try_into_via(self) -> Result<Self::Via, Self::IntoViaError> {
                    Ok(self as $Via)
                }
            }
        };
    }
    */
    unsafe impl GodotFfi for bool {
        const VARIANT_TYPE: ExtVariantType = ExtVariantType::Concrete(sys::VariantType::BOOL);

        ffi_methods! { type sys::GDExtensionTypePtr = *mut Self; .. }
    }

    unsafe impl GodotFfi for i64 {
        const VARIANT_TYPE: ExtVariantType = ExtVariantType::Concrete(sys::VariantType::INT);

        fn default_param_metadata() -> sys::GDExtensionClassMethodArgumentMetadata {
            sys::GDEXTENSION_METHOD_ARGUMENT_METADATA_INT_IS_INT64
        }

        ffi_methods! { type sys::GDExtensionTypePtr = *mut Self; .. }
    }

    unsafe impl GodotFfi for f64 {
        const VARIANT_TYPE: ExtVariantType = ExtVariantType::Concrete(sys::VariantType::FLOAT);

        fn default_param_metadata() -> sys::GDExtensionClassMethodArgumentMetadata {
            sys::GDEXTENSION_METHOD_ARGUMENT_METADATA_REAL_IS_DOUBLE
        }

        ffi_methods! { type sys::GDExtensionTypePtr = *mut Self; .. }
    }

    unsafe impl GodotFfi for () {
        const VARIANT_TYPE: ExtVariantType = ExtVariantType::Concrete(sys::VariantType::NIL);

        unsafe fn new_from_sys(_ptr: sys::GDExtensionConstTypePtr) -> Self {
            // Do nothing
        }

        unsafe fn new_with_uninit(init: impl FnOnce(sys::GDExtensionUninitializedTypePtr)) -> Self {
            // `init` may contain code that should be run, however it shouldn't actually write to the passed in pointer.
            let mut unit = ();
            init(std::ptr::addr_of_mut!(unit).cast());
            unit
        }

        unsafe fn new_with_init(init: impl FnOnce(sys::GDExtensionTypePtr)) -> Self {
            // `init` may contain code that should be run, however it shouldn't actually write to the passed in pointer.
            let mut unit = ();
            init(std::ptr::addr_of_mut!(unit).cast());
            unit
        }

        fn sys(&self) -> sys::GDExtensionConstTypePtr {
            // ZST dummy pointer
            std::ptr::from_ref(self).cast()
        }

        fn sys_mut(&mut self) -> sys::GDExtensionTypePtr {
            // ZST dummy pointer
            std::ptr::from_mut(self).cast()
        }

        // SAFETY:
        // We're not accessing the value in `_ptr`.
        unsafe fn from_arg_ptr(
            _ptr: sys::GDExtensionTypePtr,
            _call_type: super::PtrcallType,
        ) -> Self {
        }

        // SAFETY:
        // We're not doing anything with `_dst`.
        unsafe fn move_return_ptr(
            self,
            _dst: sys::GDExtensionTypePtr,
            _call_type: super::PtrcallType,
        ) {
            // Do nothing
        }
    }
}
