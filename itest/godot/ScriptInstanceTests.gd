# Copyright (c) godot-rust; Bromeon and contributors.
# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at https://mozilla.org/MPL/2.0/.

extends TestSuite

func create_script_instance() -> RefCounted:
	var script = TestScript.new()
	var script_owner = RefCounted.new()

	script_owner.script = script

	return script_owner


func test_script_instance_get_property():
	var object = create_script_instance()

	var value: int = object.script_property_a

	assert_eq(value, 10)


func test_script_instance_set_property():
	var object = create_script_instance()

	assert_eq(object.script_property_b, false)

	object.script_property_b = true

	assert_eq(object.script_property_b, true)


func test_script_instance_call():
	var object = create_script_instance()

	var arg_a = "test string"
	var arg_b = 5

	var result = object.script_method_a(arg_a, arg_b)

	assert_eq(result, "{0}{1}".format([arg_a, arg_b]))


func test_script_instance_property_list():
	var object = create_script_instance()

	var list = object.get_property_list()

	assert_eq(list[-1]["name"], "script_property_a");
	assert_eq(list[-1]["type"], Variant.Type.TYPE_INT)


func test_script_instance_method_list():
	var object = create_script_instance()

	var list = object.get_method_list()

	assert_eq(list[-1]["name"], "script_method_a")
	assert_eq(list[-1]["args"][0]["type"], Variant.Type.TYPE_STRING)
	assert_eq(list[-1]["args"][1]["type"], Variant.Type.TYPE_INT)


func test_script_instance_has_method():
	var object = create_script_instance()

	assert(object.has_method("script_method_a"));
	assert(!object.has_method("script_method_z"));


func test_script_instance_to_string():
	var object = create_script_instance()

	assert_eq(object.to_string(), "script instance to string")
