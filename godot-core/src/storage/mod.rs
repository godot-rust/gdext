/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

mod instance_storage;
#[cfg_attr(not(feature = "experimental-threads"), allow(dead_code))]
mod multi_threaded;
#[cfg_attr(feature = "experimental-threads", allow(dead_code))]
mod single_threaded;

use godot_ffi::out;
pub use instance_storage::*;
use std::any::type_name;

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Shared code for submodules

fn bind_failed<T>(err: Box<dyn std::error::Error>, tracker: &DebugBorrowTracker) -> ! {
    let ty = type_name::<T>();

    eprint!("{tracker}");

    panic!(
        "Gd<T>::bind() failed, already bound; T = {ty}.\n  \
        Make sure to use `self.base_mut()` or `self.base()` instead of `self.to_gd()` when possible.\n  \
        Details: {err}."
    )
}

fn bind_mut_failed<T>(err: Box<dyn std::error::Error>, tracker: &DebugBorrowTracker) -> ! {
    let ty = type_name::<T>();

    eprint!("{tracker}");

    panic!(
        "Gd<T>::bind_mut() failed, already bound; T = {ty}.\n  \
        Make sure to use `self.base_mut()` instead of `self.to_gd()` when possible.\n  \
        Details: {err}."
    )
}

fn bug_inaccessible<T>(err: Box<dyn std::error::Error>) -> ! {
    // We should never hit this, except maybe in extreme cases like having more than `usize::MAX` borrows.
    let ty = type_name::<T>();

    panic!(
        "`base_mut()` failed for type T = {ty}.\n  \
        This is most likely a bug, please report it.\n  \
        Details: {err}."
    )
}

fn log_construct<T>() {
    out!(
        "    Storage::construct             <{ty}>",
        ty = type_name::<T>()
    );
}

fn log_inc_ref<T: StorageRefCounted>(storage: &T) {
    out!(
        "    Storage::on_inc_ref (rc={rc})     <{ty}> -- {base:?}",
        rc = T::godot_ref_count(storage),
        base = storage.base(),
        ty = type_name::<T>(),
    );
}

fn log_dec_ref<T: StorageRefCounted>(storage: &T) {
    out!(
        "  | Storage::on_dec_ref (rc={rc})     <{ty}> -- {base:?}",
        rc = T::godot_ref_count(storage),
        base = storage.base(),
        ty = type_name::<T>(),
    );
}

fn log_drop<T: StorageRefCounted>(storage: &T) {
    out!(
        "    Storage::drop (rc={rc})           <{base:?}>",
        rc = storage.godot_ref_count(),
        base = storage.base(),
    );
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Tracking borrows in Debug mode

#[cfg(debug_assertions)]
use borrow_info::DebugBorrowTracker;

#[cfg(not(debug_assertions))]
use borrow_info_noop::DebugBorrowTracker;

#[cfg(debug_assertions)]
mod borrow_info {
    use std::backtrace::Backtrace;
    use std::fmt;
    use std::sync::Mutex;

    struct TrackedBorrow {
        backtrace: Backtrace,
        is_mut: bool,
    }

    /// Informational-only info about ongoing borrows.
    pub(super) struct DebugBorrowTracker {
        // Currently just tracks the last borrow. Could technically track 1 mut or N ref borrows, but would need destructor integration.
        // Also never clears it when a guard drops, assuming that once a borrow fails, there must be at least one previous borrow conflicting.
        // Is also not yet integrated with "inaccessible" borrows (reborrow through base_mut).
        last_borrow: Mutex<Option<TrackedBorrow>>,
    }

    impl DebugBorrowTracker {
        pub fn new() -> Self {
            Self {
                last_borrow: Mutex::new(None),
            }
        }

        // Currently considers RUST_BACKTRACE due to performance reasons; force_capture() can be quite slow.
        // User is expected to set the env var during debug sessions.

        #[track_caller]
        pub fn track_ref_borrow(&self) {
            let mut guard = self.last_borrow.lock().unwrap();
            *guard = Some(TrackedBorrow {
                backtrace: Backtrace::capture(),
                is_mut: false,
            });
        }

        #[track_caller]
        pub fn track_mut_borrow(&self) {
            let mut guard = self.last_borrow.lock().unwrap();
            *guard = Some(TrackedBorrow {
                backtrace: Backtrace::capture(),
                is_mut: true,
            });
        }
    }

    impl fmt::Display for DebugBorrowTracker {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            let guard = self.last_borrow.lock().unwrap();
            if let Some(borrow) = &*guard {
                let mutability = if borrow.is_mut { "bind_mut" } else { "bind" };

                let prefix = format!("backtrace of previous `{mutability}` borrow");
                let backtrace = crate::format_backtrace!(prefix, &borrow.backtrace);

                writeln!(f, "{backtrace}")
            } else {
                writeln!(f, "no previous borrows tracked.")
            }
        }
    }
}

#[cfg(not(debug_assertions))]
mod borrow_info_noop {
    use std::fmt;

    pub(super) struct DebugBorrowTracker;

    impl DebugBorrowTracker {
        pub fn new() -> Self {
            Self
        }

        pub fn track_ref_borrow(&self) {}

        pub fn track_mut_borrow(&self) {}
    }

    impl fmt::Display for DebugBorrowTracker {
        fn fmt(&self, _: &mut fmt::Formatter<'_>) -> fmt::Result {
            Ok(())
        }
    }
}
