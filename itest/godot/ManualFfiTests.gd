# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at https://mozilla.org/MPL/2.0/.

extends Node

func run() -> bool:
	print("[GD] Test ManualFfi...")
	var ok = true
	#ok = ok && test_missing_init()
	ok = ok && test_to_string()

	print("[GD] ManualFfi tested (passed=", ok, ")")
	return ok

func test_missing_init() -> bool:
	var obj = WithoutInit.new()
	print("[GD] WithoutInit is: ", obj)
	return true

func test_to_string() -> bool:
	var ffi = VirtualMethodTest.new()
	var s = str(ffi)

	print("to_string: ", s)
	print("to_string: ", ffi)
	return true