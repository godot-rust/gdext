# Copyright (c) godot-rust; Bromeon and contributors.
# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at https://mozilla.org/MPL/2.0/.

extends TestSuiteSpecial

# Tests in here require specific setup/configuration that is not easily achievable through the standard 
# integration testing API.
# 
# Using the standard API if possible is highly preferred.

# Test that we can call `_input_event` on a class defined in rust, as a virtual method. 
#
# This tests #267, which was caused by us incorrectly handing Objects when passed as arguments to virtual 
# methods. `_input_event` is the easiest such method to test. However it can only be triggered by letting a 
# full physics frame pass after calling `push_unhandled_input`. Thus we cannot use the standard API for 
# testing this at the moment, since we dont have any way to let frames pass in between the start and end of 
# an integration test. 
func test_collision_object_2d_input_event():
	var collision_object := CollisionObject2DTest.new()
	collision_object.input_pickable = true

	var collision_shape := CollisionShape2D.new()
	collision_shape.shape = RectangleShape2D.new()
	collision_object.add_child(collision_shape)

	var window := Window.new()
	window.physics_object_picking = true
	window.add_child(collision_object)

	var root: Node = Engine.get_main_loop().root
	root.add_child(window)

	assert_that(not collision_object.input_event_called(), "Input event should not be propagated")
	assert_eq(collision_object.get_viewport(), null, "Collision viewport should be null")

	var event := InputEventMouseMotion.new()
	event.global_position = Vector2.ZERO

	# FIXME: https://github.com/godotengine/godot/pull/88992 changed behavior here. Adjust code depending on resolution.
	return

	# Godot 4.0 compat: behavior of `push_unhandled_input` was not consistent with `push_input`.
	if Engine.get_version_info().minor == 0:
		window.push_unhandled_input(event)
	else:
		window.push_input(event)

	# Ensure we run a full physics frame
	await root.get_tree().physics_frame

	assert_that(collision_object.input_event_called(), "Input event should be propagated")
	assert_eq(collision_object.get_viewport(), window, "Collision viewport should be the (non-null) window")

	window.queue_free()

