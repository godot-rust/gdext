# Copyright (c) godot-rust; Bromeon and contributors.
# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at https://mozilla.org/MPL/2.0/.

extends TestSuiteSpecial

# Test cases for async functions functionality

func test_async_vector2_multiply():
	print("=== Testing async Vector2 multiplication ===")
	var async_obj = AsyncTestClass.new()
	
	var future = async_obj.async_vector2_multiply(Vector2(3.0, 4.0))
	print("Got future: ", future)
	print("Future type: ", typeof(future))
	print("Future class: ", future.get_class())
	
	# Test if the object has the finished signal
	if future.has_signal("finished"):
		print("✓ Future has 'finished' signal")
		
		# Connect to the signal and wait for result
		var signal_obj = Signal(future, "finished")
		var result = await signal_obj
		print("Received result: ", result)
		print("Result type: ", typeof(result))
		
		# Validate result - await returns the signal parameter directly
		print("Actual result: ", result)
		assert_that(result is Vector2, "Result should be Vector2")
		var expected = Vector2(6.0, 8.0)
		assert_that(result.is_equal_approx(expected), "Vector2 should be multiplied correctly: expected " + str(expected) + ", got " + str(result))
		print("✓ Vector2 multiplication test passed")
	else:
		assert_that(false, "Future does not have 'finished' signal")

func test_async_simple_math():
	print("=== Testing async simple math ===")
	var async_obj = AsyncTestClass.new()
	
	var future = async_obj.async_compute_sum(10, 5)
	print("Got future: ", future)
	
	if future.has_signal("finished"):
		print("✓ Future has 'finished' signal")
		
		var signal_obj = Signal(future, "finished")
		var result = await signal_obj
		print("Received result: ", result)
		
		print("Actual result: ", result)
		assert_that(result is int, "Result should be int")
		assert_eq(result, 15, "10 + 5 should equal 15")
		print("✓ Simple math test passed")
	else:
		assert_that(false, "Future does not have 'finished' signal")

func test_async_magic_number():
	print("=== Testing async magic number ===")
	var async_obj = AsyncTestClass.new()
	
	var future = async_obj.async_get_magic_number()
	print("Got future: ", future)
	
	if future.has_signal("finished"):
		print("✓ Future has 'finished' signal")
		
		var signal_obj = Signal(future, "finished")
		var result = await signal_obj
		print("Received result: ", result)
		
		print("Actual result: ", result)
		assert_that(result is int, "Result should be int")
		assert_eq(result, 42, "Magic number should be 42")
		print("✓ Magic number test passed")
	else:
		assert_that(false, "Future does not have 'finished' signal")

func test_async_http_request():
	print("=== Testing async HTTP request ===")
	var network_obj = AsyncNetworkTestClass.new()
	
	var future = network_obj.async_http_request()
	print("Got HTTP future: ", future)
	
	if future.has_signal("finished"):
		print("✓ HTTP Future has 'finished' signal")
		
		var signal_obj = Signal(future, "finished")
		var result = await signal_obj
		print("Received HTTP result: ", result)
		
		print("Actual HTTP result: ", result)
		assert_that(result is int, "HTTP result should be int")
		# Accept both success (200) and network failure (-1)
		assert_that(result == 200 or result == -1, "HTTP result should be 200 (success) or -1 (network error), got " + str(result))
		if result == 200:
			print("✓ HTTP request successful!")
		else:
			print("! HTTP request failed (network issue - this is acceptable in CI)")
		print("✓ HTTP request test completed")
	else:
		assert_that(false, "HTTP Future does not have 'finished' signal") 