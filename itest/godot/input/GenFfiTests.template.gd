# Copyright (c) godot-rust; Bromeon and contributors.
# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at https://mozilla.org/MPL/2.0/.

extends TestSuite

# Note: GDScript only uses ptrcalls if it has the full type information available at "compile" (parse) time.
# That includes all arguments (including receiver) as well as function signature (parameters + return type).
# Otherwise, GDScript will use varcall. Both are tested below.
# It is thus important that `ffi` is initialized using = for varcalls, and using := for ptrcalls.

#(
func test_varcall_IDENT():
	mark_test_pending()
	var ffi = GenFfi.new()

	var from_rust: Variant = ffi.return_IDENT()
	_check_callconv("return_IDENT", "varcall")

	assert_that(ffi.accept_IDENT(from_rust), "ffi.accept_IDENT(from_rust)")
	_check_callconv("accept_IDENT", "varcall")

	var from_gdscript: Variant = VAL
	var mirrored: Variant = ffi.mirror_IDENT(from_gdscript)
	assert_eq(mirrored, from_gdscript, "mirrored == from_gdscript")
	_check_callconv("mirror_IDENT", "varcall")
	mark_test_succeeded()
#)

# Godot currently does not support calling static methods via reflection, which is why we use an instance for the return_static_IDENT() call.
# The call must be dynamic, as otherwise, Godot would have the static type info available and could use ptrcall.
# This is only needed for return_static_IDENT() which takes no parameters -- the other two methods take Variant parameters,
# so Godot cannot use ptrcalls anyway.
#(
func test_varcall_static_IDENT():
	mark_test_pending()
	var instance = GenFfi.new() # workaround
	var from_rust: Variant = instance.return_static_IDENT()
	_check_callconv("return_static_IDENT", "varcall")

	assert_that(GenFfi.accept_static_IDENT(from_rust), "ffi.accept_static_IDENT(from_rust)")
	_check_callconv("accept_static_IDENT", "varcall")

	var from_gdscript: Variant = VAL
	var mirrored: Variant = GenFfi.mirror_static_IDENT(from_gdscript)
	assert_eq(mirrored, from_gdscript, "mirrored_static == from_gdscript")
	_check_callconv("mirror_static_IDENT", "varcall")
	mark_test_succeeded()
#)

#(
func test_ptrcall_IDENT():
	mark_test_pending()
	var ffi := GenFfi.new()

	var from_rust: TYPE = ffi.return_IDENT()
	_check_callconv("return_IDENT", "ptrcall")

	assert_that(ffi.accept_IDENT(from_rust), "ffi.accept_IDENT(from_rust)")
	_check_callconv("accept_IDENT", "ptrcall")

	var from_gdscript: TYPE = VAL
	var mirrored: TYPE = ffi.mirror_IDENT(from_gdscript)
	assert_eq(mirrored, from_gdscript, "mirrored == from_gdscript")
	_check_callconv("mirror_IDENT", "ptrcall")
	mark_test_succeeded()
#)

# Functions that are invoked via ptrcall do not have an API to propagate the error back to the caller, but Godot pre-initializes their
# return value to the default value of that type. This test verifies that in case of panic, the default value (per Godot) is returned.
#(
func test_ptrcall_panic_IDENT():
	mark_test_pending()
	var ffi := GenFfi.new()

	var from_rust: TYPE = ffi.panic_IDENT()
	_check_callconv("panic_IDENT", "ptrcall")

	var expected_default: TYPE # initialize to default (null for objects)
	assert_eq(from_rust, expected_default, "return value from panicked ptrcall fn == default value")
	mark_test_succeeded()
#)

#(
func test_ptrcall_static_IDENT():
	mark_test_pending()
	var from_rust: TYPE = GenFfi.return_static_IDENT()
	_check_callconv("return_static_IDENT", "ptrcall")

	assert_that(GenFfi.accept_static_IDENT(from_rust), "ffi.accept_static_IDENT(from_rust)")
	_check_callconv("accept_static_IDENT", "ptrcall")

	var from_gdscript: TYPE = VAL
	var mirrored: TYPE = GenFfi.mirror_static_IDENT(from_gdscript)
	assert_eq(mirrored, from_gdscript, "mirrored_static == from_gdscript")
	_check_callconv("mirror_static_IDENT", "ptrcall")
	mark_test_succeeded()
#)

func _check_callconv(function: String, expected: String) -> void:
	# Godot does not yet implement ptrcall for functions that involve at least 1 parameter of type Variant
	# (interestingly not a return value).
	if function in ["accept_variant", "mirror_variant", "accept_static_variant", "mirror_static_variant"]:
		# This test deliberately fails in case Godot implements support for either of the above, to notify us.
		expected = "varcall"

	var ok = GenFfi.check_last_notrace(function, expected)

	# A lot of this has only been fixed for Godot 4.3; tracking regressions for older versions doesn't make much sense.
	# Do not move version check to the beginning -- traced function needs to be popped.
	if Engine.get_version_info().minor >= 3:
		assert_that(ok, str("calling convention mismatch in function '", function, "'"))
