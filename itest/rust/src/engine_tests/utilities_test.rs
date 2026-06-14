/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

// Note: in the past, we had is_instance_valid() -- we also tested that godot-rust is not susceptible to the godot-cpp issue
// https://github.com/godotengine/godot-cpp/issues/1390.

use godot::builtin::{GString, Variant, vslice};
use godot::classes::Engine;
use godot::global::*;
use godot::obj::Singleton;

use crate::framework::itest;

#[itest]
fn utilities_abs() {
    let input = Variant::from(-7);
    let output = abs(&input);

    assert_eq!(output, Variant::from(7));
}

#[itest]
fn utilities_sign() {
    let input = Variant::from(-7);
    let output = sign(&input);

    assert_eq!(output, Variant::from(-1));
}

#[itest]
fn utilities_str() {
    let a = 12;
    let b = " is a ";
    let c = true;
    let d = " number";
    let concat = str(vslice![a, b, c, d]);

    let empty = str(&[]);

    assert_eq!(concat, "12 is a true number");
    assert_eq!(concat, godot_str!("{a}{b}{c}{d}"));
    assert_eq!(empty, GString::new());
}

#[itest]
fn utilities_wrap() {
    let output = wrap(
        &Variant::from(3.4),
        &Variant::from(2.0),
        &Variant::from(3.0),
    );
    assert_eq!(output, Variant::from(2.4));

    let output = wrap(
        &Variant::from(-5.7),
        &Variant::from(-3.0),
        &Variant::from(-2.0),
    );
    assert_eq!(output, Variant::from(-2.7));
}

#[itest(skip)] // Switch to focus to test manually.
fn utilities_print_custom() {
    // Each invocation should not panic; output is suppressed to avoid noisy test logs.
    let source = PrintSource {
        function: "module::function",
        file: "src/file.rs",
        line: 42,
    };
    for level in [
        PrintLevel::Info,
        PrintLevel::Warn,
        PrintLevel::Error,
        PrintLevel::ScriptError,
    ] {
        print_custom(PrintRecord {
            level,
            message: "print-regular",
            rationale: Some("detail-message"),
            source: Some(source),
            editor_notify: false,
        });
        print_custom(PrintRecord {
            level,
            message: "print-no-message",
            rationale: None,
            source: Some(source),
            editor_notify: false,
        });
        print_custom(PrintRecord {
            level,
            message: "print-no-message-no-source",
            rationale: None,
            source: None, // Fall back to caller location.
            editor_notify: false,
        });
    }

    // Convenience: build PrintSource from std::panic::Location.
    #[track_caller]
    fn build() -> PrintSource<'static> {
        // Function name passed separately; file/line auto-filled.
        PrintSource::from_location(std::panic::Location::caller(), "build")
    }

    let expected_line = line!() + 2;
    let expected_file = file!();
    let src = build();
    assert_eq!(src.line, expected_line);
    assert_eq!(src.file, expected_file);
}

#[itest]
fn utilities_max() {
    let output = max(&Variant::from(1.0), &Variant::from(3.0), vslice![5.0, 7.0]);
    assert_eq!(output, Variant::from(7.0));

    let output = max(
        &Variant::from(-1.0),
        &Variant::from(-3.0),
        vslice![-5.0, -7.0],
    );
    assert_eq!(output, Variant::from(-1.0));
}

#[itest]
fn utilities_suppress_print() {
    let suppressed = || !Engine::singleton().is_printing_error_messages();

    // Assume at start of test, no suppression is active. Also detects not-cleaned-up global state from earlier tests.
    assert!(!suppressed());

    // Printing is disabled while a guard is alive, and restored to the original value once the outermost guard drops.
    let outer = suppress_godot_errors();
    assert!(suppressed());

    {
        let _inner = suppress_godot_errors();
        assert!(suppressed());
    }
    // Inner drop must not re-enable while the outer guard is still active.
    assert!(suppressed());

    drop(outer);
    assert!(!suppressed());

    // When printing is already disabled, the guard must restore to disabled -- not enable it.
    Engine::singleton().set_print_error_messages(false);
    {
        let _guard = suppress_godot_errors();
        assert!(suppressed());
    }
    assert!(suppressed());
}
