# Copyright (c) godot-rust; Bromeon and contributors.
# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at https://mozilla.org/MPL/2.0/.

extends TestSuiteSpecial

# Test cases for async functions functionality

# Simplified async helper functions
static func await_rust_async(future_obj) -> Variant:
	"""
	Simplified way to await Rust async functions.
	Usage: var result = await await_rust_async(some_async_function())
	"""
	if not future_obj.has_signal("finished"):
		push_error("Object does not have 'finished' signal - not a valid async future")
		return null
	
	var signal_obj = Signal(future_obj, "finished")
	return await signal_obj

# Direct async function call with await
static func call_async(object: Object, method_name: String, args: Array = []) -> Variant:
	"""
	Call an async method and await its result in one line.
	Usage: var result = await call_async(obj, "async_method_name", [arg1, arg2])
	"""
	var future_obj = object.callv(method_name, args)
	return await await_rust_async(future_obj)

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

# Test the REVOLUTIONARY direct await pattern!
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

# Test the revolutionary direct Signal return approach
func test_direct_signal_return():
	print("=== Testing Direct Signal Return (Revolutionary!) ===")
	var result = await direct_signal_test()
	var expected = Vector2(30.0, 60.0)  # input * 3
	assert_that(result.is_equal_approx(expected), "Direct signal test should return input * 3")
	print("✓ Direct Signal return works! This is REVOLUTIONARY!")

func async_vector2_multiply(input: Vector2) -> Vector2:
	var async_obj = AsyncTestClass.new()
	return await call_async(async_obj, "async_vector2_multiply", [input])

func async_string_process(input: StringName) -> StringName:
	var async_obj = AsyncTestClass.new()
	return await call_async(async_obj, "async_string_process", [input])

func async_simple_calc(x: int, y: int) -> int:
	var async_obj = AsyncTestClass.new()
	return await call_async(async_obj, "async_simple_calc", [x, y])

# *** EXPERIMENTAL: Direct Signal Await Test ***
# Test if we can directly await a function that returns Signal
func direct_signal_test() -> Vector2:
	var gd_obj = GdSelfObj.new()
	var signal_result = gd_obj.direct_signal_test(Vector2(10.0, 20.0))
	
	# This should work if Signal can be awaited directly!
	var result = await signal_result
	print("Direct signal test result: ", result)
	return result 