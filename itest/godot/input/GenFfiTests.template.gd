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
	var ffi = GenFfi.new()

	var from_rust: Variant = ffi.return_IDENT()
	_check_callconv("return_IDENT", "varcall")

	assert_that(ffi.accept_IDENT(from_rust), "ffi.accept_IDENT(from_rust)")
	_check_callconv("accept_IDENT", "varcall")

	var from_gdscript: Variant = VAL
	var mirrored: Variant = ffi.mirror_IDENT(from_gdscript)
	assert_eq(mirrored, from_gdscript, "mirrored == from_gdscript")
	_check_callconv("mirror_IDENT", "varcall")
#)

#(
func test_varcall_static_IDENT():
	var from_rust: Variant = GenFfi.return_static_IDENT()
	_check_callconv("return_static_IDENT", "varcall")

	assert_that(GenFfi.accept_static_IDENT(from_rust), "ffi.accept_static_IDENT(from_rust)")
	_check_callconv("accept_static_IDENT", "varcall")

	var from_gdscript: Variant = VAL
	var mirrored: Variant = GenFfi.mirror_static_IDENT(from_gdscript)
	assert_eq(mirrored, from_gdscript, "mirrored_static == from_gdscript")
	_check_callconv("mirror_static_IDENT", "varcall")
#)

#(
func test_ptrcall_IDENT():
	var ffi := GenFfi.new()

	var from_rust: TYPE = ffi.return_IDENT()
	_check_callconv("return_IDENT", "ptrcall")

	assert_that(ffi.accept_IDENT(from_rust), "ffi.accept_IDENT(from_rust)")
	_check_callconv("accept_IDENT", "ptrcall")

	var from_gdscript: TYPE = VAL
	var mirrored: TYPE = ffi.mirror_IDENT(from_gdscript)
	assert_eq(mirrored, from_gdscript, "mirrored == from_gdscript")
	_check_callconv("mirror_IDENT", "ptrcall")
#)

#(
func test_ptrcall_static_IDENT():
	var from_rust: TYPE = GenFfi.return_static_IDENT()
	_check_callconv("return_static_IDENT", "ptrcall")

	assert_that(GenFfi.accept_static_IDENT(from_rust), "ffi.accept_static_IDENT(from_rust)")
	_check_callconv("accept_static_IDENT", "ptrcall")

	var from_gdscript: TYPE = VAL
	var mirrored: TYPE = GenFfi.mirror_static_IDENT(from_gdscript)
	assert_eq(mirrored, from_gdscript, "mirrored_static == from_gdscript")
	_check_callconv("mirror_static_IDENT", "ptrcall")
#)

func _check_callconv(function: String, expected: String) -> void:
	# TODO Ptrcall not yet implemented in Godot:
	# * Methods that involve at least 1 parameter of type Variant (interestingly not a return value).
	# * Static methods (oversight).
	if function.contains("_static_") or function == "accept_variant" or function == "mirror_variant":
		# This test deliberately fails in case Godot implements support for either of the above, to notify us.
		expected = "varcall"

	var ok = GenFfi.check_last_notrace(function, expected)
	assert_that(ok, str("calling convention mismatch in function '", function, "'"))
