/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use godot::classes::Node;
use godot::prelude::*;
use godot::tools::{get_autoload_by_name, try_get_autoload_by_name};

use crate::framework::{itest, quick_thread};

#[derive(GodotClass)]
#[class(init, base=Node)]
struct AutoloadClass {
    base: Base<Node>,
    #[var]
    property: i32,
}

#[godot_api]
impl AutoloadClass {
    #[func]
    fn verify_works(&self) -> i32 {
        787
    }
}

#[itest]
fn autoload_get() {
    let mut autoload = get_autoload_by_name::<AutoloadClass>("MyAutoload");
    {
        let mut guard = autoload.bind_mut();
        assert_eq!(guard.verify_works(), 787);
        assert_eq!(guard.property, 0, "still has default value");

        guard.property = 42;
    }

    // Fetch same autoload anew.
    let autoload2 = get_autoload_by_name::<AutoloadClass>("MyAutoload");
    assert_eq!(autoload2.bind().property, 42);

    // Reset for other tests.
    autoload.bind_mut().property = 0;
}

#[itest]
fn autoload_try_get_named() {
    let autoload = try_get_autoload_by_name::<AutoloadClass>("MyAutoload").expect("fetch autoload");

    assert_eq!(autoload.bind().verify_works(), 787);
    assert_eq!(autoload.bind().property, 0, "still has default value");
}

#[itest]
fn autoload_try_get_named_inexistent() {
    let result = try_get_autoload_by_name::<AutoloadClass>("InexistentAutoload");
    result.expect_err("non-existent autoload");
}

#[itest]
fn autoload_try_get_named_bad_type() {
    let result = try_get_autoload_by_name::<Node2D>("MyAutoload");
    result.expect_err("autoload of incompatible node type");
}

#[itest]
fn autoload_from_other_thread() {
    use std::sync::{Arc, Mutex};

    // We can't return the Result from the thread because Gd<T> is not Send, so we extract the error message instead.
    let outer_error = Arc::new(Mutex::new(String::new()));
    let inner_error = Arc::clone(&outer_error);

    quick_thread(move || {
        let result = try_get_autoload_by_name::<AutoloadClass>("MyAutoload");
        match result {
            Ok(_) => panic!("autoload access from non-main thread should fail"),
            Err(err) => {
                *inner_error.lock().unwrap() = err.to_string();
            }
        }
    });

    let msg = outer_error.lock().unwrap();
    assert_eq!(
        *msg,
        "Autoloads must be fetched from main thread, as Gd<T> is not thread-safe"
    );
}
