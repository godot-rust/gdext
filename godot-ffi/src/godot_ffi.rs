/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::{self as sys, ptr_then};
use std::{fmt::Debug, ptr};

/// Adds methods to convert from and to Godot FFI pointers.
/// See [crate::ffi_methods] for ergonomic implementation.
///
/// # Safety
///
/// [`from_arg_ptr`](GodotFfi::from_arg_ptr) and [`move_return_ptr`](GodotFfi::move_return_ptr)
/// must properly initialize and clean up values given the [`PtrcallType`] provided by the caller.
pub unsafe trait GodotFfi {
    /// Construct from Godot opaque pointer.
    ///
    /// # Safety
    /// `ptr` must be a valid _type ptr_: it must follow Godot's convention to encode `Self`,
    /// which is different depending on the type.
    /// The type in `ptr` must not require any special consideration upon referencing. Such as
    /// incrementing a refcount.
    unsafe fn from_sys(ptr: sys::GDExtensionTypePtr) -> Self;

    /// Construct uninitialized opaque data, then initialize it with `init_fn` function.
    ///
    /// # Safety
    /// `init_fn` must be a function that correctly handles a (possibly-uninitialized) _type ptr_.
    unsafe fn from_sys_init(init_fn: impl FnOnce(sys::GDExtensionUninitializedTypePtr)) -> Self;

    /// Like [`Self::from_sys_init`], but pre-initializes the sys pointer to a `Default::default()` instance
    /// before calling `init_fn`.
    ///
    /// Some FFI functions in Godot expect a pre-existing instance at the destination pointer, e.g. CoW/ref-counted
    /// builtin types like `Array`, `Dictionary`, `String`, `StringName`.
    ///
    /// If not overridden, this just calls [`Self::from_sys_init`].
    ///
    /// # Safety
    /// `init_fn` must be a function that correctly handles a (possibly-uninitialized) _type ptr_.
    unsafe fn from_sys_init_default(init_fn: impl FnOnce(sys::GDExtensionTypePtr)) -> Self
    where
        Self: Sized, // + Default
    {
        // SAFETY: this default implementation is potentially incorrect.
        // By implementing the GodotFfi trait, you acknowledge that these may need to be overridden.
        Self::from_sys_init(|ptr| init_fn(sys::AsUninit::force_init(ptr)))

        // TODO consider using this, if all the implementors support it
        // let mut result = Self::default();
        // init_fn(result.sys_mut().as_uninit());
        // result
    }

    /// Return Godot opaque pointer, for an immutable operation.
    ///
    /// Note that this is a `*mut` pointer despite taking `&self` by shared-ref.
    /// This is because most of Godot's Rust API is not const-correct. This can still
    /// enhance user code (calling `sys_mut` ensures no aliasing at the time of the call).
    fn sys(&self) -> sys::GDExtensionTypePtr;

    /// Return Godot opaque pointer, for a mutable operation.
    ///
    /// Should usually not be overridden; behaves like `sys()` but ensures no aliasing
    /// at the time of the call (not necessarily during any subsequent modifications though).
    fn sys_mut(&mut self) -> sys::GDExtensionTypePtr {
        self.sys()
    }

    // TODO check if sys() can take over this
    // also, from_sys() might take *const T
    // possibly separate 2 pointer types
    fn sys_const(&self) -> sys::GDExtensionConstTypePtr {
        self.sys()
    }

    /// Construct from a pointer to an argument in a call.
    ///
    /// # Safety
    /// * `ptr` must be a valid _type ptr_: it must follow Godot's convention to encode `Self`,
    ///   which is different depending on the type.
    ///
    /// * `ptr` must encode `Self` according to the given `call_type`'s encoding of argument values.
    unsafe fn from_arg_ptr(ptr: sys::GDExtensionTypePtr, call_type: PtrcallType) -> Self;

    /// Move self into the pointer in pointer `dst`, dropping what is already in `dst.
    ///
    /// # Safety
    /// * `dst` must be a valid _type ptr_: it must follow Godot's convention to encode `Self`,
    ///    which is different depending on the type.
    ///
    /// * `dst` must be able to accept a value of type `Self` encoded according to the given
    ///   `call_type`'s encoding of return values.
    unsafe fn move_return_ptr(self, dst: sys::GDExtensionTypePtr, call_type: PtrcallType);
}

