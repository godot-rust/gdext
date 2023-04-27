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