/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

/// A type that tracks the state of borrows for a [`GdCell`].
///
/// This state upholds these invariants:
/// - You can only take a shared borrow when there is no accessible mutable borrow.
/// - You can only take a mutable borrow when there is neither an accessible mutable borrow, nor a shared
///   borrow.
/// - You can only set a mutable borrow as inaccessible when an accessible mutable borrow exists.
/// - You can only unset a mutable borrow as inaccessible when there is no accessible mutable borrow and no
///   shared borrows.
///
/// If a catastrophic error occurs, then the state will be poisoned. If the state is poisoned then that's
/// almost certainly an implementation bug, and should never happen. But in an abundance of caution it is
/// included to be safe.
#[derive(Clone, PartialEq, Debug)]
pub struct BorrowState {
    /// The number of `&T` references that are tracked.
    shared_count: usize,
    /// The number of `&mut T` references that are tracked.
    mut_count: usize,
    /// The number of `&mut T` references that are inaccessible.
    inaccessible_count: usize,
    /// `true` if the borrow state has reached an erroneous or unreliable state.
    poisoned: bool,
}

impl BorrowState {
    /// Create a new borrow state representing no borrows.
    pub fn new() -> Self {
        Self {
            shared_count: 0,
            mut_count: 0,
            inaccessible_count: 0,
            poisoned: false,
        }
    }

    /// Returns `true` if there are any accessible mutable references.
    pub fn has_accessible(&self) -> bool {
        let count = self.mut_count - self.inaccessible_count;

        assert!(
            count <= 1,
            "there should never be more than 1 accessible mutable reference"
        );

        count == 1
    }

    /// Returns the number of tracked shared references.
    ///
    /// Any amount of shared references will prevent [`Self::increment_mut`] from succeeding.
    pub fn shared_count(&self) -> usize {
        self.shared_count
    }

    /// Returns the number of tracked mutable references.
    pub fn mut_count(&self) -> usize {
        self.mut_count
    }

    /// Returns `true` if the state has reached an erroneous or unreliable state.
    pub fn is_poisoned(&self) -> bool {
        self.poisoned
    }

    /// Set self as having reached an erroneous or unreliable state.
    ///
    /// Always returns [`BorrowStateErr::Poisoned`].
    pub(crate) fn poison(&mut self, err: impl Into<String>) -> Result<(), BorrowStateErr> {
        self.poisoned = true;

        Err(BorrowStateErr::Poisoned(err.into()))
    }

    fn ensure_not_poisoned(&self) -> Result<(), BorrowStateErr> {
        if self.is_poisoned() {
            return Err(BorrowStateErr::IsPoisoned);
        }

        Ok(())
    }

    fn ensure_can_ref(&self) -> Result<(), BorrowStateErr> {
        self.ensure_not_poisoned()?;

        if self.has_accessible() {
            return Err("cannot borrow while accessible mutable borrow exists".into());
        }

        Ok(())
    }

    fn ensure_can_mut_ref(&self) -> Result<(), BorrowStateErr> {
        self.ensure_not_poisoned()?;

        if self.has_accessible() {
            return Err("cannot borrow while accessible mutable borrow exists".into());
        }

        if self.shared_count != 0 {
            return Err("cannot borrow mutable while shared borrow exists".into());
        }

        Ok(())
    }

    /// Track a new shared reference.
    ///
    /// Returns the new total number of shared references.
    ///
    /// This fails when:
    /// - There exists an accessible mutable reference.
    /// - There exist `usize::MAX` shared references.
    pub fn increment_shared(&mut self) -> Result<usize, BorrowStateErr> {
        self.ensure_not_poisoned()?;

        self.ensure_can_ref()?;

        self.shared_count = self
            .shared_count
            .checked_add(1)
            .ok_or("could not increment shared count")?;

        Ok(self.shared_count)
    }

    /// Untrack an existing shared reference.
    ///
    /// Returns the new total number of shared references.
    ///
    /// This fails when:
    /// - There are currently no tracked shared references.
    pub fn decrement_shared(&mut self) -> Result<usize, BorrowStateErr> {
        self.ensure_not_poisoned()?;

        if self.shared_count == 0 {
            return Err("cannot decrement shared counter when no shared reference exists".into());
        }

        if self.has_accessible() {
            self.poison("shared reference tracked while accessible mutable reference exists")?;
        }

        // We know `shared_count` isn't 0.
        self.shared_count -= 1;

        Ok(self.shared_count)
    }

