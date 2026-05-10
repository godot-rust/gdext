# Copyright (c) godot-rust; Bromeon and contributors.
# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at https://mozilla.org/MPL/2.0/.

class_name TestSuite
extends RefCounted

# Test failure state is split into two orthogonal flags:
# - _assertion_failed: a hard assertion (assert_*, assert_fail) failed; never cleared until reset_state().
# - _pending: mark_test_pending() was called and mark_test_succeeded() has not yet rolled it back.
#
# Test is considered failed iff either flag is set (see is_test_failed()).
# Keeping them orthogonal ensures a real assertion failure inside a pending block survives a later
# mark_test_succeeded() call -- only the provisional pending flag gets cleared, not the assertion failure.
var _assertion_failed: bool = false
var _pending: bool = false

# -----------------------------------------------------------------------------------------------------------------------------------------------
# Public API, called by the test runner.

func reset_state() -> void:
	_assertion_failed = false
	_pending = false

	# Note: some tests disable error messages, but they are re-enabled by the Rust test runner before each test.

func is_test_failed() -> bool:
	return _assertion_failed || _pending

func run_test(suite: RefCounted, method_name: String) -> GDScriptTestRunner.GDScriptTestCase:
	return GDScriptTestRunner.GDScriptExecutableTestCase.new(suite, method_name)

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

func assert_eq(actual: Variant, expected: Variant, message: String = "") -> bool:
	# We want strict equality, while still allowing Object(null) == Nil(null).
	if actual == null and expected == null or typeof(actual) == typeof(expected) and actual == expected:
		return true

	_assertion_failed = true

	var actual_ty = typeof(actual)
	var expected_ty = typeof(expected)

	print_newline() # previous line not yet broken
	if message:
		print_error("GDScript assertion failed:  %s\n    actual: %s (%s)\n  expected: %s (%s)" % [message, actual, type_string(actual_ty), expected, type_string(expected_ty)])
	else:
		print_error("GDScript assertion failed:  `(actual == expected)`\n    actual: %s (%s)\n  expected: %s (%s)" % [actual, type_string(actual_ty), expected, type_string(expected_ty)])

	# Note: stacktrace cannot be printed, because not connected to a debugging server (editor).
	return false

## Pre-emptively mark this test as "failed unless confirmed success". Use mark_test_succeeded() to rollback if test actually succeeds.
##
## For situations where statements coming after this will abort the test function without reliable ways to detect.
func mark_test_pending() -> void:
	if _pending:
		print_error("Test is already pending.")
		_assertion_failed = true
		return

	_pending = true

## Roll back the pending failure assumption if the test actually succeeded.
## Does not clear real assertion failures recorded between mark_test_pending() and this call.
func mark_test_succeeded() -> void:
	if not _pending:
		print_error("Cannot mark test as succeeded, test was not marked as pending.")
		_assertion_failed = true
		return

	_pending = false

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

	# Re-enable error messages in case expect_fail() suppressed them.
	Engine.print_error_messages = true

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
