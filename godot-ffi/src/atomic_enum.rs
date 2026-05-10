/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::marker::PhantomData;
use std::sync::atomic::{AtomicU8, Ordering};

/// A lock-free cell that stores a value of enum type `E` using a single byte.
///
/// All operations use [`Ordering::Relaxed`] -- sufficient for this crate (global flags not involved in acquire/release synchronisation chains).
pub struct AtomicEnum<E> {
    atomic: AtomicU8,
    _phantom: PhantomData<E>,
}

impl<E: AtomicIntLike> AtomicEnum<E> {
    /// Creates a new `AtomicEnum` with the given initial value.
    pub fn new(value: E) -> Self {
        Self {
            atomic: AtomicU8::new(value.to_u8()),
            _phantom: PhantomData,
        }
    }

    /// Creates a new `AtomicEnum` initialized to [`E::DEFAULT_ORD`][AtomicIntLike::DEFAULT_ORD].
    ///
    /// `const` constructor for use in `static` initializers, where [`new`][Self::new] cannot be used due to lack of `const` trait fns.
    pub const fn default() -> Self {
        Self {
            atomic: AtomicU8::new(E::DEFAULT_ORD),
            _phantom: PhantomData,
        }
    }

    /// Loads the current value.
    pub fn load(&self) -> E {
        let raw = self.atomic.load(Ordering::Relaxed);
        E::from_u8(raw)
    }

    /// Stores a new value.
    pub fn store(&self, value: E) {
        self.atomic.store(value.to_u8(), Ordering::Relaxed);
    }

    /// Stores a new value and returns the previous one.
    pub fn replace(&self, value: E) -> E {
        let old = self.atomic.swap(value.to_u8(), Ordering::Relaxed);
        E::from_u8(old)
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Trait and macro

/// Trait for types that can be used with [`AtomicEnum`].
///
/// Provides conversion to/from `u8` plus default constant for `const` construction. Typically implemented via the
/// [`atomic_enum!`][crate::atomic_enum] macro.
///
/// Automatically implemented for `Option<T>` by mapping `None` to `u8::MAX`. Make sure no variant has this value.
pub trait AtomicIntLike: Copy {
    /// Raw byte used by [`AtomicEnum::default()`] for `const` (e.g. `static`) initialization.
    const DEFAULT_ORD: u8;

    /// Convert this value to a `u8`.
    fn to_u8(self) -> u8;

    /// Convert a `u8` back to this type.
    ///
    /// # Panics
    /// Panics if the value is not a valid discriminant.
    fn from_u8(value: u8) -> Self;
}

impl<T: AtomicIntLike> AtomicIntLike for Option<T> {
    const DEFAULT_ORD: u8 = u8::MAX;

    fn to_u8(self) -> u8 {
        match self {
            None => u8::MAX,
            Some(inner) => T::to_u8(inner),
        }
    }

    fn from_u8(value: u8) -> Self {
        match value {
            u8::MAX => None,
            inner => Some(T::from_u8(inner)),
        }
    }
}

/// Macro to define atomic enums, automatically implementing the [`AtomicIntLike`] trait.
#[macro_export]
macro_rules! atomic_enum {
    (
        $(#[$attr:meta])*
        $vis:vis enum $name:ident {
            $(
                $(#[$vattr:meta])*
                $variant:ident = $ord:literal
            ),+ $(,)?
        }
    ) => {
        $(#[$attr])*
        #[repr(u8)]
        $vis enum $name {
            $($(#[$vattr])* $variant = $ord,)+
        }

        impl $crate::AtomicIntLike for $name {
            // Default is first variant's discriminant. Could use #[default], but needs complex macro machinery.
            const DEFAULT_ORD: u8 = [$($ord),+][0];

            fn to_u8(self) -> u8 { self as u8 }
            fn from_u8(value: u8) -> Self {
                match value {
                    $($ord => Self::$variant,)+
                    other => panic!(
                        concat!("invalid ", stringify!($name), " discriminant: {}"),
                        other
                    ),
                }
            }
        }
    };
}
