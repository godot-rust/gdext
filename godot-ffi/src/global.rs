/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::sync::{Mutex, MutexGuard, TryLockError};

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
/// - Completely safe usage. Almost completely safe implementation (besides `unreachable_unchecked`).
///
/// There are two `const` methods for construction: [`new()`](Self::new) and [`default()`](Self::default).
/// For access, you should primarily use [`lock()`](Self::lock). There is also [`try_lock()`](Self::try_lock) for special cases.
pub struct Global<T> {
    // When needed, this could be changed to use RwLock and separate read/write guards.
    value: Mutex<InitState<T>>,
}

impl<T> Global<T> {
    /// Create `Global<T>`, providing a lazy initialization function.
    ///
    /// The initialization function is only called once, when the global is first accessed through [`lock()`](Self::lock).
    pub const fn new(init_fn: fn() -> T) -> Self {
        // Note: could be generalized to F: FnOnce() -> T + Send. See also once_cell::Lazy<T, F>.
        Self {
            value: Mutex::new(InitState::Pending(init_fn)),
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
        let mutex_guard = self
            .value
            .lock()
            .expect("Global<T> poisoned; a thread has panicked while holding a lock to it");

        let guard = Self::ensure_init(mutex_guard, true)
            .unwrap_or_else(|| panic!("previous Global<T> initialization failed due to panic"));

        guard
    }

    /// Non-panicking access with error introspection.
    pub fn try_lock(&self) -> Result<GlobalGuard<'_, T>, GlobalLockError<'_, T>> {
        let guard = match self.value.try_lock() {
            Ok(mutex_guard) => Self::ensure_init(mutex_guard, false),
            Err(TryLockError::WouldBlock) => {
                return Err(GlobalLockError::WouldBlock);
            }
            Err(TryLockError::Poisoned(poisoned)) => {
                return Err(GlobalLockError::Poisoned {
                    // We can likely use `new_unchecked` here, but verifying that it's safe would need somewhat tricky reasoning.
                    // Since this error condition isn't very common, it is likely not very important to optimize access to the value here.
                    // Especially since most users will likely not want to access it anyway.
                    circumvent: GlobalGuard::new(poisoned.into_inner())
                        .expect("Poisoned global guard should always be initialized"),
                });
            }
        };

        guard.ok_or(GlobalLockError::InitFailed)
    }

    fn ensure_init(
        mut mutex_guard: MutexGuard<'_, InitState<T>>,
        may_panic: bool,
    ) -> Option<GlobalGuard<'_, T>> {
        let init_fn = match &mut *mutex_guard {
            InitState::Initialized(_) => {
                // SAFETY: `mutex_guard` is `Initialized`.
                return Some(unsafe { GlobalGuard::new_unchecked(mutex_guard) });
            }
            InitState::Failed => {
                return None;
            }
            InitState::Pending(init_fn) => init_fn,
        };

        // Unwinding should be safe here, as there is no unsafe code relying on it.
        let init_fn = std::panic::AssertUnwindSafe(init_fn);
        match std::panic::catch_unwind(init_fn) {
            Ok(value) => *mutex_guard = InitState::Initialized(value),
            Err(e) => {
                eprintln!("panic during Global<T> initialization");
                *mutex_guard = InitState::Failed;

                if may_panic {
                    std::panic::resume_unwind(e);
                } else {
                    // Note: this currently swallows panic.
                    return None;
                }
            }
        };

        // SAFETY: `mutex_guard` was either set to `Initialized` above, or we returned from the function.
        Some(unsafe { GlobalGuard::new_unchecked(mutex_guard) })
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Guards

// Encapsulate private fields.
mod global_guard {
    use std::ops::{Deref, DerefMut};
    use std::sync::MutexGuard;

    use super::InitState;

    /// Guard that temporarily gives access to a `Global<T>`'s inner value.
    pub struct GlobalGuard<'a, T> {
        // Safety invariant: Is `Initialized`.
        mutex_guard: MutexGuard<'a, InitState<T>>,
    }

    impl<'a, T> GlobalGuard<'a, T> {
        pub(super) fn new(mutex_guard: MutexGuard<'a, InitState<T>>) -> Option<Self> {
            match &*mutex_guard {
                InitState::Initialized(_) => Some(Self { mutex_guard }),
                _ => None,
            }
        }

        /// # Safety
        ///
        /// The value must be `Initialized`.
        pub(super) unsafe fn new_unchecked(mutex_guard: MutexGuard<'a, InitState<T>>) -> Self {
            debug_assert!(matches!(*mutex_guard, InitState::Initialized(_)));

            Self::new(mutex_guard).unwrap_unchecked()
        }
    }

    impl<T> Deref for GlobalGuard<'_, T> {
        type Target = T;

        fn deref(&self) -> &Self::Target {
            // SAFETY: `self` is `Initialized`.
            unsafe { self.mutex_guard.as_initialized().unwrap_unchecked() }
        }
    }

    impl<T> DerefMut for GlobalGuard<'_, T> {
        fn deref_mut(&mut self) -> &mut Self::Target {
            // SAFETY: `self` is `Initialized`.
            unsafe { self.mutex_guard.as_initialized_mut().unwrap_unchecked() }
        }
    }
}

pub use global_guard::GlobalGuard;

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Errors

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
// Internals

enum InitState<T> {
    Initialized(T),
    Pending(fn() -> T),
    Failed,
}

impl<T> InitState<T> {
    fn as_initialized(&self) -> Option<&T> {
        match self {
            InitState::Initialized(t) => Some(t),
            _ => None,
        }
    }

    fn as_initialized_mut(&mut self) -> Option<&mut T> {
        match self {
            InitState::Initialized(t) => Some(t),
            _ => None,
        }
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Tests

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

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
