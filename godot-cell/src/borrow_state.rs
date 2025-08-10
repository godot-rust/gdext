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
    /// - There is no current
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
        Self::Custom(value.into())
    }
}

impl From<String> for BorrowStateErr {
    fn from(value: String) -> Self {
        Self::Custom(value)
    }
}

#[cfg(all(test, feature = "proptest"))]
mod proptests {
    use proptest::arbitrary::Arbitrary;
    use proptest::collection::vec;
    use proptest::prelude::*;

    use super::*;

    impl BorrowState {
        fn has_shared_reference(&self) -> bool {
            self.shared_count > 0
        }
    }

    #[derive(Copy, Clone, Eq, PartialEq, Debug)]
    enum Operation {
        IncShared,
        DecShared,
        IncMut,
        DecMut,
        SetInaccessible,
        UnsetInaccessible,
    }

    impl Operation {
        fn execute(&self, state: &mut BorrowState) -> Result<(), BorrowStateErr> {
            use Operation as Op;

            let result = match self {
                Op::IncShared => state.increment_shared(),
                Op::DecShared => state.decrement_shared(),
                Op::IncMut => state.increment_mut(),
                Op::DecMut => state.decrement_mut(),
                Op::SetInaccessible => state.set_inaccessible(),
                Op::UnsetInaccessible => state.unset_inaccessible(),
            };

            result.map(|_| ())
        }
    }

    prop_compose! {
        fn arbitrary_op()(id in 0..6) -> Operation {
            use Operation as Op;

            match id {
                0 => Op::IncShared,
                1 => Op::DecShared,
                2 => Op::IncMut,
                3 => Op::DecMut,
                4 => Op::SetInaccessible,
                5 => Op::UnsetInaccessible,
                _ => unreachable!()
            }
        }
    }

    impl Arbitrary for Operation {
        type Parameters = ();

        type Strategy = BoxedStrategy<Self>;

        fn arbitrary_with(_: Self::Parameters) -> Self::Strategy {
            Strategy::boxed(arbitrary_op())
        }
    }

    #[derive(Clone, Eq, PartialEq, Debug)]
    struct OperationExecutor {
        vec: Vec<Operation>,
    }

    impl OperationExecutor {
        fn execute_all(&self, state: &mut BorrowState) {
            for op in self.vec.iter() {
                _ = op.execute(state);
            }
        }

        fn remove_shared_inc_dec_pairs(mut self) -> Self {
            loop {
                let mut inc_index = None;
                let mut just_saw_inc = false;

                for (i, op) in self.vec.iter().enumerate() {
                    match op {
                        Operation::IncShared => just_saw_inc = true,
                        Operation::DecShared if just_saw_inc => {
                            inc_index = Some(i - 1);
                            break;
                        }
                        _ => just_saw_inc = false,
                    }
                }

                match inc_index {
                    Some(i) => {
                        self.vec.remove(i + 1);
                        self.vec.remove(i);
                    }
                    None => break,
                }
            }

            self
        }
    }

    impl From<Vec<Operation>> for OperationExecutor {
        fn from(vec: Vec<Operation>) -> Self {
            Self { vec }
        }
    }

    prop_compose! {
        fn arbitrary_ops(max_len: usize)(len in 0..max_len)(operations in vec(any::<Operation>(), len)) -> Vec<Operation> {
            operations
        }
    }

    proptest! {
        #[test]
        fn operations_do_only_whats_expected_or_nothing(operations in arbitrary_ops(50)) {
            use Operation as Op;
            let mut state = BorrowState::new();
            for op in operations {
                let expected_on_success = match op {
                    Op::IncShared => |mut original: BorrowState| {
                        original.shared_count += 1;
                        original
                    },
                    Op::DecShared => |mut original: BorrowState| {
                        original.shared_count -= 1;
                        original
                    },
                    Op::IncMut => |mut original: BorrowState| {
                        original.mut_count += 1;
                        original
                    },
                    Op::DecMut => |mut original: BorrowState| {
                        original.mut_count -= 1;
                        original
                    },
                    Op::SetInaccessible => |mut original: BorrowState| {
                        original.inaccessible_count += 1;
                        original
                    },
                    Op::UnsetInaccessible => |mut original: BorrowState| {
                        original.inaccessible_count -= 1;
                        original
                    },
                };

                let original = state.clone();
                if op.execute(&mut state).is_ok() {
                    assert_eq!(state, expected_on_success(original));
                } else {
                    assert_eq!(state, original);
                }
            }
        }
    }

    proptest! {
        #[test]
        fn no_poison(operations in arbitrary_ops(50)) {
            let mut state = BorrowState::new();
            for op in operations {
                if let Err(err) = op.execute(&mut state) {
                    assert_ne!(err, BorrowStateErr::IsPoisoned);
                    assert!(!matches!(err, BorrowStateErr::Poisoned(_)));
                }

                assert!(!state.is_poisoned());
            }
        }
    }

