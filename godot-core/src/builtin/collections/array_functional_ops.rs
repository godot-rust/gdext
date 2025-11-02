/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::builtin::{to_usize, Array, Callable, Variant, VariantArray};
use crate::meta::{ArrayElement, AsArg};
use crate::{meta, sys};

/// Immutable, functional-programming operations for `Array`, based on Godot callables.
///
/// Returned by [`Array::functional_ops()`].
///
/// These methods exist to provide parity with Godot, e.g. when porting GDScript code to Rust. However, they come with several disadvantages
/// compared to Rust's [iterator adapters](https://doc.rust-lang.org/stable/core/iter/index.html#adapters):
/// - Not type-safe: callables are dynamically typed, so you need to double-check signatures. Godot may misinterpret returned values
///   (e.g. predicates apply to any "truthy" values, not just booleans).
/// - Slower: dispatching through callables is typically more costly than iterating over variants, especially since every call involves multiple
///   variant conversions, too. Combining multiple operations like `filter().map()` is very expensive due to intermediate allocations.
/// - Less composable/flexible: Godot's `map()` always returns an untyped array, even if the input is typed and unchanged by the mapping.
///   Rust's `collect()` on the other hand gives you control over the output type. Chaining iterators can apply multiple transformations lazily.
///
/// In many cases, it is thus better to use [`Array::iter_shared()`] combined with iterator adapters. Check the individual method docs of
/// this struct for concrete alternatives.
pub struct ArrayFunctionalOps<'a, T: ArrayElement> {
    array: &'a Array<T>,
}

impl<'a, T: ArrayElement> ArrayFunctionalOps<'a, T> {
    pub(super) fn new(owner: &'a Array<T>) -> Self {
        Self { array: owner }
    }

    /// Returns a new array containing only the elements for which the callable returns a truthy value.
    ///
    /// **Rust alternatives:** [`Iterator::filter()`].
    ///
    /// The callable has signature `fn(T) -> bool`.
    ///
    /// # Example
    /// ```no_run
    /// # use godot::prelude::*;
    /// let array = array![1, 2, 3, 4, 5];
    /// let even = array.functional_ops().filter(&Callable::from_fn("is_even", |args| {
    ///     args[0].to::<i64>() % 2 == 0
    /// }));
    /// assert_eq!(even, array![2, 4]);
    /// ```
    #[must_use]
    pub fn filter(&self, callable: &Callable) -> Array<T> {
        // SAFETY: filter() returns array of same type as self.
        unsafe { self.array.as_inner().filter(callable) }
    }

    /// Returns a new untyped array with each element transformed by the callable.
    ///
    /// **Rust alternatives:** [`Iterator::map()`].
    ///
    /// The callable has signature `fn(T) -> Variant`. Since the transformation can change the element type, this method returns
    /// a `VariantArray` (untyped array).
    ///
    /// # Example
    /// ```no_run
    /// # use godot::prelude::*;
    /// let array = array![1.1, 1.5, 1.9];
    /// let rounded = array.functional_ops().map(&Callable::from_fn("round", |args| {
    ///     args[0].to::<f64>().round() as i64
    /// }));
    /// assert_eq!(rounded, varray![1, 2, 2]);
    /// ```
    #[must_use]
    pub fn map(&self, callable: &Callable) -> VariantArray {
        // SAFETY: map() returns an untyped array.
        unsafe { self.array.as_inner().map(callable) }
    }

    /// Reduces the array to a single value by iteratively applying the callable.
    ///
    /// **Rust alternatives:** [`Iterator::fold()`] or [`Iterator::reduce()`].
    ///
    /// The callable takes two arguments: the accumulator and the current element.
    /// It returns the new accumulator value. The process starts with `initial` as the accumulator.
    ///
    /// # Example
    /// ```no_run
    /// # use godot::prelude::*;
    /// let array = array![1, 2, 3, 4];
    /// let sum = array.functional_ops().reduce(
    ///     &Callable::from_fn("sum", |args| {
    ///         args[0].to::<i64>() + args[1].to::<i64>()
    ///     }),
    ///     &0.to_variant()
    /// );
    /// assert_eq!(sum, 10.to_variant());
    /// ```
    #[must_use]
    pub fn reduce(&self, callable: &Callable, initial: &Variant) -> Variant {
        self.array.as_inner().reduce(callable, initial)
    }

