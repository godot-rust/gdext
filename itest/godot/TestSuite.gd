# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at https://mozilla.org/MPL/2.0/.

class_name TestSuite
extends RefCounted

var _assertion_failed: bool = false

## Asserts that `what` is `true`, but does not abort the test. Returns `what` so you can return
## early from the test function if the assertion failed.
func assert_that(what: bool, message: String = "") -> bool:
	if what:
		return true

	_assertion_failed = true
	if message:
		print("assertion failed: %s" % message)
	else:
		print("assertion failed")
	return false

func assert_eq(left, right, message: String = "") -> bool:
	if left == right:
		return true

	_assertion_failed = true
	if message:
		print("assertion failed: %s\n  left: %s\n right: %s" % [message, left, right])
	else:
		print("assertion failed: `(left == right)`\n  left: %s\n right: %s" % [left, right])
	return false
