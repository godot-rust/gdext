/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::mem;

/// Ergonomic late-initialization container.
///
/// While deferred initialization is generally seen as bad practice, it is often inevitable in game development.
/// Godot in particular encourages initialization inside `ready()`, e.g. to access the scene tree after a node is inserted into it.
///
/// `Late<T>` is a mix of [`Option<T>`][std::option::Option] and [`Lazy<T>`](https://docs.rs/once_cell/1/once_cell/unsync/struct.Lazy.html).
///
/// # Example
/// ```
/// use godot::obj::LateInit;
///
/// // Inside init():
/// let mut l = LateInit::<i32>::new(|| -42);
///
/// // Inside ready():
/// l.init();
/// assert_eq!(*l, -42); // uses Deref
/// assert_eq!(l.abs(), 42); // method calls look like direct access
/// ```
pub struct LateInit<T> {
    state: InitState<T>,
}

impl<T> LateInit<T> {
    /// Creates a new container with a closure that initializes the value.
    pub fn new<F>(init_fn: F) -> Self
    where
        F: FnOnce() -> T + 'static,
    {
        Self {
            state: InitState::Uninitialized {
                initializer: Box::new(init_fn),
            },
        }
    }

    /// Runs initialization.
    ///
    /// # Panics
    /// If the value is already initialized.
    pub fn init(&mut self) {
        // Two branches needed, because mem::replace() could accidentally overwrite an already initialized value.
        match &self.state {
            InitState::Uninitialized { .. } => {}
            InitState::Initialized { .. } => panic!("value already initialized"),
            InitState::Loading => {
                // SAFETY: Loading is ephemeral state that is only set below and immediately overwritten.
                unsafe { std::hint::unreachable_unchecked() }
            }
        };

        // Temporarily replace with dummy state, as it's not possible to take ownership of the initializer closure otherwise.
        let InitState::Uninitialized { initializer } =
            mem::replace(&mut self.state, InitState::Loading)
        else {
            // SAFETY: condition checked above.
            unsafe { std::hint::unreachable_unchecked() }
        };

        self.state = InitState::Initialized {
            value: initializer(),
        };
    }
}

// Panicking Deref is not best practice according to Rust, but constant get() calls are significantly less ergonomic and make it harder to
// migrate between T and LateInit<T>, because all the accesses need to change.
impl<T> std::ops::Deref for LateInit<T> {
    type Target = T;

    /// Returns a shared reference to the value.
    ///
    /// # Panics
    /// If the value is not yet initialized.
    fn deref(&self) -> &Self::Target {
        match &self.state {
            InitState::Initialized { value } => value,
            InitState::Uninitialized { .. } => panic!("value not yet initialized"),
            InitState::Loading => unreachable!(),
        }
    }
}

impl<T> std::ops::DerefMut for LateInit<T> {
    /// Returns an exclusive reference to the value.
    ///     
    /// # Panics
    /// If the value is not yet initialized.
    fn deref_mut(&mut self) -> &mut Self::Target {
        match &mut self.state {
            InitState::Initialized { value } => value,
            InitState::Uninitialized { .. } => panic!("value not yet initialized"),
            InitState::Loading => unreachable!(),
        }
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

enum InitState<T> {
    Uninitialized { initializer: Box<dyn FnOnce() -> T> },
    Initialized { value: T },
    Loading, // needed because state cannot be empty
}
