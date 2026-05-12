# Copyright (c) godot-rust; Bromeon and contributors.
# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at https://mozilla.org/MPL/2.0/.

extends TestSuite

func create_script_instance() -> Array:
	var language: TestScriptLanguage = TestScriptLanguage.new()
	var script: TestScript = language.new_script()
	var script_owner = RefCounted.new()

	script_owner.script = script

	return [script_owner, language]


func test_script_instance_get_property():
	var tuple := create_script_instance()
	var object: RefCounted = tuple[0]
	var language: TestScriptLanguage = tuple[1]

	var value: int = object.script_property_a

	assert_eq(value, 10)
	language.free()


func test_script_instance_set_property():
	var tuple := create_script_instance()
	var object: RefCounted = tuple[0]
	var language: TestScriptLanguage = tuple[1]

	assert_eq(object.script_property_b, false)

	object.script_property_b = true

	assert_eq(object.script_property_b, true)
	language.free()


func test_script_instance_call():
	var tuple := create_script_instance()
	var object: RefCounted = tuple[0]
	var language: TestScriptLanguage = tuple[1]

	var arg_a = "test string"
	var arg_b = 5

	var result = object.script_method_a(arg_a, arg_b)

	assert_eq(result, "{0}{1}".format([arg_a, arg_b]))
	language.free()


func test_script_instance_property_list():
	var tuple := create_script_instance()
	var object: RefCounted = tuple[0]
	var language: TestScriptLanguage = tuple[1]

	var list = object.get_property_list()

	assert_eq(list[-1]["name"], "script_property_a");
	assert_eq(list[-1]["type"], Variant.Type.TYPE_INT)
	language.free()


func test_script_instance_method_list():
	var tuple := create_script_instance()
	var object: RefCounted = tuple[0]
	var language: TestScriptLanguage = tuple[1]

	var list = object.get_method_list()

	assert_eq(list[-1]["name"], "script_method_a")
	assert_eq(list[-1]["args"][0]["type"], Variant.Type.TYPE_STRING)
	assert_eq(list[-1]["args"][1]["type"], Variant.Type.TYPE_INT)
	language.free()


func test_script_instance_has_method():
	var tuple := create_script_instance()
	var object: RefCounted = tuple[0]
	var language: TestScriptLanguage = tuple[1]

	assert(object.has_method("script_method_a"));
	assert(!object.has_method("script_method_z"));
	language.free()


func test_script_instance_to_string():
	var tuple := create_script_instance()
	var object: RefCounted = tuple[0]
	var language: TestScriptLanguage = tuple[1]

	assert_eq(object.to_string(), "script instance to string")
	language.free()


func test_script_instance_mut_call():
	var tuple := create_script_instance()
	var object: RefCounted = tuple[0]
	var language: TestScriptLanguage = tuple[1]
	var before = object.script_property_b
	
	var result = object.script_method_toggle_property_b()

	assert(result)
	assert_eq(object.script_property_b, !before)
	language.free()


func test_script_instance_re_entering_call():
	var tuple := create_script_instance()
	var object: RefCounted = tuple[0]
	var language: TestScriptLanguage = tuple[1]
	var before = object.script_property_b
	
	var result = object.script_method_re_entering()

	assert(result)
	assert_eq(object.script_property_b, !before)
	language.free()


func test_object_script_instance():
	var object = Node.new()
	var language: TestScriptLanguage = TestScriptLanguage.new()
	var script: TestScript = language.new_script()

	object.script = script

	var result = object.script_method_re_entering()

	assert(result)
	object.free()
	language.free()


func test_script_instance_property_can_revert():
	var tuple := create_script_instance()
	var object: RefCounted = tuple[0]
	var language: TestScriptLanguage = tuple[1]

	var can_revert = object.property_can_revert("revertible_property")
	assert(can_revert)

	can_revert = object.property_can_revert("other_property")
	assert(!can_revert)
	
	language.free()


func test_script_instance_property_get_revert():
	var tuple := create_script_instance()
	var object: RefCounted = tuple[0]
	var language: TestScriptLanguage = tuple[1]

	var revert_value = object.property_get_revert("revertible_property")
	assert_eq(revert_value, 42)

	revert_value = object.property_get_revert("other_property")
	assert_eq(revert_value, null)
	
	language.free()


func test_script_instance_get_class_category():
	var tuple := create_script_instance()
	var object: RefCounted = tuple[0]
	var language: TestScriptLanguage = tuple[1]
	var property_found := false

	var property_list = object.get_property_list()

	for prop in property_list:
		if prop["name"] == "test_script_name":
			property_found = true

	assert(property_found)
	language.free()


func test_script_instance_property_validate():
	var language: TestScriptLanguage = TestScriptLanguage.new()
	var script: TestScript = language.new_script()
	var object = Node.new()

	object.script = script

	var property_list = object.get_property_list()
	var target_prop = null

	for prop in property_list:
		if prop["name"] == "owner":
			target_prop = prop

	assert(target_prop != null)
	assert_eq(target_prop["usage"], PropertyUsageFlags.PROPERTY_USAGE_ALWAYS_DUPLICATE)

	language.free()
	object.free()


func test_script_instance_notification():
	var tuple := create_script_instance()
	var object: RefCounted = tuple[0]
	var language: TestScriptLanguage = tuple[1]

	object.notification(Object.NOTIFICATION_PREDELETE)
	assert_eq(object.get_meta("last_notification"), Object.NOTIFICATION_PREDELETE)

	language.free()
