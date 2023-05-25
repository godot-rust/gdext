# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at https://mozilla.org/MPL/2.0/.

extends Node
class_name GDScriptTestRunner

func _ready():
	# Ensure physics is initialized, for tests that require it.
	await get_tree().physics_frame

	var allow_focus := true
	var unrecognized_args: Array = []
	for arg in OS.get_cmdline_user_args():
		match arg:
			"--disallow-focus":
				allow_focus = false
			_:
				unrecognized_args.push_back(arg)

	if unrecognized_args:
		push_error("Unrecognized arguments: ", unrecognized_args)
		get_tree().quit(2)
		return

	var rust_runner = IntegrationTests.new()

	var gdscript_suites: Array = [
		load("res://ManualFfiTests.gd").new(),
		load("res://gen/GenFfiTests.gd").new(),
		load("res://InheritTests.gd").new()
	]
	
	var gdscript_tests: Array = []
	for suite in gdscript_suites:
		for method in suite.get_method_list():
			var method_name: String = method.name
			if method_name.begins_with("test_"):
				gdscript_tests.push_back(GDScriptExecutableTestCase.new(suite, method_name))

	var special_case_test_suites: Array = [
		load("res://SpecialTests.gd").new(),
	]

	for suite in special_case_test_suites:
		for method in suite.get_method_list():
			var method_name: String = method.name
			if method_name.begins_with("test_"):
				gdscript_tests.push_back(await suite.run_test(suite, method_name))

	var success: bool = rust_runner.run_all_tests(
		gdscript_tests,
		gdscript_suites.size(),
		allow_focus,
		self,
	)

	var exit_code: int = 0 if success else 1
	get_tree().quit(exit_code)


class GDScriptTestCase:
	var suite: Object
	var method_name: String
	var suite_name: String
	
	func _init(suite: Object, method_name: String):
		self.suite = suite
		self.method_name = method_name
		self.suite_name = _suite_name(suite)

	func run():
		push_error("run unimplemented")
		return false
	
	static func _suite_name(suite: Object) -> String:
		var script: GDScript = suite.get_script()
		return str(script.resource_path.get_file().get_basename(), ".gd")

# Standard test case used for whenever something can be tested by just running a gdscript function.
class GDScriptExecutableTestCase extends GDScriptTestCase:
	func run():
		# This is a no-op if the suite doesn't have this property.
		suite.set("_assertion_failed", false)
		var result = suite.call(method_name)
		var ok: bool = (result == true || result == null) && !suite.get("_assertion_failed")
		return ok

# Hardcoded test case used for special cases where the standard testing API is not sufficient.
#
# Stores the errors generated during the execution, so they can be printed when it is appropriate to do so.
# As we may not run this test case at the time we say we do in the terminal.
class GDScriptHardcodedTestCase extends GDScriptTestCase:
	# Errors generated during execution of the test.
	var errors: Array[String] = []
	var execution_time_seconds: float = 0
	var result: bool = false

	func run():
		return result
