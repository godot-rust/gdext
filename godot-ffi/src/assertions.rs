/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! Assertion macros for compile-time and runtime checks with different safeguard levels.

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Compile-time assertions

/// Verifies a condition at compile time.
// https://blog.rust-lang.org/2021/12/02/Rust-1.57.0.html#panic-in-const-contexts
#[macro_export]
macro_rules! static_assert {
    ($cond:expr) => {
        const _: () = assert!($cond);
    };
    ($cond:expr, $msg:literal) => {
        const _: () = assert!($cond, $msg);
    };
}

/// Verifies at compile time that two types `T` and `U` have the same size and alignment.
#[macro_export]
macro_rules! static_assert_eq_size_align {
    ($T:ty, $U:ty) => {
        godot_ffi::static_assert!(
            std::mem::size_of::<$T>() == std::mem::size_of::<$U>()
                && std::mem::align_of::<$T>() == std::mem::align_of::<$U>()
        );
    };
    ($T:ty, $U:ty, $msg:literal) => {
        godot_ffi::static_assert!(
            std::mem::size_of::<$T>() == std::mem::size_of::<$U>()
                && std::mem::align_of::<$T>() == std::mem::align_of::<$U>(),
            $msg
        );
    };
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Runtime assertions - strict mode

/// Acts like `assert!` when `safeguards_strict` is enabled (default in debug builds), and becomes a no-op otherwise.
#[macro_export]
macro_rules! strict_assert {
    ($($arg:tt)*) => {
        #[cfg(safeguards_strict)]
        assert!($($arg)*);
    };
}

/// Acts like `assert_eq!` when `safeguards_strict` is enabled (default in debug builds), and becomes a no-op otherwise.
#[macro_export]
macro_rules! strict_assert_eq {
    ($actual:expr, $expected:expr) => {
        #[cfg(safeguards_strict)]
        assert_eq!($actual, $expected);
    };
    ($actual:expr, $expected:expr, $($arg:tt)*) => {
        #[cfg(safeguards_strict)]
        assert_eq!($actual, $expected, $($arg)*);
    };
}

/// Acts like `assert_ne!` when `safeguards_strict` is enabled (default in debug builds), and becomes a no-op otherwise.
#[macro_export]
macro_rules! strict_assert_ne {
    ($actual:expr, $expected:expr) => {
        #[cfg(safeguards_strict)]
        assert_ne!($actual, $expected);
    };
    ($actual:expr, $expected:expr, $($arg:tt)*) => {
        #[cfg(safeguards_strict)]
        assert_ne!($actual, $expected, $($arg)*);
    };
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Runtime assertions - balanced mode

/// Acts like `assert!` when `safeguards_balanced` is enabled, and becomes a no-op otherwise.
#[macro_export]
macro_rules! balanced_assert {
    ($($arg:tt)*) => {
        #[cfg(safeguards_balanced)]
        assert!($($arg)*);
    };
}

/// Acts like `assert_eq!` when `safeguards_balanced` is enabled, and becomes a no-op otherwise.
#[macro_export]
macro_rules! balanced_assert_eq {
    ($actual:expr, $expected:expr) => {
        #[cfg(safeguards_balanced)]
        assert_eq!($actual, $expected);
    };
    ($actual:expr, $expected:expr, $($arg:tt)*) => {
        #[cfg(safeguards_balanced)]
        assert_eq!($actual, $expected, $($arg)*);
    };
}

/// Acts like `assert_ne!` when `safeguards_balanced` is enabled, and becomes a no-op otherwise.
#[macro_export]
macro_rules! balanced_assert_ne {
    ($actual:expr, $expected:expr) => {
        #[cfg(safeguards_balanced)]
        assert_ne!($actual, $expected);
    };
    ($actual:expr, $expected:expr, $($arg:tt)*) => {
        #[cfg(safeguards_balanced)]
        assert_ne!($actual, $expected, $($arg)*);
    };
}
