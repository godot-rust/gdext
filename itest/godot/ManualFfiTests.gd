# Copyright (c) godot-rust; Bromeon and contributors.
# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at https://mozilla.org/MPL/2.0/.

extends TestSuite

func test_missing_init():
	var class_found = ClassDB.class_exists("WithoutInit")
	var can_instantiate = ClassDB.can_instantiate("WithoutInit")
	var instance = ClassDB.instantiate("WithoutInit")

	assert_eq(class_found, true, "ClassDB.class_exists() is true")
	assert_eq(can_instantiate, false, "ClassDB.can_instantiate() is false")
	assert_eq(instance, null, "ClassDB.instantiate() returns null")

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

func test_export_dyn_gd():
	var dyn_gd_exporter = RefcDynGdVarDeclarer.new()

	# NodeHealth is valid candidate both for `empty` and `second` fields.
	var node = NodeHealth.new()
	dyn_gd_exporter.first = node
	assert_eq(dyn_gd_exporter.first, node)

	dyn_gd_exporter.second = node
	assert_eq(dyn_gd_exporter.second, node)

	# RefcHealth is valid candidate for `first` field.
	var refc = RefcHealth.new()
	dyn_gd_exporter.first = refc
	assert_eq(dyn_gd_exporter.first, refc)
	node.free()

func test_export_dyn_gd_should_fail_for_wrong_type():
	if runs_release():
		return

	var dyn_gd_exporter = RefcDynGdVarDeclarer.new()
	var refc = RefcHealth.new()

	expect_fail()
	dyn_gd_exporter.second = refc # Causes current function to fail. Following code unreachable.

	assert_fail("`DynGdExporter.second` should only accept NodeHealth and only if it implements `InstanceIdProvider` trait")


# Test that relaxed conversions (Variant::try_to_relaxed) are used in both varcall/ptrcall.
func test_ffi_relaxed_conversions_in_varcall_ptrcall():
	mark_test_pending()

	# Enforce varcall by having untyped object, and ptrcall by object + arguments typed.
	var varcaller: Variant = ConversionTest.new()
	var ptrcaller: ConversionTest = ConversionTest.new()

	var result1: String = ptrcaller.accept_f32(42)
	assert_eq(result1, "42", "ptrcall int->f32 should work with relaxed conversion")

	var result2: String = ptrcaller.accept_i32(42.7)
	assert_eq(result2, "42", "ptrcall float->i32 should work with relaxed conversion")

	var untyped_int: Variant = 42
	var result3 = varcaller.accept_f32(untyped_int)
	assert_eq(result3, "42", "varcall int->f32 should work with relaxed conversion")

	var untyped_float: Variant = 42.7
	var result4 = varcaller.accept_i32(untyped_float)
	assert_eq(result4, "42", "varcall float->i32 should work with relaxed conversion")

	# If we reach this point, all conversions succeeded.
	assert_eq(ConversionTest.successful_calls(), 4, "all calls should succeed with relaxed conversion")
	mark_test_succeeded()


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
	if runs_release():
		return

	var has_property: HasCustomProperty = HasCustomProperty.new()
	expect_fail()
	has_property.some_c_style_enum = 10 # Causes current function to fail. Following code unreachable.

	assert_fail("HasCustomProperty.some_c_style_enum should only accept integers in the range `(0 ..= 2)`")

func test_custom_property_wrong_values_2():
	if runs_release():
		return

	var has_property: HasCustomProperty = HasCustomProperty.new()
	expect_fail()
	has_property.not_exportable = {"a": "hello", "b": Callable()} # Causes current function to fail. Following code unreachable.

	assert_fail("HasCustomProperty.not_exportable should only accept dictionaries with float values")

func test_phantom_var():
	var obj := HasPhantomVar.new()

	assert_eq(obj.read_only, 0)
	assert_eq(obj.read_write, 0)

	obj.read_write = 1

	assert_eq(obj.read_only, 1)
	assert_eq(obj.read_write, 1)

