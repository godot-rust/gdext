/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::cell::OnceCell;
use std::sync::{Mutex, MutexGuard, PoisonError, TryLockError};

/// Ergonomic global variables.
///
/// No more `Mutex<Option<...>>` shenanigans with lazy initialization on each use site, or `OnceLock` which limits to immutable access.
///
/// This type is very similar to [`once_cell::Lazy`](https://docs.rs/once_cell/latest/once_cell/sync/struct.Lazy.html) in its nature,
/// with a minimalistic implementation. Unlike `Lazy`, it is only designed for global variables, not for local lazy initialization
/// (following "do one thing and do it well").
///
/// `Global<T>` features:
/// - `const` constructors, allowing to be used in `static` variables without `Option`.
/// - Initialization function provided in constructor, not in each use site separately.
/// - Ergonomic access through guards to both `&T` and `&mut T`.
/// - Completely safe usage. Little use of `unsafe` in the implementation (for performance reasons).
///
/// There are two `const` methods for construction: [`new()`](Self::new) and [`default()`](Self::default).
/// For access, you should primarily use [`lock()`](Self::lock). There is also [`try_lock()`](Self::try_lock) for special cases.
pub struct Global<T> {
    // When needed, this could be changed to use RwLock and separate read/write guards.
    value: Mutex<OnceCell<T>>,
    init_fn: fn() -> T,
}

impl<T> Global<T> {
    /// Create `Global<T>`, providing a lazy initialization function.
    ///
    /// The initialization function is only called once, when the global is first accessed through [`lock()`](Self::lock).
    pub const fn new(init_fn: fn() -> T) -> Self {
        // Note: could be generalized to F: FnOnce() -> T + Send. See also once_cell::Lazy<T, F>.
        Self {
            value: Mutex::new(OnceCell::new()),
            init_fn,
        }
    }

    /// Create `Global<T>` with `T::default()` as initialization function.
    ///
    /// This is inherent rather than implementing the `Default` trait, because the latter is not `const` and thus useless in static contexts.
    pub const fn default() -> Self
    where
        T: Default,
    {
        Self::new(T::default)
    }

    /// Returns a guard that gives shared or mutable access to the value.
    ///
    /// Blocks until the internal mutex is available.
    ///
    /// # Panics
    /// If the initialization function panics. Once that happens, the global is considered poisoned and all future calls to `lock()` will panic.
    /// This can currently not be recovered from.
    pub fn lock(&self) -> GlobalGuard<'_, T> {
        let guard = self.value.lock().unwrap_or_else(PoisonError::into_inner);
        guard.get_or_init(self.init_fn);

        // SAFETY: `get_or_init()` has already panicked if it wants to, propagating the panic to its caller,
        // so the object is guaranteed to be initialized.
        unsafe { GlobalGuard::new_unchecked(guard) }
    }

    /// Non-blocking access with error introspection.
    pub fn try_lock(&self) -> Result<GlobalGuard<'_, T>, GlobalLockError<'_, T>> {
        /// Initializes the cell and returns a guard.
        fn init<'mutex: 'cell, 'cell, T>(
            g: MutexGuard<'mutex, OnceCell<T>>,
            init_fn: fn() -> T,
        ) -> Result<GlobalGuard<'cell, T>, GlobalLockError<'cell, T>> {
            // Initialize the cell.
            std::panic::catch_unwind(|| g.get_or_init(init_fn))
                .map_err(|_| GlobalLockError::InitFailed)?;

            // SAFETY: `get_or_init()` has already panicked if it wants to, which has been successfully unwound,
            // therefore the object is guaranteed to be initialized.
            Ok(unsafe { GlobalGuard::new_unchecked(g) })
        }

        match self.value.try_lock() {
            Ok(guard) => init(guard, self.init_fn),
            Err(TryLockError::WouldBlock) => Err(GlobalLockError::WouldBlock),

            // This is a cold branch, where the initialization function panicked.
            Err(TryLockError::Poisoned(x)) => {
                // We do the same things as in the hot branch.
                let circumvent = init(x.into_inner(), self.init_fn)?;
                Err(GlobalLockError::Poisoned { circumvent })
            }
        }
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Guards

// Encapsulate private fields.
mod global_guard {
    use std::ops::{Deref, DerefMut};

