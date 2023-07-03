/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

// TODO(bromeon): test this against Godot's own is_equal_approx() implementation for equality-comparable built-in types (excl Callable/Rid/...)

/// Approximate equality-comparison of geometric types.
///
/// The implementation is specific to the type. It's mostly used for gdext-internal tests, but you may use it for your own code.
/// Note that we give no guarantees about precision, and implementation can change at any time.
///
/// We currently also do not guarantee that this gives the same results as Godot's own `is_equal_approx()` function; although this may
/// be the goal in the future.
pub trait ApproxEq: PartialEq {
    fn approx_eq(&self, other: &Self) -> bool;
}

/// Asserts that two values are approximately equal
///
/// For comparison, this uses `ApproxEq::approx_eq` by default, or the provided `fn = ...` function.
#[macro_export]
macro_rules! assert_eq_approx {
    ($actual:expr, $expected:expr, fn = $func:expr $(,)?) => {
        match ($actual, $expected) {
            (a, b) => assert!(($func)(&a, &b), "\n  left: {:?},\n right: {:?}", $actual, $expected)
        }
    };
    ($actual:expr, $expected:expr, fn = $func:expr, $($t:tt)+) => {
        match ($actual, $expected) {
            (a, b) => assert!(($func)(&a, &b), "\n  left: {:?},\n right: {:?}{}", $actual, $expected, format_args!($($t)+) )
        }
    };
    ($actual:expr, $expected:expr $(,)?) => {
        match ($actual, $expected) {
             (a, b) => assert!($crate::builtin::math::ApproxEq::approx_eq(&a, &b), "\n  left: {:?},\n right: {:?}", $actual, $expected),
            // (a, b) => $crate::assert_eq_approx!($actual, $expected, fn = $crate::builtin::ApproxEq::approx_eq),
        }
    };
    ($actual:expr, $expected:expr, $($t:tt)+) => {
        match ($actual, $expected) {
            (a, b) => assert!($crate::builtin::math::ApproxEq::approx_eq(&a, &b), "\n  left: {:?},\n right: {:?},\n{}", $actual, $expected, format_args!($($t)+)),
            // (a, b) => $crate::assert_eq_approx!($actual, $expected, fn = $crate::builtin::ApproxEq::approx_eq, $($t)+),
        }
    };
}

/// Asserts that two values are not approximately equal, using the provided
/// `func` for equality checking.
#[macro_export]
macro_rules! assert_ne_approx {
    ($actual:expr, $expected:expr, fn = $func:expr $(, $($t:tt)* )?) => {
        #[allow(clippy::redundant_closure_call)]
        {
            $crate::assert_eq_approx!($actual, $expected, fn = |a,b| !($func)(a, b) $(, $($t)* )?)
        }
    };

    ($actual:expr, $expected:expr $(, $($t:tt)* )?) => {
        #[allow(clippy::redundant_closure_call)]
        {
            $crate::assert_eq_approx!($actual, $expected, fn = |a, b| !$crate::builtin::math::ApproxEq::approx_eq(a, b) $(, $($t)* )?)
        }
    };
}
