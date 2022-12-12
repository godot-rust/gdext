/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate as sys;
use std::fmt::Debug;

/// Adds methods to convert from and to Godot FFI pointers.
#[doc(hidden)]
pub trait GodotFfi {
    /// Construct from Godot opaque pointer.
    unsafe fn from_sys(ptr: sys::GDExtensionTypePtr) -> Self;

    /// Construct uninitialized opaque data, then initialize it with `init` function.
    unsafe fn from_sys_init(init_fn: impl FnOnce(sys::GDExtensionTypePtr)) -> Self;

    /// Return Godot opaque pointer, for an immutable operation.
    ///
    /// Note that this is a `*mut` pointer despite taking `&self` by shared-ref.
    /// This is because most of Godot's rust API is not const-correct. This can still
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

    unsafe fn write_sys(&self, dst: sys::GDExtensionTypePtr);
}

/// Trait implemented for all types that can be passed to and from user-defined `#[func]` methods
/// through Godot's _ptrcall_ calling convention.
pub trait GodotFuncMarshal: Sized {
    /// Intermediate type through which Self is converted, and which can cause failure.
    type Via: Debug;

    /// Used for function arguments. On failure, the argument which can't be converted to Self is returned.
    unsafe fn try_from_sys(ptr: sys::GDExtensionTypePtr) -> Result<Self, Self::Via>;

    /// Used for function return values. On failure, `self` which can't be converted to Via is returned.
    unsafe fn try_write_sys(&self, dst: sys::GDExtensionTypePtr) -> Result<(), Self>;
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Macros to choose a certain implementation of `GodotFfi` trait for GDExtensionTypePtr;
// or a free-standing `impl` for concrete sys pointers such as GDExtensionObjectPtr.
// See doc comment of `ffi_methods!` for information

#[macro_export]
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
		unsafe fn $from_sys_init(init: impl FnOnce($Ptr)) -> Self {
            let mut raw = std::mem::MaybeUninit::uninit();
            init(raw.as_mut_ptr() as $Ptr);

            Self::from_opaque(raw.assume_init())
        }
	};
	(OpaquePtr $Ptr:ty; $( #[$attr:meta] )? $vis:vis $sys:ident = sys) => {
		$( #[$attr] )? $vis
		fn $sys(&self) -> $Ptr {
            &self.opaque as *const _ as $Ptr
        }
	};
	(OpaquePtr $Ptr:ty; $( #[$attr:meta] )? $vis:vis $write_sys:ident = write_sys) => {
		$( #[$attr] )? $vis
		unsafe fn $write_sys(&self, dst: $Ptr) {
            // Note: this is the same impl as for impl_ffi_as_opaque_value, which is... interesting
            std::ptr::write(dst as *mut _, self.opaque)
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
		unsafe fn $from_sys_init(init: impl FnOnce($Ptr)) -> Self {
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
	(OpaqueValue $Ptr:ty; $( #[$attr:meta] )? $vis:vis $write_sys:ident = write_sys) => {
		$( #[$attr] )? $vis
		unsafe fn $write_sys(&self, dst: $Ptr) {
            // Note: this is the same impl as for impl_ffi_as_opaque_value, which is... interesting
            std::ptr::write(dst as *mut _, self.opaque);
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
		unsafe fn $from_sys_init(init: impl FnOnce($Ptr)) -> Self {
            let mut raw = std::mem::MaybeUninit::<Self>::uninit();
            init(raw.as_mut_ptr() as $Ptr);

            raw.assume_init()
        }
	};
	(SelfPtr $Ptr:ty; $( #[$attr:meta] )? $vis:vis $sys:ident = sys) => {
		$( #[$attr] )? $vis
		fn $sys(&self) -> $Ptr {
            self as *const Self as $Ptr
        }
	};
	(SelfPtr $Ptr:ty; $( #[$attr:meta] )? $vis:vis $write_sys:ident = write_sys) => {
		$( #[$attr] )? $vis
		unsafe fn $write_sys(&self, dst: $Ptr) {
            *(dst as *mut Self) = *self;
        }
	};
}

#[macro_export]
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

	( // impl GodotFfi for T (default all 4)
		$Impl:ident $Ptr:ty; ..
	) => {
		$crate::ffi_methods_one!($Impl $Ptr; from_sys = from_sys);
		$crate::ffi_methods_one!($Impl $Ptr; from_sys_init = from_sys_init);
		$crate::ffi_methods_one!($Impl $Ptr; sys = sys);
		$crate::ffi_methods_one!($Impl $Ptr; write_sys = write_sys);
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
    use super::{GodotFfi, GodotFuncMarshal};
    use crate as sys;
    use std::convert::Infallible;

    macro_rules! impl_godot_marshalling {
        ($T:ty) => {
            impl GodotFfi for $T {
                ffi_methods! { type sys::GDExtensionTypePtr = *mut Self; .. }
            }
        };

        ($T:ty as $Via:ty) => {
            // implicit bounds:
            //    T: TryFrom<Via>, Copy
            //    Via: TryFrom<T>, GodotFfi
            impl GodotFuncMarshal for $T {
                type Via = $Via;

                unsafe fn try_from_sys(ptr: sys::GDExtensionTypePtr) -> Result<Self, $Via> {
                    let via = <$Via as GodotFfi>::from_sys(ptr);
                    Self::try_from(via).map_err(|_| via)
                }

                unsafe fn try_write_sys(&self, dst: sys::GDExtensionTypePtr) -> Result<(), Self> {
                    <$Via>::try_from(*self)
                        .and_then(|via| {
                            <$Via as GodotFfi>::write_sys(&via, dst);
                            Ok(())
                        })
                        .map_err(|_| *self)
                }
            }
        };

        ($T:ty as $Via:ty; lossy) => {
            // implicit bounds:
            //    T: TryFrom<Via>, Copy
            //    Via: TryFrom<T>, GodotFfi
            impl GodotFuncMarshal for $T {
                type Via = $Via;

                unsafe fn try_from_sys(ptr: sys::GDExtensionTypePtr) -> Result<Self, $Via> {
                    let via = <$Via as GodotFfi>::from_sys(ptr);
                    Ok(via as Self)
                }

                unsafe fn try_write_sys(&self, dst: sys::GDExtensionTypePtr) -> Result<(), Self> {
                    let via = *self as $Via;
                    <$Via as GodotFfi>::write_sys(&via, dst);
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

    impl GodotFfi for () {
        unsafe fn from_sys(_ptr: sys::GDExtensionTypePtr) -> Self {
            // Do nothing
        }

        unsafe fn from_sys_init(_init: impl FnOnce(sys::GDExtensionTypePtr)) -> Self {
            // Do nothing
        }

        fn sys(&self) -> sys::GDExtensionTypePtr {
            // ZST dummy pointer
            self as *const _ as sys::GDExtensionTypePtr
        }

        unsafe fn write_sys(&self, _dst: sys::GDExtensionTypePtr) {
            // Do nothing
        }
    }

    impl<T> GodotFuncMarshal for T
    where
        T: GodotFfi,
    {
        type Via = Infallible;

        unsafe fn try_from_sys(ptr: sys::GDExtensionTypePtr) -> Result<Self, Infallible> {
            Ok(Self::from_sys(ptr))
        }

        unsafe fn try_write_sys(&self, dst: sys::GDExtensionTypePtr) -> Result<(), Self> {
            self.write_sys(dst);
            Ok(())
        }
    }
}