// In Godot 4.0.x, a lot of that are "constructed into" require a default-initialized value.
// In Godot 4.1+, placement new is used, requiring no prior value.
// This method abstracts over that. Outside of GodotFfi because it should not be overridden.

/// # Safety
///
/// See [`GodotFfi::from_sys_init`] and [`GodotFfi::from_sys_init_default`].
#[cfg(gdextension_api = "4.0")]
pub unsafe fn from_sys_init_or_init_default<T: GodotFfi>(
    init_fn: impl FnOnce(sys::GDExtensionTypePtr),
) -> T {
    T::from_sys_init_default(init_fn)
}

/// # Safety
///
/// See [`GodotFfi::from_sys_init`] and [`GodotFfi::from_sys_init_default`].
#[cfg(not(gdextension_api = "4.0"))]
pub unsafe fn from_sys_init_or_init_default<T: GodotFfi>(
    init_fn: impl FnOnce(sys::GDExtensionUninitializedTypePtr),
) -> T {
    T::from_sys_init(init_fn)
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

/// Marks a type as having a nullable counterpart in Godot.
///
/// This trait primarily exists to implement GodotFfi for `Option<Gd<T>>`, which is not possible
/// due to Rusts orphan rule. The rule also enforces better API design, though. `godot_ffi` should
/// not concern itself with the details of how Godot types work and merely defines the FFI abstraction.
/// By having a marker trait for nullable types, we can provide a generic implementation for
/// compatible types, without knowing their definition.
///
/// # Safety
///
/// The type has to have a pointer-sized counterpart in Godot, which needs to be nullable.
/// So far, this only applies to class types (Object hierarchy).
pub unsafe trait GodotNullablePtr: GodotFfi {}

unsafe impl<T> GodotFfi for Option<T>
where
    T: GodotNullablePtr,
{
    unsafe fn from_sys(ptr: sys::GDExtensionTypePtr) -> Self {
        ptr_then(ptr, |ptr| T::from_sys(ptr))
    }

    unsafe fn from_sys_init(init_fn: impl FnOnce(sys::GDExtensionUninitializedTypePtr)) -> Self {
        let mut raw = std::mem::MaybeUninit::uninit();
        init_fn(raw.as_mut_ptr() as sys::GDExtensionUninitializedTypePtr);

        Self::from_sys(raw.assume_init())
    }

    fn sys(&self) -> sys::GDExtensionTypePtr {
        match self {
            Some(value) => value.sys(),
            None => ptr::null_mut() as sys::GDExtensionTypePtr,
        }
    }

    unsafe fn from_arg_ptr(ptr: sys::GDExtensionTypePtr, call_type: PtrcallType) -> Self {
        ptr_then(ptr, |ptr| T::from_arg_ptr(ptr, call_type))
    }

    unsafe fn move_return_ptr(self, ptr: sys::GDExtensionTypePtr, call_type: PtrcallType) {
        if let Some(value) = self {
            value.move_return_ptr(ptr, call_type)
        }
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

/// An indication of what type of pointer call is being made.
#[derive(Default, Copy, Clone, Eq, PartialEq, Debug)]
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
    /// See also https://github.com/godotengine/godot-cpp/issues/954.
    Virtual,
}

/// Trait implemented for all types that can be passed to and from user-defined `#[func]` methods
/// through Godot's _ptrcall_ calling convention.
pub trait GodotFuncMarshal: Sized {
    /// Intermediate type through which Self is converted, and which can cause failure.
    type Via: Debug;

    /// Used for function arguments. On failure, the argument which can't be converted to Self is returned.
    ///
    /// # Safety
    /// The value behind `ptr` must be the C FFI type that corresponds to `Self`.
    /// See also [`GodotFfi::from_arg_ptr`].
    unsafe fn try_from_arg(
        ptr: sys::GDExtensionTypePtr,
        call_type: PtrcallType,
    ) -> Result<Self, Self::Via>;

    /// Used for function return values. On failure, `self` which can't be converted to Via is returned.
    ///
    /// # Safety
    /// The value behind `ptr` must be the C FFI type that corresponds to `Self`.
    /// `dst` must point to an initialized value of type `Via`.
    /// See also [`GodotFfi::move_return_ptr`].
    unsafe fn try_return(
        self,
        dst: sys::GDExtensionTypePtr,
        call_type: PtrcallType,
    ) -> Result<(), Self>;
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Macros to choose a certain implementation of `GodotFfi` trait for GDExtensionTypePtr;
// or a free-standing `impl` for concrete sys pointers such as GDExtensionObjectPtr.
// See doc comment of `ffi_methods!` for information

#[macro_export]
#[doc(hidden)]
macro_rules! ffi_methods_one {
    // type $Ptr = *mut Opaque
    (OpaquePtr $Ptr:ty; $( #[$attr:meta] )? $vis:vis $from_sys:ident = from_sys) => {
        $( #[$attr] )? $vis
        unsafe fn $from_sys(ptr: $Ptr) -> Self {
            let opaque = std::ptr::read(ptr as *mut _);
            Self::from_opaque(opaque)
        }
    };
    (OpaquePtr $Ptr:ty; $( #[$attr:meta] )? $vis:vis $from_sys_init:ident = from_sys_init) => {
        $( #[$attr] )? $vis
        unsafe fn $from_sys_init(init: impl FnOnce(<$Ptr as $crate::AsUninit>::Ptr)) -> Self {
            let mut raw = std::mem::MaybeUninit::uninit();
            init(raw.as_mut_ptr() as <$Ptr as $crate::AsUninit>::Ptr);

            Self::from_opaque(raw.assume_init())
        }
    };
    (OpaquePtr $Ptr:ty; $( #[$attr:meta] )? $vis:vis $sys:ident = sys) => {
        $( #[$attr] )? $vis
        fn $sys(&self) -> $Ptr {
            &self.opaque as *const _ as $Ptr
        }
    };
    (OpaquePtr $Ptr:ty; $( #[$attr:meta] )? $vis:vis $from_arg_ptr:ident = from_arg_ptr) => {
        $( #[$attr] )? $vis
        unsafe fn $from_arg_ptr(ptr: $Ptr, _call_type: $crate::PtrcallType) -> Self {
            Self::from_sys(ptr as *mut _)
        }
    };
    (OpaquePtr $Ptr:ty; $( #[$attr:meta] )? $vis:vis $move_return_ptr:ident = move_return_ptr) => {
        $( #[$attr] )? $vis
        unsafe fn $move_return_ptr(mut self, dst: $Ptr, _call_type: $crate::PtrcallType) {
            std::ptr::swap(dst as *mut _, std::ptr::addr_of_mut!(self.opaque))
        }
    };

    // type $Ptr = Opaque
    (OpaqueValue $Ptr:ty; $( #[$attr:meta] )? $vis:vis $from_sys:ident = from_sys) => {
        $( #[$attr] )? $vis
        unsafe fn $from_sys(ptr: $Ptr) -> Self {
            let opaque = std::mem::transmute(ptr);
            Self::from_opaque(opaque)
        }
    };
    (OpaqueValue $Ptr:ty; $( #[$attr:meta] )? $vis:vis $from_sys_init:ident = from_sys_init) => {
        $( #[$attr] )? $vis
        unsafe fn $from_sys_init(init: impl FnOnce(<$Ptr as $crate::AsUninit>::Ptr)) -> Self {
            let mut raw = std::mem::MaybeUninit::uninit();
            init(std::mem::transmute(raw.as_mut_ptr()));
            Self::from_opaque(raw.assume_init())
        }
    };
    (OpaqueValue $Ptr:ty; $( #[$attr:meta] )? $vis:vis $sys:ident = sys) => {
        $( #[$attr] )? $vis
        fn $sys(&self) -> $Ptr {
            unsafe { std::mem::transmute(self.opaque) }
        }
    };
    (OpaqueValue $Ptr:ty; $( #[$attr:meta] )? $vis:vis $from_arg_ptr:ident = from_arg_ptr) => {
        $( #[$attr] )? $vis
        unsafe fn $from_arg_ptr(ptr: $Ptr, _call_type: $crate::PtrcallType) -> Self {
            Self::from_sys(ptr as *mut _)
        }
    };
    (OpaqueValue $Ptr:ty; $( #[$attr:meta] )? $vis:vis $move_return_ptr:ident = move_return_ptr) => {
        $( #[$attr] )? $vis
        unsafe fn $move_return_ptr(mut self, dst: $Ptr, _call_type: $crate::PtrcallType) {
            std::ptr::swap(dst, std::mem::transmute::<_, $Ptr>(self.opaque))
        }
    };

    // type $Ptr = *mut Self
    (SelfPtr $Ptr:ty; $( #[$attr:meta] )? $vis:vis $from_sys:ident = from_sys) => {
        $( #[$attr] )? $vis
        unsafe fn $from_sys(ptr: $Ptr) -> Self {
            *(ptr as *mut Self)
        }
    };
    (SelfPtr $Ptr:ty; $( #[$attr:meta] )? $vis:vis $from_sys_init:ident = from_sys_init) => {
        $( #[$attr] )? $vis
        unsafe fn $from_sys_init(init: impl FnOnce(<$Ptr as $crate::AsUninit>::Ptr)) -> Self {
            let mut raw = std::mem::MaybeUninit::<Self>::uninit();
            init(raw.as_mut_ptr() as <$Ptr as $crate::AsUninit>::Ptr);

            raw.assume_init()
        }
    };
    (SelfPtr $Ptr:ty; $( #[$attr:meta] )? $vis:vis $sys:ident = sys) => {
        $( #[$attr] )? $vis
        fn $sys(&self) -> $Ptr {
            self as *const Self as $Ptr
        }
    };
    (SelfPtr $Ptr:ty; $( #[$attr:meta] )? $vis:vis $from_arg_ptr:ident = from_arg_ptr) => {
        $( #[$attr] )? $vis
        unsafe fn $from_arg_ptr(ptr: $Ptr, _call_type: $crate::PtrcallType) -> Self {
            *(ptr as *mut Self)
        }
    };
    (SelfPtr $Ptr:ty; $( #[$attr:meta] )? $vis:vis $move_return_ptr:ident = move_return_ptr) => {
        $( #[$attr] )? $vis
        unsafe fn $move_return_ptr(self, dst: $Ptr, _call_type: $crate::PtrcallType) {
            *(dst as *mut Self) = self
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
        $crate::ffi_methods_one!($Impl $Ptr; from_sys = from_sys);
        $crate::ffi_methods_one!($Impl $Ptr; from_sys_init = from_sys_init);
        $crate::ffi_methods_one!($Impl $Ptr; sys = sys);
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
/// * `Opaque`<br>
///   Implements FFI methods for a type with `Opaque` data.
///   The sys pointer is directly reinterpreted from/to the `Opaque` and **not** its address.
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
/// Turning pointer call arguments into a value is simply calling `from_opaque` on the
/// dereferenced argument pointer.
/// Returning a value from a pointer call is simply calling [`std::ptr::swap`] on the return pointer
/// and the address to the `opaque` field.
///
/// ## Using `Opaque`
///
/// Turning pointer call arguments into a value is simply calling `from_opaque` on the argument pointer.
/// Returning a value from a pointer call is simply calling [`std::ptr::swap`] on the return pointer
/// and the `opaque` field transmuted into a pointer.
///  
/// ## Using `*mut Self`
///
/// Turning pointer call arguments into a value is a dereference.
/// Returning a value from a pointer call is `*ret_ptr = value`.
#[macro_export]
macro_rules! ffi_methods {
    ( // Sys pointer = address of opaque
        type $Ptr:ty = *mut Opaque;
        $( $rest:tt )*
    ) => {
        $crate::ffi_methods_rest!(OpaquePtr $Ptr; $($rest)*);
    };

    ( // Sys pointer = value of opaque
        type $Ptr:ty = Opaque;
        $( $rest:tt )*
    ) => {
        $crate::ffi_methods_rest!(OpaqueValue $Ptr; $($rest)*);
    };

    ( // Sys pointer = address of self
        type $Ptr:ty = *mut Self;
        $( $rest:tt )*
    ) => {
        $crate::ffi_methods_rest!(SelfPtr $Ptr; $($rest)*);
    };
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Implementation for common types (needs to be this crate due to orphan rule)
mod scalars {
    use super::{GodotFfi, GodotFuncMarshal, PtrcallType};
    use crate as sys;
    use std::convert::Infallible;

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

                unsafe fn try_from_arg(
                    ptr: sys::GDExtensionTypePtr,
                    call_type: PtrcallType,
                ) -> Result<Self, $Via> {
                    let via = <$Via>::from_arg_ptr(ptr, call_type);
                    Self::try_from(via).map_err(|_| via)
                }

                unsafe fn try_return(
                    self,
                    dst: sys::GDExtensionTypePtr,
                    call_type: PtrcallType,
                ) -> Result<(), Self> {
                    <$Via>::from(self).move_return_ptr(dst, call_type);
                    Ok(())
                }
            }
        };

        ($T:ty as $Via:ty; lossy) => {
            // implicit bounds:
            //    T: TryFrom<Via>, Copy
            //    Via: TryFrom<T>, GodotFfi
            impl GodotFuncMarshal for $T {
                type Via = $Via;

                unsafe fn try_from_arg(
                    ptr: sys::GDExtensionTypePtr,
                    call_type: PtrcallType,
                ) -> Result<Self, $Via> {
                    let via = <$Via>::from_arg_ptr(ptr, call_type);
                    Ok(via as Self)
                }

                unsafe fn try_return(
                    self,
                    dst: sys::GDExtensionTypePtr,
                    call_type: PtrcallType,
                ) -> Result<(), Self> {
                    (self as $Via).move_return_ptr(dst, call_type);
                    Ok(())
                }
            }
        };
    }

    // Directly implements GodotFfi + GodotFuncMarshal (through blanket impl)
    impl_godot_marshalling!(bool);
    impl_godot_marshalling!(i64);
    impl_godot_marshalling!(f64);

    // Only implements GodotFuncMarshal
    impl_godot_marshalling!(i32 as i64);
    impl_godot_marshalling!(u32 as i64);
    impl_godot_marshalling!(i16 as i64);
    impl_godot_marshalling!(u16 as i64);
    impl_godot_marshalling!(i8 as i64);
    impl_godot_marshalling!(u8 as i64);

    impl_godot_marshalling!(f32 as f64; lossy);

    unsafe impl<T> GodotFfi for *const T {
        ffi_methods! { type sys::GDExtensionTypePtr = *mut Self; .. }
    }

    unsafe impl<T> GodotFfi for *mut T {
        ffi_methods! { type sys::GDExtensionTypePtr = *mut Self; .. }
    }

    unsafe impl GodotFfi for () {
        unsafe fn from_sys(_ptr: sys::GDExtensionTypePtr) -> Self {
            // Do nothing
        }

        unsafe fn from_sys_init(_init: impl FnOnce(sys::GDExtensionUninitializedTypePtr)) -> Self {
            // Do nothing
        }

        fn sys(&self) -> sys::GDExtensionTypePtr {
            // ZST dummy pointer
            self as *const _ as sys::GDExtensionTypePtr
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

    impl<T> GodotFuncMarshal for T
    where
        T: GodotFfi,
    {
        type Via = Infallible;

        unsafe fn try_from_arg(
            ptr: sys::GDExtensionTypePtr,
            call_type: PtrcallType,
        ) -> Result<Self, Infallible> {
            Ok(Self::from_arg_ptr(ptr, call_type))
        }

        unsafe fn try_return(
            self,
            dst: sys::GDExtensionTypePtr,
            call_type: PtrcallType,
        ) -> Result<(), Self> {
            self.move_return_ptr(dst, call_type);
            Ok(())
        }
    }
}
