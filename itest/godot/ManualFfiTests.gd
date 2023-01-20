# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at https://mozilla.org/MPL/2.0/.

extends Node

func run() -> bool:
	print("[GD] Test ManualFfi...")
	var ok = true
	ok = ok && test_missing_init()
	ok = ok && test_to_string()
	ok = ok && test_export()

	print("[GD] ManualFfi tested (passed=", ok, ")")
	return ok

func test_missing_init() -> bool:
	return true # TODO: fix dynamic eval

	var expr = Expression.new()
	var error = expr.parse("WithoutInit.new()")
	if error != OK:
		print("Failed to parse dynamic expression")
		return false

	var instance = expr.execute()
	if expr.has_execute_failed():
		print("Failed to evaluate dynamic expression")
		return false

	print("[GD] WithoutInit is: ", instance)
	return true

func test_to_string() -> bool:
	var ffi = VirtualMethodTest.new()
	var s = str(ffi)

	print("to_string: ", s)
	print("to_string: ", ffi)
	return true
	
func test_export() -> bool:
	var obj = HasProperty.new()

	obj.int_val = 5
	print("[GD] HasProperty's int_val property is: ", obj.int_val, " and should be 5")
	var int_val_correct = obj.int_val == 5

	obj.string_val = "test val"
	print("[GD] HasProperty's string_val property is: ", obj.string_val, " and should be \"test val\"")
	var string_val_correct = obj.string_val == "test val"

	var node = Node.new()
	obj.object_val = node
	print("[GD] HasProperty's object_val property is: ", obj.object_val, " and should be ", node)
	var object_val_correct = obj.object_val == node

	obj.free()
	node.free()

	return int_val_correct && string_val_correct && object_val_correct
