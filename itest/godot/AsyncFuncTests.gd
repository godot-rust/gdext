# Copyright (c) godot-rust; Bromeon and contributors.
# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at https://mozilla.org/MPL/2.0/.

extends TestSuiteSpecial

# Test cases for async functions functionality

# === STATIC ASYNC METHOD TESTS ===

func test_async_vector2_multiply():
	print("=== Testing async Vector2 multiplication (REVOLUTIONARY!) ===")
	var async_obj = AsyncTestClass.new()
	
	# 🚀 REVOLUTIONARY: Direct await - no helpers needed!
	var result = await async_obj.async_vector2_multiply(Vector2(3.0, 4.0))
	
	print("Received result: ", result)
	print("Result type: ", typeof(result))
	print("Actual result: ", result)
	
	# Validate result
	assert_that(result is Vector2, "Result should be Vector2")
	var expected = Vector2(6.0, 8.0)
	assert_that(result.is_equal_approx(expected), "Vector2 should be multiplied correctly: expected " + str(expected) + ", got " + str(result))
	print("✓ Vector2 multiplication test passed with DIRECT AWAIT!")

func test_async_simple_math():
	print("=== Testing async simple math (REVOLUTIONARY!) ===")
	var async_obj = AsyncTestClass.new()
	
	# 🚀 REVOLUTIONARY: Direct await - no helpers needed!
	var result = await async_obj.async_compute_sum(10, 5)
	
	print("Received result: ", result)
	print("Actual result: ", result)
	
	# Validate result
	assert_that(result is int, "Result should be int")
	assert_eq(result, 15, "10 + 5 should equal 15")
	print("✓ Simple math test passed with DIRECT AWAIT!")

func test_async_magic_number():
	print("=== Testing async magic number (REVOLUTIONARY!) ===")
	var async_obj = AsyncTestClass.new()
	
	# 🚀 REVOLUTIONARY: Direct await - no helpers needed!
	var result = await async_obj.async_get_magic_number()
	
	print("Received result: ", result)
	print("Actual result: ", result)
	
	# Validate result
	assert_that(result is int, "Result should be int")
	assert_eq(result, 42, "Magic number should be 42")
	print("✓ Magic number test passed with DIRECT AWAIT!")

func test_async_http_request():
	print("=== Testing async HTTP request (REVOLUTIONARY!) ===")
	var network_obj = AsyncNetworkTestClass.new()
	
	# 🚀 REVOLUTIONARY: Direct await - no helpers needed!
	var result = await network_obj.async_http_request()
	
	print("Received HTTP result: ", result)
	print("Actual HTTP result: ", result)
	
	# Validate result
	assert_that(result is int, "HTTP result should be int")
	# Accept both success (200) and network failure (-1)
	assert_that(result == 200 or result == -1, "HTTP result should be 200 (success) or -1 (network error), got " + str(result))
	if result == 200:
		print("✓ HTTP request successful!")
	else:
		print("! HTTP request failed (network issue - this is acceptable in CI)")
	print("✓ HTTP request test completed with DIRECT AWAIT!")

# === REVOLUTIONARY ASYNC INSTANCE METHOD TESTS ===

func test_async_instance_method_simple():
	print("=== Testing REVOLUTIONARY Async Instance Method! ===")
	var simple_obj = SimpleAsyncClass.new()
	
	# Set a value first
	simple_obj.set_value(100)
	var sync_value = simple_obj.get_value()
	print("Sync value: ", sync_value)
	assert_eq(sync_value, 100, "Sync value should be 100")
	
	# 🚀 REVOLUTIONARY: Direct await on INSTANCE METHOD!
	print("--- Testing revolutionary async instance method ---")
	var async_result = await simple_obj.async_get_value()
	
	print("Async result: ", async_result)
	print("Async result type: ", typeof(async_result))
	
	# Validate result
	assert_that(async_result is int, "Async result should be int")
	assert_eq(async_result, 100, "Async instance method should return same value as sync method")
	print("✓ REVOLUTIONARY async instance method works!")

func test_async_instance_method_multiple_calls():
	print("=== Testing Multiple Async Instance Method Calls ===")
	var simple_obj = SimpleAsyncClass.new()
	
	# Test multiple different values
	simple_obj.set_value(42)
	var result1 = await simple_obj.async_get_value()
	assert_eq(result1, 42, "First async call should return 42")
	
	simple_obj.set_value(999)
	var result2 = await simple_obj.async_get_value()
	assert_eq(result2, 999, "Second async call should return 999")
	
	simple_obj.set_value(-55)
	var result3 = await simple_obj.async_get_value()
	assert_eq(result3, -55, "Third async call should return -55")
	
	print("✓ Multiple async instance method calls work perfectly!")

