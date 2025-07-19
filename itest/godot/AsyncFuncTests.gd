# Copyright (c) godot-rust; Bromeon and contributors.
# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at https://mozilla.org/MPL/2.0/.

extends TestSuiteSpecial

# Test cases for async functions functionality

# === STATIC ASYNC METHOD TESTS ===

func test_async_static_methods():
	var async_obj = AsyncTestClass.new()
	
	# Test Vector2 operation
	var vector_result = await async_obj.async_vector2_multiply(Vector2(3.0, 4.0))
	assert_that(vector_result is Vector2, "Result should be Vector2")
	assert_that(vector_result.is_equal_approx(Vector2(6.0, 8.0)), "Vector2 should be multiplied correctly")
	
	# Test integer math
	var math_result = await async_obj.async_compute_sum(10, 5)
	assert_that(math_result is int, "Result should be int")
	assert_eq(math_result, 15, "10 + 5 should equal 15")
	
	# Test magic number
	var magic_result = await async_obj.async_get_magic_number()
	assert_that(magic_result is int, "Magic result should be int")
	assert_eq(magic_result, 42, "Magic number should be 42")
	
	# Test string result
	var message_result = await async_obj.async_get_message()
	assert_that(message_result is StringName, "Message result should be StringName")
	assert_eq(str(message_result), "async message", "Message should be correct")

func test_async_instance_methods():
	var simple_obj = SimpleAsyncClass.new()
	
	# Test basic async instance method
	simple_obj.set_value(100)
	var sync_value = simple_obj.get_value()
	assert_eq(sync_value, 100, "Sync value should be 100")
	
	var async_result = await simple_obj.async_get_value()
	assert_that(async_result is int, "Async result should be int")
	assert_eq(async_result, 100, "Async instance method should return same value as sync method")
	
	# Test multiple calls with different values
	for test_value in [42, -55, 999]:
		simple_obj.set_value(test_value)
		var sync_result = simple_obj.get_value()
		var async_value = await simple_obj.async_get_value()
		assert_eq(sync_result, async_value, "Sync and async methods should return same value for " + str(test_value))

func test_multiple_async_instances():
	# Test that multiple objects maintain separate state
	var obj1 = SimpleAsyncClass.new()
	var obj2 = SimpleAsyncClass.new()
	
	obj1.set_value(111)
	obj2.set_value(222)
	
	var result1 = await obj1.async_get_value()
	var result2 = await obj2.async_get_value()
	
	assert_eq(result1, 111, "Object 1 should maintain its value")
	assert_eq(result2, 222, "Object 2 should maintain its value")