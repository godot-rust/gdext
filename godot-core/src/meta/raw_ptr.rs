/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::meta::error::ConvertError;
use crate::meta::{sealed, FromGodot, GodotConvert, GodotType, ToGodot};

/// Wrapper around a raw pointer, providing `ToGodot`/`FromGodot` for FFI passing.
///
/// This type allows raw pointers to be passed through the Godot FFI boundary. The pointer is converted to its memory address (as `i64`)
/// for FFI purposes.
///
/// You might need this in `#[func]`, in dynamic calls (`Object.call`) or other scenarios where you have to interface with Godot's low-level
/// pointer APIs. These pointers typically refer to GDExtension _native structures_, but there are a few other cases (e.g. `*const u8` for
/// C char arrays).
///
/// # Example
/// ```no_run
/// use godot::meta::{RawPtr, ToGodot};
/// use godot::classes::native::AudioFrame;
///
/// let frame = AudioFrame { left: 0.0, right: 1.0 };
/// let ptr: *const AudioFrame = &raw const frame;
///
/// // SAFETY: we keep `frame` alive while using `wrapped`.
/// let wrapped = unsafe { RawPtr::new(ptr) };
///
/// let variant = wrapped.to_variant();
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RawPtr<P: FfiRawPointer> {
    ptr: P,
}

impl<P: FfiRawPointer> RawPtr<P> {
    // Safety note: strictly speaking, return values might need to be a different type, as it's possible to e.g. store parameters in a RawPtr
    // field, and then return them later. An unsafe RawParam->RawReturn conversion could allow this. It's mostly a theoretical issue though,
    // and would make both public API and codegen implementation more complex, so we avoid it for now.

    /// Constructs a new `RawPtr` from a raw pointer.
    ///
    /// # Safety
    /// The pointer must remain valid as long as a Godot API accesses its value. Special care is necessary in `#[func]` and `I*` virtual
    /// function return types: if the pointer refers to a local variable, it will become immediately dangling, causing undefined behavior.
    #[inline]
    pub unsafe fn new(ptr: P) -> Self {
        RawPtr { ptr }
    }

    /// Constructs a new `RawPtr` wrapping a null pointer.
    ///
    /// # Safety
    /// You must ensure that Godot can handle null pointers in the specific Godot API where this value will be used.
    #[inline]
    pub unsafe fn null() -> Self {
        RawPtr::new(P::ptr_from_i64(0))
    }

    /// Returns the wrapped raw pointer.
    #[inline]
    pub fn ptr(self) -> P {
        self.ptr
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Trait impls

impl<P> GodotType for RawPtr<P>
where
    P: FfiRawPointer + 'static,
{
    type Ffi = i64;
    type ToFfi<'f> = i64;

    fn to_ffi(&self) -> Self::ToFfi<'_> {
        self.ptr.to_i64()
    }

    fn into_ffi(self) -> Self::Ffi {
        self.ptr.to_i64()
    }

    fn try_from_ffi(ffi: Self::Ffi) -> Result<Self, ConvertError> {
        Ok(RawPtr {
            ptr: P::ptr_from_i64(ffi),
        })
    }

    fn godot_type_name() -> String {
        "int".to_string()
    }
}

impl<P> GodotConvert for RawPtr<P>
where
    P: FfiRawPointer + 'static,
{
    type Via = Self;
}

impl<P> ToGodot for RawPtr<P>
where
    P: FfiRawPointer + 'static,
{
    type Pass = crate::meta::ByValue;

    fn to_godot(&self) -> Self::Via {
        *self
    }
}

impl<P> FromGodot for RawPtr<P>
where
    P: FfiRawPointer + 'static,
{
    fn try_from_godot(via: Self::Via) -> Result<Self, ConvertError> {
        Ok(via)
    }
}

impl<P> sealed::Sealed for RawPtr<P> where P: FfiRawPointer + 'static {}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Pointer trait

/// Trait for raw pointers that can be passed over the Godot FFI boundary as `i64` addresses.
///
/// This trait is implemented for `*const T` and `*mut T`, and used as a bound inside [`RawPtr`][crate::meta::RawPtr].
pub trait FfiRawPointer: Copy + sealed::Sealed {
    /// Converts the pointer to its memory address as `i64`.
    #[doc(hidden)]
    fn to_i64(self) -> i64;

    /// Reconstructs the pointer from a memory address stored as `i64`.
    #[doc(hidden)]
    fn ptr_from_i64(addr: i64) -> Self;
}

impl<T> FfiRawPointer for *const T {
    #[inline]
    fn to_i64(self) -> i64 {
        self as i64
    }

    #[inline]
    fn ptr_from_i64(addr: i64) -> Self {
        addr as *const T
    }
}

impl<T> FfiRawPointer for *mut T {
    #[inline]
    fn to_i64(self) -> i64 {
        self as i64
    }

    #[inline]
    fn ptr_from_i64(addr: i64) -> Self {
        addr as *mut T
    }
}