    /// Returns `true` if the callable returns a truthy value for at least one element.
    ///
    /// **Rust alternatives:** [`Iterator::any()`].
    ///
    /// The callable has signature `fn(element) -> bool`.
    ///
    /// # Example
    /// ```no_run
    /// # use godot::prelude::*;
    /// let array = array![1, 2, 3, 4];
    /// let any_even = array.functional_ops().any(&Callable::from_fn("is_even", |args| {
    ///     args[0].to::<i64>() % 2 == 0
    /// }));
    /// assert!(any_even);
    /// ```
    pub fn any(&self, callable: &Callable) -> bool {
        self.array.as_inner().any(callable)
    }

    /// Returns `true` if the callable returns a truthy value for all elements.
    ///
    /// **Rust alternatives:** [`Iterator::all()`].
    ///
    /// The callable has signature `fn(element) -> bool`.
    ///
    /// # Example
    /// ```no_run
    /// # use godot::prelude::*;
    /// let array = array![2, 4, 6];
    /// let all_even = array.functional_ops().all(&Callable::from_fn("is_even", |args| {
    ///     args[0].to::<i64>() % 2 == 0
    /// }));
    /// assert!(all_even);
    /// ```
    pub fn all(&self, callable: &Callable) -> bool {
        self.array.as_inner().all(callable)
    }

    /// Finds the index of the first element matching a custom predicate.
    ///
    /// **Rust alternatives:** [`Iterator::position()`].
    ///
    /// The callable has signature `fn(element) -> bool`.
    ///
    /// Returns the index of the first element for which the callable returns a truthy value, starting from `from`.
    /// If no element matches, returns `None`.
    ///
    /// # Example
    /// ```no_run
    /// # use godot::prelude::*;
    /// let array = array![1, 2, 3, 4, 5];
    /// let is_even = Callable::from_fn("is_even", |args| {
    ///     args[0].to::<i64>() % 2 == 0
    /// });
    /// assert_eq!(array.functional_ops().find_custom(&is_even, None), Some(1)); // value 2
    /// assert_eq!(array.functional_ops().find_custom(&is_even, Some(2)), Some(3)); // value 4
    /// ```
    #[cfg(since_api = "4.4")]
    pub fn find_custom(&self, callable: &Callable, from: Option<usize>) -> Option<usize> {
        let from = from.map(|i| i as i64).unwrap_or(0);
        let found_index = self.array.as_inner().find_custom(callable, from);

        sys::found_to_option(found_index)
    }

    /// Finds the index of the last element matching a custom predicate, searching backwards.
    ///
    /// **Rust alternatives:** [`Iterator::rposition()`].
    ///
    /// The callable has signature `fn(element) -> bool`.
    ///
    /// Returns the index of the last element for which the callable returns a truthy value, searching backwards from `from`.
    /// If no element matches, returns `None`.
    ///
    /// # Example
    /// ```no_run
    /// # use godot::prelude::*;
    /// let array = array![1, 2, 3, 4, 5];
    /// let is_even = Callable::from_fn("is_even", |args| {
    ///     args[0].to::<i64>() % 2 == 0
    /// });
    /// assert_eq!(array.functional_ops().rfind_custom(&is_even, None), Some(3)); // value 4
    /// assert_eq!(array.functional_ops().rfind_custom(&is_even, Some(2)), Some(1)); // value 2
    /// ```
    #[cfg(since_api = "4.4")]
    pub fn rfind_custom(&self, callable: &Callable, from: Option<usize>) -> Option<usize> {
        let from = from.map(|i| i as i64).unwrap_or(-1);
        let found_index = self.array.as_inner().rfind_custom(callable, from);

        sys::found_to_option(found_index)
    }

    /// Finds the index of a value in a sorted array using binary search, with `Callable` custom predicate.
    ///
    /// The callable `pred` takes two elements `(a, b)` and should return if `a < b` (strictly less).
    /// For a type-safe version, check out [`Array::bsearch_by()`].
    ///
    /// If the value is not present in the array, returns the insertion index that would maintain sorting order.
    ///
    /// Calling `bsearch_custom()` on an unsorted array results in unspecified behavior. Consider using [`Array::sort_unstable_custom()`]
    /// to ensure the sorting order is compatible with your callable's ordering.
    pub fn bsearch_custom(&self, value: impl AsArg<T>, pred: &Callable) -> usize {
        meta::arg_into_ref!(value: T);

        to_usize(
            self.array
                .as_inner()
                .bsearch_custom(&value.to_variant(), pred, true),
        )
    }
}