func test_async_instance_vs_sync_consistency():
	print("=== Testing Async vs Sync Instance Method Consistency ===")
	var simple_obj = SimpleAsyncClass.new()
	
	# Test that async and sync methods return the same value
	for test_value in [0, 1, -1, 42, 12345, -9999]:
		simple_obj.set_value(test_value)
		
		var sync_result = simple_obj.get_value()
		var async_result = await simple_obj.async_get_value()
		
		print("Value ", test_value, ": sync=", sync_result, ", async=", async_result)
		assert_eq(sync_result, async_result, "Sync and async methods should return same value for " + str(test_value))
	
	print("✓ Async and sync instance methods are consistent!")

# === MULTIPLE OBJECT INSTANCE TESTS ===

func test_multiple_async_instances():
	print("=== Testing Multiple Async Instance Objects ===")
	var obj1 = SimpleAsyncClass.new()
	var obj2 = SimpleAsyncClass.new()
	var obj3 = SimpleAsyncClass.new()
	
	# Set different values for each object
	obj1.set_value(111)
	obj2.set_value(222)
	obj3.set_value(333)
	
	# Call async methods on all objects - they should maintain separate state
	var result1 = await obj1.async_get_value()
	var result2 = await obj2.async_get_value()
	var result3 = await obj3.async_get_value()
	
	print("Results: obj1=", result1, ", obj2=", result2, ", obj3=", result3)
	
	assert_eq(result1, 111, "Object 1 should maintain its value")
	assert_eq(result2, 222, "Object 2 should maintain its value")
	assert_eq(result3, 333, "Object 3 should maintain its value")
	
	print("✓ Multiple async instance objects maintain separate state!")

# === ORIGINAL STATIC METHOD TESTS ===

func test_simplified_async_usage():
	print("=== Testing REVOLUTIONARY Direct Await Pattern! ===")
	var async_obj = AsyncTestClass.new()
	
	# 🚀 REVOLUTIONARY: Direct await - just like native GDScript async!
	print("--- Testing revolutionary direct await ---")
	var result = await async_obj.async_vector2_multiply(Vector2(3.0, 4.0))
	
	print("Result: ", result)
	assert_that(result.is_equal_approx(Vector2(6.0, 8.0)), "Vector2 should be multiplied correctly")
	print("✓ REVOLUTIONARY direct await works!")
	
	# 🚀 Another example - math operation
	print("--- Testing another direct await ---")
	var result2 = await async_obj.async_compute_sum(10, 5)
	
	print("Result: ", result2)
	assert_eq(result2, 15, "10 + 5 should equal 15")
	print("✓ Another direct await works perfectly!")
	
	print("✓ REVOLUTIONARY async pattern test completed - NO HELPERS NEEDED!")

func test_multiple_async_simplified():
	print("=== Testing Multiple Async Operations (REVOLUTIONARY!) ===")
	var async_obj = AsyncTestClass.new()
	
	# 🚀 REVOLUTIONARY: Direct await for multiple operations - no helpers!
	print("--- Starting multiple async operations ---")
	var result1 = await async_obj.async_compute_sum(1, 2)
	var result2 = await async_obj.async_compute_sum(3, 4)  
	var result3 = await async_obj.async_get_magic_number()
	
	print("Results: [", result1, ", ", result2, ", ", result3, "]")
	assert_eq(result1, 3, "1 + 2 should equal 3")
	assert_eq(result2, 7, "3 + 4 should equal 7") 
	assert_eq(result3, 42, "Magic number should be 42")
	print("✓ Multiple REVOLUTIONARY async operations work perfectly!")

# === RUNTIME TESTS ===

func test_async_runtime_chain():
	print("=== Testing Async Runtime Chain ===")
	var runtime_obj = AsyncRuntimeTestClass.new()
	
	var result = await runtime_obj.test_simple_async_chain()
	print("Chain result: ", result)
	assert_that(result is StringName, "Result should be StringName")
	assert_eq(str(result), "Simple async chain test passed", "Chain test should return expected message")
	print("✓ Async runtime chain test passed!")

func test_async_runtime_math():
	print("=== Testing Async Runtime Math ===")
	var runtime_obj = AsyncRuntimeTestClass.new()
	
	var result = await runtime_obj.test_simple_async()
	print("Math result: ", result)
	assert_that(result is int, "Result should be int")
	assert_eq(result, 100, "42 + 58 should equal 100")
	print("✓ Async runtime math test passed!") 