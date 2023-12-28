/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::ops::{Deref, DerefMut};
use std::sync::{Mutex, MutexGuard};

/// Ergonomic global variables.
///
/// No more `Mutex<Option<...>>` shenanigans with lazy initialization on each use site, or `OnceLock` which limits to immutable access.
///
/// This type is very similar to [`once_cell::Lazy`](https://docs.rs/once_cell/latest/once_cell/sync/struct.Lazy.html) in its nature,
/// with a minimalistic implementation. It features:
/// - A `const` constructor, allowing to be used in `static` variables without `Option`.
/// - Initialization function provided in constructor, not in each use site separately.
/// - Ergonomic access through guards to both `&T` and `&mut T`.
/// - Completely safe usage. Almost completely safe implementation (besides `unreachable_unchecked`).
///
/// There are two main methods: [`new()`](Self::new) and [`lock()`](Self::lock). Additionally, [`default()`](Self::default) is provided
/// for convenience.
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
        let guard = self.ensure_init();
        debug_assert!(matches!(*guard, InitState::Initialized(_)));

        GlobalGuard { guard }
    }

    fn ensure_init(&self) -> MutexGuard<'_, InitState<T>> {
        let mut guard = self.value.lock().expect("lock poisoned");
        let pending_state = match &mut *guard {
            InitState::Initialized(_) => {
                return guard;
            }
            InitState::TransientInitializing => {
                // SAFETY: only set inside this function and all paths (panic + return) leave the enum in a different state.
                unsafe { std::hint::unreachable_unchecked() };
            }
            InitState::Failed => {
                panic!("previous Global<T> initialization failed due to panic")
            }
            state @ InitState::Pending(_) => {
                std::mem::replace(state, InitState::TransientInitializing)
            }
        };

        let InitState::Pending(init_fn) = pending_state else {
            // SAFETY: all other paths leave the function, see above.
            unsafe { std::hint::unreachable_unchecked() }
        };

        // Unwinding should be safe here, as there is no unsafe code relying on it.
        let init_fn = std::panic::AssertUnwindSafe(init_fn);
        match std::panic::catch_unwind(init_fn) {
            Ok(value) => *guard = InitState::Initialized(value),
            Err(e) => {
                eprintln!("panic during Global<T> initialization");
                *guard = InitState::Failed;
                std::panic::resume_unwind(e);
            }
        };

        guard
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Guards

/// Guard that temporarily gives access to a `Global<T>`'s inner value.
pub struct GlobalGuard<'a, T> {
    guard: MutexGuard<'a, InitState<T>>,
}

impl<T> Deref for GlobalGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.guard.unwrap_ref()
    }
}

impl<T> DerefMut for GlobalGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.guard.unwrap_mut()
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Internals

enum InitState<T> {
    Initialized(T),
    Pending(fn() -> T),
    TransientInitializing,
    Failed,
}

impl<T> InitState<T> {
    fn unwrap_ref(&self) -> &T {
        match self {
            InitState::Initialized(t) => t,
            _ => {
                // SAFETY: This method is only called from a guard, which can only be obtained in Initialized state.
                unsafe { std::hint::unreachable_unchecked() }
            }
        }
    }

    fn unwrap_mut(&mut self) -> &mut T {
        match self {
            InitState::Initialized(t) => t,
            _ => {
                // SAFETY: This method is only called from a guard, which can only be obtained in Initialized state.
                unsafe { std::hint::unreachable_unchecked() }
            }
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
}
