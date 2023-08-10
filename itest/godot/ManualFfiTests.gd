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

func test_init_defaults():
	var obj = WithInitDefaults.new()

	assert_eq(obj.default_int, 0)
	assert_eq(obj.literal_int, 42)
	assert_eq(obj.expr_int, -42)

func test_to_string():
	var ffi = VirtualMethodTest.new()
	
	assert_eq(str(ffi), "VirtualMethodTest[integer=0]")

func test_export():
	var obj = HasProperty.new()

	assert_eq(obj.int_val, 0)
	obj.int_val = 1
	assert_eq(obj.int_val, 1)

	assert_eq(obj.int_val_read, 2)

	obj.int_val_write = 3
	assert_eq(obj.retrieve_int_val_write(), 3)

	assert_eq(obj.int_val_rw, 0)
	obj.int_val_rw = 4
	assert_eq(obj.int_val_rw, 4)

	assert_eq(obj.int_val_getter, 0)
	obj.int_val_getter = 5
	assert_eq(obj.int_val_getter, 5)

	assert_eq(obj.int_val_setter, 0)
	obj.int_val_setter = 5
	assert_eq(obj.int_val_setter, 5)

	obj.string_val = "test val"
	assert_eq(obj.string_val, "test val")

	var node = Node.new()
	obj.object_val = node
	assert_eq(obj.object_val, node)
	
	var texture_val_meta = obj.get_property_list().filter(
		func(el): return el["name"] == "texture_val_rw"
	).front()
	
	assert_that(texture_val_meta != null, "'texture_val_rw' is defined")
	assert_eq(texture_val_meta["hint"], PropertyHint.PROPERTY_HINT_RESOURCE_TYPE)
	assert_eq(texture_val_meta["hint_string"], "Texture")
	
	obj.free()
	node.free()

class MockObjGd extends Object:
	var i: int = 0

	func _init(i: int):
		self.i = i

func test_object_pass_to_user_func_varcall():
	var obj_test = ObjectTest.new()
	var obj: MockObjGd = MockObjGd.new(10)
	assert_eq(obj_test.pass_object(obj), 10)

func test_object_pass_to_user_func_ptrcall():
	var obj_test: ObjectTest = ObjectTest.new()
	var obj: MockObjGd = MockObjGd.new(10)
	assert_eq(obj_test.pass_object(obj), 10)

func test_object_return_from_user_func_varcall():
	var obj_test = ObjectTest.new()
	var obj: MockObjRust = obj_test.return_object() 
	assert_eq(obj.i, 42)
	obj.free()

func test_object_return_from_user_func_ptrcall():
	var obj_test: ObjectTest = ObjectTest.new()
	var obj: MockObjRust = obj_test.return_object() 
	assert_eq(obj.i, 42)
	obj.free()

class MockRefCountedGd extends RefCounted:
	var i: int = 0

	func _init(i: int):
		self.i = i

func test_refcounted_pass_to_user_func_varcall():
	var obj_test = ObjectTest.new()
	var obj: MockRefCountedGd = MockRefCountedGd.new(10)
	assert_eq(obj_test.pass_refcounted(obj), 10)

func test_refcounted_pass_to_user_func_ptrcall():
	var obj_test: ObjectTest = ObjectTest.new()
	var obj: MockRefCountedGd = MockRefCountedGd.new(10)
	assert_eq(obj_test.pass_refcounted(obj), 10)

func test_refcounted_as_object_pass_to_user_func_varcall():
	var obj_test = ObjectTest.new()
	var obj: MockRefCountedGd = MockRefCountedGd.new(10)
	assert_eq(obj_test.pass_refcounted_as_object(obj), 10)

func test_refcounted_as_object_pass_to_user_func_ptrcall():
	var obj_test: ObjectTest = ObjectTest.new()
	var obj: MockRefCountedGd = MockRefCountedGd.new(10)
	assert_eq(obj_test.pass_refcounted_as_object(obj), 10)

func test_refcounted_return_from_user_func_varcall():
	var obj_test = ObjectTest.new()
	var obj: MockRefCountedRust = obj_test.return_refcounted() 
	assert_eq(obj.i, 42)

func test_refcounted_return_from_user_func_ptrcall():
	var obj_test: ObjectTest = ObjectTest.new()
	var obj: MockRefCountedRust = obj_test.return_refcounted() 
	assert_eq(obj.i, 42)

func test_refcounted_as_object_return_from_user_func_varcall():
	var obj_test = ObjectTest.new()
	var obj: MockRefCountedRust = obj_test.return_refcounted_as_object() 
	assert_eq(obj.i, 42)

func test_refcounted_as_object_return_from_user_func_ptrcall():
	var obj_test: ObjectTest = ObjectTest.new()
	var obj: MockRefCountedRust = obj_test.return_refcounted_as_object() 
	assert_eq(obj.i, 42)

func test_custom_constructor():
	var obj = CustomConstructor.construct_object(42)
	assert_eq(obj.val, 42)
	obj.free()

func test_option_refcounted_none_varcall():
	var ffi := OptionFfiTest.new()

	var from_rust: Variant = ffi.return_option_refcounted_none()
	assert_that(ffi.accept_option_refcounted_none(from_rust), "ffi.accept_option_refcounted_none(from_rust)")

	var from_gdscript: Variant = null
	var mirrored: Variant = ffi.mirror_option_refcounted(from_gdscript)
	assert_eq(mirrored, from_gdscript, "mirrored == from_gdscript")

func test_option_refcounted_none_ptrcall():
	var ffi := OptionFfiTest.new()

	var from_rust: Object = ffi.return_option_refcounted_none()
	assert_that(ffi.accept_option_refcounted_none(from_rust), "ffi.accept_option_refcounted_none(from_rust)")

	var from_gdscript: Object = null
	var mirrored: Object = ffi.mirror_option_refcounted(from_gdscript)
	assert_eq(mirrored, from_gdscript, "mirrored == from_gdscript")

