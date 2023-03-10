# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at https://mozilla.org/MPL/2.0/.

extends TestSuite

func test_missing_init():
	return # TODO: fix dynamic eval

	var expr = Expression.new()
	var error = expr.parse("WithoutInit.new()")
	if !assert_eq(error, OK, "Failed to parse dynamic expression"):
		return

	var instance = expr.execute()
	if !assert_that(!expr.has_execute_failed(), "Failed to evaluate dynamic expression"):
		return

	print("[GD] WithoutInit is: ", instance)

func test_to_string():
	var ffi = VirtualMethodTest.new()
	
	assert_eq(str(ffi), "VirtualMethodTest[integer=0]")

func test_export():
	var obj = HasProperty.new()

	obj.int_val = 5
	assert_eq(obj.int_val, 5)

	obj.string_val = "test val"
	assert_eq(obj.string_val, "test val")

	var node = Node.new()
	obj.object_val = node
	assert_eq(obj.object_val, node)
	
	var texture_val_meta = obj.get_property_list().filter(
		func(el): return el["name"] == "texture_val"
	).front()
	
	assert_that(texture_val_meta != null, "'texture_val' is defined")
	assert_eq(texture_val_meta["hint"], PropertyHint.PROPERTY_HINT_RESOURCE_TYPE)
	assert_eq(texture_val_meta["hint_string"], "Texture")
	
	obj.free()
	node.free()

func test_untyped_array_pass_to_user_func():
	var obj = ArrayTest.new()
	var array: Array = [42, "answer"]
	assert_eq(obj.pass_untyped_array(array), 2)

func test_untyped_array_return_from_user_func():
	var obj = ArrayTest.new()
	var array: Array = obj.return_untyped_array()
	assert_eq(array, [42, "answer"])

func test_typed_array_pass_to_user_func():
	var obj = ArrayTest.new()
	var array: Array[int] = [1, 2, 3]
	assert_eq(obj.pass_typed_array(array), 6)

func test_typed_array_return_from_user_func():
	var obj = ArrayTest.new()
	var array: Array[int] = obj.return_typed_array(3)
	assert_eq(array, [1, 2, 3])