    /// Track a new mutable reference.
    ///
    /// Returns the new total number of mutable references.
    ///
    /// This fails when:
    /// - There exists an accessible mutable reference.
    /// - There exists a shared reference.
    /// - There are `usize::MAX` tracked mutable references.
    ///
    /// Any amount of shared references will prevent [`Self::increment_inaccessible`] from succeeding.
    pub fn increment_mut(&mut self) -> Result<usize, BorrowStateErr> {
        self.ensure_not_poisoned()?;

        self.ensure_can_mut_ref()?;

        self.mut_count = self
            .mut_count
            .checked_add(1)
            .ok_or("could not increment mut count")?;

        Ok(self.mut_count)
    }

    /// Untrack an existing mutable reference.
    ///
    /// Returns the new total number of mutable references.
    ///
    /// This fails when:
    /// - There are currently no mutable references.
    /// - There is a mutable reference, but it's inaccessible.
    pub fn decrement_mut(&mut self) -> Result<usize, BorrowStateErr> {
        self.ensure_not_poisoned()?;

        if self.mut_count == 0 {
            return Err("cannot decrement mutable counter when no mutable reference exists".into());
        }

        if self.mut_count == self.inaccessible_count {
            return Err(
                "cannot decrement mutable counter when current mutable reference is inaccessible"
                    .into(),
            );
        }

        if self.mut_count - 1 != self.inaccessible_count {
            self.poison("`inaccessible_count` does not fit its invariant")?;
        }

        // We know `mut_count` isn't 0.
        self.mut_count -= 1;

        Ok(self.mut_count)
    }

    /// Set the current mutable reference as inaccessible.
    ///
    /// Returns the new total of inaccessible mutable references.
    ///
    /// Fails when:
    /// - There is no current mutable reference that can be promoted to inaccessible.
    pub fn set_inaccessible(&mut self) -> Result<usize, BorrowStateErr> {
        if !self.has_accessible() {
            return Err(
                "cannot set current reference as inaccessible when no accessible reference exists"
                    .into(),
            );
        }

        self.inaccessible_count = self
            .inaccessible_count
            .checked_add(1)
            .ok_or("could not increment inaccessible count")?;

        Ok(self.inaccessible_count)
    }

    pub(crate) fn may_unset_inaccessible(&self) -> bool {
        !self.has_accessible() && self.shared_count() == 0 && self.inaccessible_count > 0
    }

    pub fn unset_inaccessible(&mut self) -> Result<usize, BorrowStateErr> {
        if self.has_accessible() {
            return Err("cannot set current reference as accessible when an accessible mutable reference already exists".into());
        }

        if self.shared_count() > 0 {
            return Err(
                "cannot set current reference as accessible when a shared reference exists".into(),
            );
        }

        if self.inaccessible_count == 0 {
            return Err(
                "cannot mark mut pointer as accessible when there are no inaccessible pointers"
                    .into(),
            );
        }

        self.inaccessible_count = self
            .inaccessible_count
            .checked_sub(1)
            .ok_or("could not decrement inaccessible count")?;

        Ok(self.inaccessible_count)
    }
}

impl Default for BorrowState {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Eq, PartialEq, Debug)]
pub enum BorrowStateErr {
    Poisoned(String),
    IsPoisoned,
    Custom(String),
}

impl std::fmt::Display for BorrowStateErr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BorrowStateErr::Poisoned(err) => write!(f, "the borrow state was poisoned: {err}"),
            BorrowStateErr::IsPoisoned => write!(f, "the borrow state is poisoned"),
            BorrowStateErr::Custom(err) => f.write_str(err),
        }
    }
}

impl std::error::Error for BorrowStateErr {}

impl<'a> From<&'a str> for BorrowStateErr {
    fn from(value: &'a str) -> Self {
        Self::Custom(value.to_string())
    }
}

impl From<String> for BorrowStateErr {
    fn from(value: String) -> Self {
        Self::Custom(value)
    }
}

#[cfg(test)]
mod test {
    use std::collections::HashSet;

    use super::*;

    /// `(shared, mut, inaccessible)` counts; uniquely identifies a non-poisoned `BorrowState`.
    type State = (usize, usize, usize);

    /// Caps each counter so the unbounded increment ops yield a finite, terminating search. 2 lets exploration cross the 1<->2 boundary,
    /// catching any accidental special-casing of "exactly one"; impl only branches on `== 0` / `> 0` / `has_accessible`, so 3+ adds nothing.
    const MAX_COUNT: usize = 2;

    /// One operation under test, with an independent model of when it succeeds, how state changes, and what it returns.
    struct Op {
        name: &'static str,
        /// The real method under test, referenced directly.
        method: fn(&mut BorrowState) -> Result<usize, BorrowStateErr>,
        /// Independently restated success condition, validated against the real implementation.
        expected_success: fn(&BorrowState) -> bool,
        /// Field change applied to a model clone on success.
        expected_change: fn(&mut BorrowState),
        /// Counter the method returns on success, read from the post-op state.
        expected_result: fn(&BorrowState) -> usize,
    }