func test_option_refcounted_some_varcall():
	var ffi := OptionFfiTest.new()

	var from_rust: Variant = ffi.return_option_refcounted_some()
	assert_that(ffi.accept_option_refcounted_some(from_rust), "ffi.accept_option_refcounted_some(from_rust)")

	var from_gdscript: Variant = RefCounted.new()
	var mirrored: Variant = ffi.mirror_option_refcounted(from_gdscript)
	assert_eq(mirrored, from_gdscript, "mirrored == from_gdscript")

func test_option_refcounted_some_ptrcall():
	var ffi := OptionFfiTest.new()

	var from_rust: Object = ffi.return_option_refcounted_some()
	assert_that(ffi.accept_option_refcounted_some(from_rust), "ffi.accept_option_refcounted_some(from_rust)")

	var from_gdscript: Object = RefCounted.new()
	var mirrored: Object = ffi.mirror_option_refcounted(from_gdscript)
	assert_eq(mirrored, from_gdscript, "mirrored == from_gdscript")

func test_option_node_none_varcall():
	var ffi := OptionFfiTest.new()

	var from_rust: Variant = ffi.return_option_node_none()
	assert_that(ffi.accept_option_node_none(from_rust), "ffi.accept_option_node_none(from_rust)")

	var from_gdscript: Variant = null
	var mirrored: Variant = ffi.mirror_option_node(from_gdscript)
	assert_eq(mirrored, from_gdscript, "mirrored == from_gdscript")

func test_option_node_none_ptrcall():
	var ffi := OptionFfiTest.new()

	var from_rust: Node = ffi.return_option_node_none()
	assert_that(ffi.accept_option_node_none(from_rust), "ffi.accept_option_node_none(from_rust)")

	var from_gdscript: Node = null
	var mirrored: Node = ffi.mirror_option_node(from_gdscript)
	assert_eq(mirrored, from_gdscript, "mirrored == from_gdscript")

func test_option_node_some_varcall():
	var ffi := OptionFfiTest.new()

	var from_rust: Variant = ffi.return_option_node_some()
	assert_that(ffi.accept_option_node_some(from_rust), "ffi.accept_option_node_some(from_rust)")

	var from_gdscript: Variant = Node.new()
	var mirrored: Variant = ffi.mirror_option_node(from_gdscript)
	assert_eq(mirrored, from_gdscript, "mirrored == from_gdscript")
	from_gdscript.free()
	from_rust.free()

func test_option_node_some_ptrcall():
	var ffi := OptionFfiTest.new()

	var from_rust: Node = ffi.return_option_node_some()
	assert_that(ffi.accept_option_node_some(from_rust), "ffi.accept_option_node_some(from_rust)")

	var from_gdscript: Node = Node.new()
	var mirrored: Node = ffi.mirror_option_node(from_gdscript)
	assert_eq(mirrored, from_gdscript, "mirrored == from_gdscript")
	from_gdscript.free()
	from_rust.free()

func test_custom_property():
	var has_property: HasCustomProperty = HasCustomProperty.new()
	assert_eq(has_property.some_c_style_enum, 0)
	assert_eq(has_property.enum_as_string(), "A")
	has_property.some_c_style_enum = 1
	assert_eq(has_property.enum_as_string(), "B")
	assert_eq(has_property.some_c_style_enum, 1)

	var d: Dictionary = has_property.not_exportable
	assert_eq(d.a, 0)
	assert_eq(d.b, 0)
	has_property.not_exportable = {"a": 28, "b": 33}
	d = has_property.not_exportable
	assert_eq(d.a, 28)
	assert_eq(d.b, 33)

func test_custom_property_wrong_values_1():
	var has_property: HasCustomProperty = HasCustomProperty.new()
	disable_error_messages()
	has_property.some_c_style_enum = 10
	enable_error_messages()
	assert_fail("HasCustomProperty.some_c_style_enum should only accept integers in the range `(0 ..= 2)`")

func test_custom_property_wrong_values_2():
	var has_property: HasCustomProperty = HasCustomProperty.new()
	disable_error_messages()
	has_property.not_exportable = {"a": "hello", "b": Callable()}
	enable_error_messages()
	assert_fail("HasCustomProperty.not_exportable should only accept dictionaries with float values")

func test_option_export():
	var obj := OptionExportFfiTest.new()

	assert_eq(obj.optional, null)
	assert_eq(obj.optional_export, null)

	obj.optional = null
	obj.optional_export = null
	assert_eq(obj.optional, null)
	assert_eq(obj.optional_export, null)

	var test_node := Node.new()

	obj.optional = test_node
	obj.optional_export = test_node
	assert_eq(obj.optional, test_node)
	assert_eq(obj.optional_export, test_node)

	obj.optional = null
	obj.optional_export = null
	assert_eq(obj.optional, null)
	assert_eq(obj.optional_export, null)

	test_node.free()

func test_func_rename():
	var func_rename := FuncRename.new()

	assert_eq(func_rename.has_method("long_function_name_for_is_true"), false)
	assert_eq(func_rename.has_method("is_true"), true)
	assert_eq(func_rename.is_true(), true)

	assert_eq(func_rename.has_method("give_one_inner"), false)
	assert_eq(func_rename.has_method("give_one"), true)
	assert_eq(func_rename.give_one(), 1)

	assert_eq(func_rename.has_method("renamed_static"), false)
	assert_eq(func_rename.has_method("spell_static"), true)
	assert_eq(func_rename.spell_static(), "static")