func test_phantom_var_writing_read_only():
	if runs_release():
		return

	# This must be untyped, otherwise the parser complains about our invalid write.
	var obj = HasPhantomVar.new()
	expect_fail()
	obj.read_only = 1
	assert_fail("HasPhantomVar.read_only should not be writable")

func test_option_export():
	var obj := OptionExportFfiTest.new()

	assert_eq(obj.optional, null)
	assert_eq(obj.optional_export, null)

	obj.optional = null
	obj.optional_export = null
	assert_eq(obj.optional, null)
	assert_eq(obj.optional_export, null)

	var test_node := Node.new()
	var test_resource := Resource.new()

	obj.optional = test_node
	obj.optional_export = test_resource
	assert_eq(obj.optional, test_node)
	assert_eq(obj.optional_export, test_resource)

	obj.optional = null
	obj.optional_export = null
	assert_eq(obj.optional, null)
	assert_eq(obj.optional_export, null)

	test_node.free()

func test_func_rename():
	var func_rename := FuncObj.new()

	assert_eq(func_rename.has_method("long_function_name_for_is_true"), false)
	assert_eq(func_rename.has_method("is_true"), true)
	assert_eq(func_rename.is_true(), true)

	assert_eq(func_rename.has_method("give_one_inner"), false)
	assert_eq(func_rename.has_method("give_one"), true)
	assert_eq(func_rename.give_one(), 1)

	assert_eq(func_rename.has_method("renamed_static"), false)
	assert_eq(func_rename.has_method("spell_static"), true)
	assert_eq(func_rename.spell_static(), "static")

func test_init_panic():
	var obj := InitPanic.new() # panics in Rust
	assert_eq(obj, null, "Rust panic in init() returns null in GDScript")

	# Alternative behavior (probably not desired):
	# assert_eq(obj.get_class(), "RefCounted", "panic in init() returns base instance without GDExtension part")

var gd_self_obj: GdSelfObj
func update_self_reference(value):
	gd_self_obj.update_internal(value)

# TODO: Once there is a way to assert for a SCRIPT ERROR failure, this can be re-enabled.
#func test_gd_self_obj_fails():
#	# Create the gd_self_obj and connect its signal to a gdscript method that calls back into it.
#	gd_self_obj = GdSelfObj.new()
#	gd_self_obj.update_internal_signal.connect(update_self_reference)
#	
#	# The returned value will still be 0 because update_internal can't be called in update_self_reference due to a borrowing issue.
#	assert_eq(gd_self_obj.fail_to_update_internal_value_due_to_conflicting_borrow(10), 0)

func test_gd_self_obj_succeeds():
	# Create the gd_self_obj and connect its signal to a gdscript method that calls back into it.
	gd_self_obj = GdSelfObj.new()
	gd_self_obj.update_internal_signal.connect(update_self_reference)

	assert_eq(gd_self_obj.succeed_at_updating_internal_value(10), 10)

func sample_func():
	pass

func test_callable_refcount():
	var test_obj: CallableRefcountTest = CallableRefcountTest.new()
	for i in range(10):
		var method := Callable(self, "sample_func")
		test_obj.accept_callable(method)
	var method := Callable(self, "sample_func")
	assert(method.is_valid())
	test_obj.free()

func test_get_set():
	var obj: GetSetTest = GetSetTest.new()
	assert(not obj.is_get_called())
	assert(not obj.is_set_called())

	assert_eq(obj.always_get_100, 100)
	assert(obj.is_get_called())
	assert(not obj.is_set_called())
	obj.unset_get_called()

	obj.always_get_100 = 10
	assert_eq(obj.always_get_100, 100)
	assert_eq(obj.get_real_always_get_100(), 10)
	assert(obj.is_set_called())
	obj.unset_get_called()
	obj.unset_set_called()

	obj.set_get = 1000
	assert_eq(obj.set_get, 1000)
	assert(obj.is_set_called())
	assert(obj.is_get_called())