    use super::*;

    /// Guard that temporarily gives access to a `Global<T>`'s inner value.
    pub struct GlobalGuard<'a, T> {
        // Safety invariant: `OnceCell` has been initialized.
        mutex_guard: MutexGuard<'a, OnceCell<T>>,
    }

    impl<'a, T> GlobalGuard<'a, T> {
        pub(super) fn new(mutex_guard: MutexGuard<'a, OnceCell<T>>) -> Option<Self> {
            // Use an eager map instead of `mutex_guard.get().map(|_| Self { mutex_guard })`
            // as `.get().map(…)` tries to move `mutex_guard` while borrowing an ignored value.
            match mutex_guard.get() {
                Some(_) => Some(Self { mutex_guard }),
                _ => None,
            }
        }

        /// # Safety
        ///
        /// The value must be initialized.
        pub(super) unsafe fn new_unchecked(mutex_guard: MutexGuard<'a, OnceCell<T>>) -> Self {
            crate::strict_assert!(
                mutex_guard.get().is_some(),
                "safety precondition violated: cell not initialized"
            );
            Self::new(mutex_guard).unwrap_unchecked()
        }
    }

    impl<T> Deref for GlobalGuard<'_, T> {
        type Target = T;
        fn deref(&self) -> &Self::Target {
            // SAFETY: `GlobalGuard` guarantees that the cell is initialized.
            unsafe { self.mutex_guard.get().unwrap_unchecked() }
        }
    }

    impl<T> DerefMut for GlobalGuard<'_, T> {
        fn deref_mut(&mut self) -> &mut Self::Target {
            // SAFETY: `GlobalGuard` guarantees that the cell is initialized.
            unsafe { self.mutex_guard.get_mut().unwrap_unchecked() }
        }
    }
}

pub use global_guard::GlobalGuard;

/// Guard that temporarily gives access to a `Global<T>`'s inner value.
pub enum GlobalLockError<'a, T> {
    /// The mutex is currently locked by another thread.
    WouldBlock,

    /// A panic occurred while a lock was held. This excludes initialization errors.
    Poisoned { circumvent: GlobalGuard<'a, T> },

    /// The initialization function panicked.
    InitFailed,
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Tests

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    static MAP: Global<HashMap<i32, &'static str>> = Global::default();
    static VEC: Global<Vec<i32>> = Global::new(|| vec![1, 2, 3]);
    static FAILED: Global<()> = Global::new(|| panic!("failed"));
    static POISON: Global<i32> = Global::new(|| 36);

    #[test]
    fn test_global_map() {
        {
            let mut map = MAP.lock();
            map.insert(2, "two");
            map.insert(3, "three");
        }

        {
            let mut map = MAP.lock();
            map.insert(1, "one");
        }

        let map = MAP.lock();
        assert_eq!(map.len(), 3);
        assert_eq!(map.get(&1), Some(&"one"));
        assert_eq!(map.get(&2), Some(&"two"));
        assert_eq!(map.get(&3), Some(&"three"));
    }

    #[test]
    fn test_global_vec() {
        {
            let mut vec = VEC.lock();
            vec.push(4);
        }

        let vec = VEC.lock();
        assert_eq!(*vec, &[1, 2, 3, 4]);
    }

    #[test]
    fn test_global_would_block() {
        let vec = VEC.lock();

        let vec2 = VEC.try_lock();
        assert!(matches!(vec2, Err(GlobalLockError::WouldBlock)));
    }

    #[test]
    fn test_global_init_failed() {
        let guard = FAILED.try_lock();
        assert!(matches!(guard, Err(GlobalLockError::InitFailed)));

        // Subsequent access still returns same error.
        let guard = FAILED.try_lock();
        assert!(matches!(guard, Err(GlobalLockError::InitFailed)));
    }

    #[test]
    fn test_global_poison() {
        let result = std::panic::catch_unwind(|| {
            let guard = POISON.lock();
            panic!("poison injection");
        });
        assert!(result.is_err());

        let guard = POISON.try_lock();
        let Err(GlobalLockError::Poisoned { circumvent }) = guard else {
            panic!("expected GlobalLockError::Poisoned");
        };
        assert_eq!(*circumvent, 36);
    }
}
