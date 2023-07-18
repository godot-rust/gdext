/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::fmt;
use std::ops::{Deref, DerefMut};

use godot_ffi as sys;
use sys::VariantType;

use crate::builtin::meta::{ClassName, VariantMetadata};
use crate::builtin::{FromVariant, ToVariant, Variant, VariantConversionError};
use crate::obj::Share;
use crate::{obj::GodotSingleton, property::Property};

use super::{GodotObjectPtr, RawGd};

/// Smart pointer to singletons owned by the Godot engine.
///
/// This smart pointer relies on the safety invariants promised by godot of singletons:
/// https://docs.godotengine.org/en/latest/tutorials/performance/thread_safe_apis.html#global-scope
/// And as such are entirely thread-safe.
///
/// You should generally call the singleton's `singleton()` function to get a new `Singleton<T>`, rather than
/// constructing a new one out of nowhere.
///
/// # Safety
///
/// A `Singleton<T>` pointer is guaranteed to always be valid. Make sure to never free a singleton from godot
/// or using an `unsafe` function in rust.
pub struct Singleton<T: GodotSingleton> {
    raw: RawGd<T>,
}

impl<T> Singleton<T>
where
    T: GodotSingleton,
{
    /// Create a `Singleton<T>` from a [`RawGd<T>`].
    ///
    /// # Panics
    ///
    /// If the `raw` is null or an invalid instance.
    pub fn from_raw(raw: RawGd<T>) -> Self {
        assert!(
            !raw.is_null(),
            "singleton {} should never be null",
            T::CLASS_NAME
        );
        assert!(
            raw.is_instance_valid(),
            "singleton {} should never be freed",
            T::CLASS_NAME
        );
        Self { raw }
    }

    /// Create a new `Singleton<T>` from the given `ptr`.
    ///
    /// # Panics
    ///
    /// If `ptr` is a null-pointer.
    ///
    /// # Safety
    ///
    /// `ptr` must either be null or a live pointer to an object of type `T`.
    #[doc(hidden)]
    pub unsafe fn from_obj_sys(sys: sys::GDExtensionObjectPtr) -> Self {
        let raw = RawGd::from_obj_sys(sys);
        Self::from_raw(raw)
    }
}

impl<T: GodotSingleton> GodotObjectPtr for Singleton<T> {
    type Class = T;

    fn raw(&self) -> &RawGd<T> {
        &self.raw
    }
}

impl<T: GodotSingleton> Clone for Singleton<T> {
    fn clone(&self) -> Self {
        Self {
            raw: self.raw.clone(),
        }
    }
}

impl<T: GodotSingleton> Share for Singleton<T> {
    fn share(&self) -> Self {
        self.clone()
    }
}

impl<T: GodotSingleton> Deref for Singleton<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        // SAFETY: Singletons cannot be safely freed.
        unsafe { self.raw.as_inner() }
    }
}

impl<T: GodotSingleton> DerefMut for Singleton<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        // SAFETY: Singletons cannot be safely freed.
        unsafe { self.raw.as_inner_mut() }
    }
}

/// Godot singletons are thread-safe, so we can safely share/send our reference between threads.
unsafe impl<T: GodotSingleton> Sync for Singleton<T> {}
unsafe impl<T: GodotSingleton> Send for Singleton<T> {}

impl<T: GodotSingleton> Property for Singleton<T> {
    type Intermediate = Self;

    fn get_property(&self) -> Self {
        self.clone()
    }

    fn set_property(&mut self, value: Self) {
        *self = value;
    }
}

impl<T: GodotSingleton> FromVariant for Singleton<T> {
    fn try_from_variant(variant: &Variant) -> Result<Self, VariantConversionError> {
        let singleton = Self::from_raw(RawGd::try_from_variant(variant)?);
        Ok(singleton)
    }
}

impl<T: GodotSingleton> ToVariant for Singleton<T> {
    fn to_variant(&self) -> Variant {
        // This already increments the refcount.
        self.raw.to_variant()
    }
}

impl<T: GodotSingleton> FromVariant for Option<Singleton<T>> {
    fn try_from_variant(variant: &Variant) -> Result<Self, VariantConversionError> {
        if variant.is_nil() {
            Ok(None)
        } else {
            Singleton::try_from_variant(variant).map(Some)
        }
    }
}

impl<T: GodotSingleton> ToVariant for Option<Singleton<T>> {
    fn to_variant(&self) -> Variant {
        match self {
            Some(gd) => gd.to_variant(),
            None => Variant::nil(),
        }
    }
}

impl<T: GodotSingleton> PartialEq for Singleton<T> {
    /// ⚠️ Returns whether two `Singleton` pointers point to the same object.
    ///
    /// # Panics
    /// When `self` or `other` is dead.
    fn eq(&self, other: &Self) -> bool {
        // Panics when one is dead
        self.instance_id() == other.instance_id()
    }
}

impl<T: GodotSingleton> Eq for Singleton<T> {}

impl<T: GodotSingleton> fmt::Display for Singleton<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.raw.display_string(f)
    }
}

impl<T: GodotSingleton> fmt::Debug for Singleton<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.raw.debug_string(f, "Gd")
    }
}

impl<T: GodotSingleton> VariantMetadata for Singleton<T> {
    fn variant_type() -> VariantType {
        RawGd::<T>::variant_type()
    }

    fn class_name() -> ClassName {
        RawGd::<T>::class_name()
    }
}

// Gd unwinding across panics does not invalidate any invariants;
// its mutability is anyway present, in the Godot engine.
impl<T: GodotSingleton> std::panic::UnwindSafe for Singleton<T> {}
impl<T: GodotSingleton> std::panic::RefUnwindSafe for Singleton<T> {}