    proptest! {
        #[test]
        fn no_shared_and_mut(operations in arbitrary_ops(50)) {
            let mut state = BorrowState::new();
            for op in operations {
                _ = op.execute(&mut state);
                if state.has_shared_reference() {
                    assert!(!state.has_accessible())
                }
            }
        }
    }

    proptest! {
        #[test]
        fn can_borrow_shared_when_borrowed_shared(operations in arbitrary_ops(50)) {
            let mut state = BorrowState::new();

            for op in operations {
                _ = op.execute(&mut state);
                if state.has_shared_reference() {
                    assert!(state.increment_shared().is_ok());
                    assert!(state.decrement_shared().is_ok());
                }
            }
        }
    }

    proptest! {
        #[test]
        fn cannot_borrow_shared_when_borrowed_accessible(operations in arbitrary_ops(50)) {
            let mut state = BorrowState::new();

            for op in operations {
                _ = op.execute(&mut state);
                if state.has_accessible() {
                    assert!(state.increment_shared().is_err());
                }
            }
        }
    }

    proptest! {
        #[test]
        fn can_borrow_shared_when_not_borrowed_accessible(operations in arbitrary_ops(50)) {
            let mut state = BorrowState::new();

            for op in operations {
                _ = op.execute(&mut state);
                if !state.has_accessible() {
                    assert!(state.increment_shared().is_ok());
                    assert!(state.decrement_shared().is_ok());
                }
            }
        }
    }

    proptest! {
        #[test]
        fn can_borrow_mut_when_no_shared_and_no_accessible(operations in arbitrary_ops(50)) {
            let mut state = BorrowState::new();

            for op in operations {
                _ = op.execute(&mut state);
                if !state.has_accessible() && !state.has_shared_reference() {
                    assert!(state.increment_mut().is_ok());
                    assert!(state.decrement_mut().is_ok());
                }
            }
        }
    }

    proptest! {
        #[test]
        fn cannot_borrow_mut_when_shared(operations in arbitrary_ops(50)) {
            let mut state = BorrowState::new();

            for op in operations {
                _ = op.execute(&mut state);
                if state.has_shared_reference() {
                    assert!(state.increment_mut().is_err());
                }
            }
        }
    }

    proptest! {
        #[test]
        fn cannot_borrow_mut_when_has_accessible(operations in arbitrary_ops(50)) {
            let mut state = BorrowState::new();

            for op in operations {
                _ = op.execute(&mut state);
                if state.has_accessible() {
                    assert!(state.increment_mut().is_err());
                }
            }
        }
    }

    proptest! {
        #[test]
        fn can_set_inaccessible_when_accessible(operations in arbitrary_ops(50)) {
            let mut state = BorrowState::new();

            for op in operations {
                _ = op.execute(&mut state);
                if state.has_accessible() {
                    assert!(state.set_inaccessible().is_ok());
                    assert!(state.unset_inaccessible().is_ok());
                }
            }
        }
    }

    proptest! {
        #[test]
        fn cannot_set_inaccessible_when_shared(operations in arbitrary_ops(50)) {
            let mut state = BorrowState::new();

            for op in operations {
                _ = op.execute(&mut state);
                if state.has_shared_reference() {
                    assert!(state.set_inaccessible().is_err());
                }
            }
        }
    }

    proptest! {
        #[test]
        fn cannot_set_inaccessible_when_inaccessible(operations in arbitrary_ops(50)) {
            let mut state = BorrowState::new();

            for op in operations {
                _ = op.execute(&mut state);
                if !state.has_accessible() {
                    assert!(state.set_inaccessible().is_err());
                }
            }
        }
    }

    proptest! {
        #[test]
        fn remove_shared_inc_dec_pairs_is_noop(operations in arbitrary_ops(50)) {
            let mut state_all = BorrowState::new();
            let executor_all = OperationExecutor::from(operations);
            executor_all.execute_all(&mut state_all);

            let mut state_no_shared_pairs = BorrowState::new();
            let executor_no_shared_pairs = executor_all.clone().remove_shared_inc_dec_pairs();
            executor_no_shared_pairs.execute_all(&mut state_no_shared_pairs);

            assert_eq!(state_all, state_no_shared_pairs);
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn poisoned_unset_shared_ref() {
        let mut state = BorrowState::new();
        assert!(!state.is_poisoned());

        _ = state.increment_mut();
        assert!(!state.is_poisoned());
        _ = state.set_inaccessible();
        assert!(!state.is_poisoned());
        _ = state.increment_shared();
        assert!(!state.is_poisoned());
        _ = state.unset_inaccessible();
        assert!(!state.is_poisoned());
        _ = state.increment_shared();
        assert!(!state.is_poisoned());
        _ = state.decrement_shared();
        assert!(!state.is_poisoned());
    }
}
