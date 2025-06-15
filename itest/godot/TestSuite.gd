# Copyright (c) godot-rust; Bromeon and contributors.
# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at https://mozilla.org/MPL/2.0/.

class_name TestSuite
extends RefCounted

var _assertion_failed: bool = false
var _pending: bool = false

# -----------------------------------------------------------------------------------------------------------------------------------------------
# Public API, called by the test runner.

func reset_state() -> void:
	_assertion_failed = false
	_pending = false

	# Note: some tests disable error messages, but they are re-enabled by the Rust test runner before each test.

func is_test_failed() -> bool:
	return _assertion_failed

# -----------------------------------------------------------------------------------------------------------------------------------------------
# Protected API, called by individual test .gd files.

func print_newline() -> void:
	printerr()

func print_error(s: String) -> void:
	push_error(s)

## Asserts that `what` is `true`, but does not abort the test. Returns `what` so you can return
## early from the test function if the assertion failed.
func assert_that(what: bool, message: String = "") -> bool:
	if what:
		return true

	_assertion_failed = true

	print_newline() # previous line not yet broken
	if message:
		print_error("GDScript assertion failed:  %s" % message)
	else:
		print_error("GDScript assertion failed.")

	return false

func assert_eq(actual, expected, message: String = "") -> bool:
	if actual == expected:
		return true

	_assertion_failed = true

	print_newline() # previous line not yet broken
	if message:
		print_error("GDScript assertion failed:  %s\n    actual: %s\n  expected: %s" % [message, actual, expected])
	else:
		print_error("GDScript assertion failed:  `(actual == expected)`\n    actual: %s\n  expected: %s" % [actual, expected])

	# Note: stacktrace cannot be printed, because not connected to a debugging server (editor).
	return false

## Pre-emptively mark this test as "failed unless confirmed success". Use mark_test_succeeded() to rollback if test actually succeeds.
##
## For situations where statements coming after this will abort the test function without reliable ways to detect.
func mark_test_pending() -> void:
	if _pending:
		push_error("Test is already pending.")
		_assertion_failed = true
		return

	_pending = true
	_assertion_failed = true
	#print("Test will fail unless rolled back: ", message)

## Roll back the failure assumption if the test actually succeeded.
func mark_test_succeeded() -> void:
	if not _pending:
		push_error("Cannot mark test as succeeded, test was not marked as pending.")
		_assertion_failed = true
		return

	_pending = false
	_assertion_failed = false

## Expects that one of the next statements will cause the calling GDScript function to abort.
##
## This should always be followed by an assert_fail() call at the end of the function.
func expect_fail() -> void:
	if runs_release():
		push_warning("Release builds do not have proper error handling, the calling test is likely to not work as expected.")

	# Note: error messages are re-enabled by the Rust test runner before each test.
	Engine.print_error_messages = false

# Asserts that the test failed to reach this point. You should disable error messages before running code 
# that is expected to print an error message that would otherwise cause the CI to report failure.
func assert_fail(message: String = "") -> bool:
	_assertion_failed = true

	print_newline()
	if message:
		print_error("Test execution should have failed: %s" % [message])
	else:
		print_error("Test execution should have failed")

	return false

## Some tests are disabled, as they rely on Godot checks which are only available in Debug builds.
## See https://github.com/godotengine/godot/issues/86264.
static func runs_release() -> bool:
	return !OS.is_debug_build()
