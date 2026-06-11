/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use godot::builtin::{Callable, GString, GodotStringExt, StringName, Variant};
use godot::global::{
    PrintLevel, PrintRecord, godot_error, godot_print, godot_print_rich, godot_warn, print_custom,
};

use crate::framework::{itest, quick_thread};

// The print group is thread-safe (see per-backend C++ rationale in `godot-core/src/global/print.rs`).
// Tests only whether these functions don't panic (i.e. no is-main-thread validation).
#[itest]
fn thread_safe_apis_print() {
    quick_thread(|| {
        godot_print!("Info.");
        godot_error!("Problem.");
        godot_warn!("Caution.");
        godot_print_rich!("Rich info.");

        print_custom(PrintRecord {
            level: PrintLevel::Info,
            message: "Custom info.",
            rationale: None,
            source: None,
            editor_notify: false,
        });
    })
}

#[itest]
fn thread_safe_apis_gstring() {
    // Test various operations: constructor, default, clone, drop, !=, <
    let a = "Abc".to_gstring();
    quick_thread(move || {
        let b = "Abd".to_gstring();
        let a = a.clone();
        assert_eq!(a, "Abc");
        assert_ne!(a, b);
        assert_ne!(a, GString::default());
        assert!(a < b);
    })
}

#[itest]
fn thread_safe_apis_string_name() {
    let a = "Abc".to_string_name();
    quick_thread(move || {
        let b = "Abd".to_string_name();
        let a = a.clone();
        assert_eq!(a, "Abc");
        assert_ne!(a, b);
        assert_ne!(a, StringName::default());
    })
}

// TODO(v0.6): Variant lifecycle stays on the main thread because a Variant can hold a reference type (Object), so dropping it from a worker thread
// is not yet sound. Re-enable once the value-type cases can opt into thread-safe access.
#[itest(skip)]
fn thread_safe_apis_variant() {
    quick_thread(|| {
        // Make sure it's not a Copy/POD Variant that can be initialized Rust-side only.
        let _v = Variant::from("hello");
    })
}

#[itest(skip)]
fn thread_safe_apis_callable() {
    quick_thread(|| {
        let _c = Callable::from_fn("on other thread", |_args| {});
    })
}

// More to add:
// * Array, Dict, Packed*Array
// * Callable (from_sync_fn currently needs exp-threads, should from_fn be allowed as long as not Send?)
