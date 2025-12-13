/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! # Internal crate of [**godot-rust**](https://godot-rust.github.io)
//!
//! Do not depend on this crate directly, instead use the `godot` crate.
//! No SemVer or other guarantees are provided.
//!
//! # Contributor docs
//!
//! A re-entrant cell implementation which allows for `&mut` references to be reborrowed even while `&mut`
//! references still exist.
//!
//! This is done by ensuring any existing `&mut` references cannot alias the new reference, and that the new
//! reference is derived from the previous one.
//!
//! This emulates rust's system for function calls. i.e `my_func(&mut borrowed)` creates a second `&mut`
//! reference inside the function.
//!
//! Instead of directly using the concept of `aliasing` pointers, we use the term `accessible` instead. A
//! reference (or other pointer) to some value is considered accessible when it is possible to either read
//! from or write to the value it points to without using `unsafe`. Importantly, if we know that a reference
//! `a` is inaccessible, and then we create a new reference `b` derived from `a` to the same value, then we
//! know for sure that `b` won't alias `a`. This is because aliasing in rust is based on accesses, and if we
//! never access `a` then we cannot ever violate aliasing for `a` and `b`. And since `b` is derived from `a`
//! (that is, `b` was created from `a` somehow such as by casting `a` to a raw pointer then to a reference
//! `b`), then `a` won't get invalidated by accesses to `b`.

mod blocking_cell;
mod blocking_guards;
mod borrow_state;
mod cell;
mod guards;

pub mod panicking {
    pub use crate::cell::GdCell;
    pub use crate::guards::{InaccessibleGuard, MutGuard, RefGuard};
}

pub mod blocking {
    pub use crate::blocking_cell::GdCellBlocking as GdCell;
    pub use crate::blocking_guards::{
        InaccessibleGuardBlocking as InaccessibleGuard, MutGuardBlocking as MutGuard,
        RefGuardBlocking as RefGuard,
    };
}
