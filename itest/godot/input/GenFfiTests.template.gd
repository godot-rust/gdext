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
	assert_that(ffi.accept_IDENT(from_rust), "ffi.accept_IDENT(from_rust)")

	var from_gdscript: Variant = VAL
	var mirrored: Variant = ffi.mirror_IDENT(from_gdscript)
	assert_eq(mirrored, from_gdscript, "mirrored == from_gdscript")
#)

#(
func test_varcall_static_IDENT():
	var from_rust: Variant = GenFfi.return_static_IDENT()
	assert_that(GenFfi.accept_static_IDENT(from_rust), "ffi.accept_static_IDENT(from_rust)")

	var from_gdscript: Variant = VAL
	var mirrored: Variant = GenFfi.mirror_static_IDENT(from_gdscript)
	assert_eq(mirrored, from_gdscript, "mirrored_static == from_gdscript")
#)

#(
func test_ptrcall_IDENT():
	var ffi := GenFfi.new()

	var from_rust: TYPE = ffi.return_IDENT()
	assert_that(ffi.accept_IDENT(from_rust), "ffi.accept_IDENT(from_rust)")

	var from_gdscript: TYPE = VAL
	var mirrored: TYPE = ffi.mirror_IDENT(from_gdscript)
	assert_eq(mirrored, from_gdscript, "mirrored == from_gdscript")
#)

#(
func test_ptrcall_static_IDENT():
	var from_rust: TYPE = GenFfi.return_static_IDENT()
	assert_that(GenFfi.accept_static_IDENT(from_rust), "ffi.accept_static_IDENT(from_rust)")

	var from_gdscript: TYPE = VAL
	var mirrored: TYPE = GenFfi.mirror_static_IDENT(from_gdscript)
	assert_eq(mirrored, from_gdscript, "mirrored_static == from_gdscript")
#)
	