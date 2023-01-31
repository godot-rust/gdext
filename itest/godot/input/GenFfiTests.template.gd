# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at https://mozilla.org/MPL/2.0/.

extends TestSuite

#(
func test_varcall_IDENT():
	var ffi = GenFfi.new()

	var from_rust = ffi.return_IDENT()
	assert_that(ffi.accept_IDENT(from_rust))

	var from_gdscript = VAL
	var mirrored = ffi.mirror_IDENT(from_gdscript)
	assert_eq(mirrored, from_gdscript)
#)