# Validates the shape of the class defined in Rust:
# - Rust declares a single property (int_val) and two functions (f1 and f2).
# - In addition, Godot defines a property with the name of the class, which acts as the top-level category in the inspector UI.
func test_renamed_func_shape():
	# Note: RenamedFunc is located in property_test.rs.
	var obj: RenamedFunc = RenamedFunc.new()

	# Get baseline Node properties and methods
	var base_node = Node.new()
	var node_props = base_node.get_property_list().map(func(p): return p.name)
	var node_methods = base_node.get_method_list().map(func(m): return m.name)
	base_node.free()
	
	# Get our object's properties and methods
	var obj_props = obj.get_property_list().map(func(p): return p.name)
	var obj_methods = obj.get_method_list().map(func(m): return m.name)
	
	# Get only the new properties and methods (not in Node)
	var gdext_props = obj_props.filter(func(name): return not node_props.has(name))
	var gdext_methods = obj_methods.filter(func(name): return not node_methods.has(name))

	# Assert counts
	assert_eq(gdext_props.size(), 2, "number of properties should be 2")
	assert_eq(gdext_methods.size(), 2, "number of methods should be 2")
	
	# Assert specific names
	assert(gdext_props.has("int_val"), "should have a property named 'int_val'")
	# Godot automatically adds a property of the class name (acts as the top-level category in the inspector UI).
	assert(gdext_props.has("RenamedFunc"), "should have a property named 'RenamedFunc'")
	assert(gdext_methods.has("f1"), "should have a method named 'f1'")
	assert(gdext_methods.has("f2"), "should have a method named 'f2'")

	obj.free()


# Validates that the property has been linked to the correct rust get/set functions.
func test_renamed_func_get_set():
	# Note: RenamedFunc is located in property_test.rs.
	var obj: RenamedFunc = RenamedFunc.new()

	assert_eq(obj.int_val, 0)
	assert_eq(obj.f1(), 0)

	obj.int_val = 42;
	
	assert_eq(obj.int_val, 42)
	assert_eq(obj.f1(), 42)

	obj.f2(84)
	
	assert_eq(obj.int_val, 84)
	assert_eq(obj.f1(), 84)

	obj.free()

# -----------------------------------------------------------------------------------------------------------------------------------------------
# Tests below verify the following:
# Calling a typed Rust function with a Variant that cannot be converted to the Rust type will cause a failed function call on _GDScript_ side,
# meaning the GDScript function aborts immediately. This happens because a `Variant -> T` conversion occurs dynamically *on GDScript side*,
# before the Rust function is called.In contrast, panics inside the Rust function (e.g. variant.to::<T>()) just cause the *Rust* function to fail.
#
# Store arguments as Variant, as GDScript wouldn't parse script otherwise. Results in varcall being used.

func test_marshalling_fail_variant_type():
	if runs_release():
		return

	# Expects Object, pass GString.
	var obj := ObjectTest.new()
	var arg: Variant = "not an object"
	expect_fail()
	obj.pass_object(arg) # Causes current function to fail. Following code unreachable.

	assert_fail("GDScript function should fail after marshalling error (bad variant type)")

func test_marshalling_fail_non_null():
	if runs_release():
		return

	# Expects Object, pass null.
	var obj := ObjectTest.new()

	expect_fail()
	obj.pass_object(null) # Causes current function to fail. Following code unreachable.

	assert_fail("GDScript function should fail after marshalling error (required non-null)")

func test_marshalling_fail_integer_overflow():
	if runs_release():
		return

	# Expects i32. This overflows.
	var obj := ObjectTest.new()
	var arg: Variant = 9223372036854775807

	expect_fail()
	obj.pass_i32(arg) # Causes current function to fail. Following code unreachable.

	assert_fail("GDScript function should fail after marshalling error (int overflow)")

func test_marshalling_continues_on_panic():
	mark_test_pending()

	# Expects i32. This overflows.
	var obj := ObjectTest.new()
	var result = obj.cause_panic() # Fails in Rust, current function continues.

	assert_eq(result, Vector3.ZERO, "Default value returned on failed function call")
	mark_test_succeeded()
