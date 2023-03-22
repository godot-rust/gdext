# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at https://mozilla.org/MPL/2.0/.

extends ArrayTest

var test_suite: TestSuite = TestSuite.new()

# In order to reproduce the behavior discovered in https://github.com/godot-rust/gdext/issues/138
# we must inherit a Godot Node. Because of this we can't just inherit TesSuite like the rest of the tests.
func assert_that(what: bool, message: String = "") -> bool:
	return test_suite.assert_that(what, message)

func assert_eq(left, right, message: String = "") -> bool:
	return test_suite.assert_eq(left, right, message)

# Called when the node enters the scene tree for the first time.
func _ready():
	pass

func test_vector_array_return_from_user_func():
	var array: Array = return_typed_array(2)
	assert_eq(array, [1,2])

# Called every frame. 'delta' is the elapsed time since the previous frame.
func _process(delta):
	pass