    const OPS: &[Op] = &[
        Op {
            name: "increment_shared",
            method: BorrowState::increment_shared,
            expected_success: |s| !s.has_accessible(),
            expected_change: |s| s.shared_count += 1,
            expected_result: |s| s.shared_count,
        },
        Op {
            name: "decrement_shared",
            method: BorrowState::decrement_shared,
            expected_success: |s| s.shared_count > 0,
            expected_change: |s| s.shared_count -= 1,
            expected_result: |s| s.shared_count,
        },
        Op {
            name: "increment_mut",
            method: BorrowState::increment_mut,
            expected_success: |s| !s.has_accessible() && s.shared_count == 0,
            expected_change: |s| s.mut_count += 1,
            expected_result: |s| s.mut_count,
        },
        Op {
            name: "decrement_mut",
            method: BorrowState::decrement_mut,
            expected_success: |s| s.has_accessible(),
            expected_change: |s| s.mut_count -= 1,
            expected_result: |s| s.mut_count,
        },
        Op {
            name: "set_inaccessible",
            method: BorrowState::set_inaccessible,
            expected_success: |s| s.has_accessible(),
            expected_change: |s| s.inaccessible_count += 1,
            expected_result: |s| s.inaccessible_count,
        },
        Op {
            name: "unset_inaccessible",
            method: BorrowState::unset_inaccessible,
            expected_success: |s| {
                !s.has_accessible() && s.shared_count == 0 && s.inaccessible_count > 0
            },
            expected_change: |s| s.inaccessible_count -= 1,
            expected_result: |s| s.inaccessible_count,
        },
    ];

    fn build((shared, mutable, inaccessible): State) -> BorrowState {
        BorrowState {
            shared_count: shared,
            mut_count: mutable,
            inaccessible_count: inaccessible,
            poisoned: false,
        }
    }

    /// Exhaustively explores every state reachable from `new()` (counts bounded by [`MAX_COUNT`]) and validates all operations.
    #[test]
    fn exhaustive_model_check() {
        // DFS reachability traversal (LIFO `Vec`), not nested `0..=MAX_COUNT` loops: the latter would visit invalid
        // states like `(mut=0, inaccessible=2)`, where `has_accessible()` underflows `mut_count - inaccessible_count`.
        let mut visited = HashSet::from([(0, 0, 0)]);
        let mut queue = vec![(0, 0, 0)];

        while let Some(state) = queue.pop() {
            let initial = build(state);

            // Structural invariants that must hold for every reachable state.
            assert!(
                !initial.is_poisoned(),
                "reachable state {state:?} is poisoned"
            );
            assert!(
                initial.mut_count - initial.inaccessible_count <= 1,
                "more than one accessible mut: {state:?}"
            );
            assert!(
                !(initial.shared_count > 0 && initial.has_accessible()),
                "shared and accessible mut coexist: {state:?}"
            );

            for op in OPS {
                let name = op.name;

                let mut actual = initial.clone();
                let result = (op.method)(&mut actual);
                let succeeded = result.is_ok();

                assert_eq!(
                    succeeded,
                    (op.expected_success)(&initial),
                    "`{name}` success mismatch in {state:?}"
                );

                // On success: exactly the documented field delta and the post-op counter returned. On failure: no change at all.
                let mut expected = initial.clone();
                if succeeded {
                    (op.expected_change)(&mut expected);
                    let expected_result = Ok((op.expected_result)(&expected));
                    assert_eq!(
                        result, expected_result,
                        "`{name}` wrong return value in {state:?}"
                    );
                }
                assert_eq!(
                    actual, expected,
                    "`{name}` produced wrong state in {state:?}"
                );
                assert!(!actual.is_poisoned(), "`{name}` poisoned state {state:?}");

                // Explore successful transitions within the modeled bound.
                let next = (
                    actual.shared_count,
                    actual.mut_count,
                    actual.inaccessible_count,
                );
                if succeeded && next.0 <= MAX_COUNT && next.1 <= MAX_COUNT && visited.insert(next) {
                    queue.push(next);
                }
            }
        }
    }

    #[test]
    fn poisoned_unset_shared_ref() {
        let mut state = BorrowState::new();
        assert!(!state.is_poisoned());

        for step in [
            BorrowState::increment_mut,
            BorrowState::set_inaccessible,
            BorrowState::increment_shared,
            BorrowState::unset_inaccessible,
            BorrowState::increment_shared,
            BorrowState::decrement_shared,
        ] {
            _ = step(&mut state);
            assert!(!state.is_poisoned());
        }
    }
}
