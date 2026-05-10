# Copyright (c) godot-rust; Bromeon and contributors.
# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at https://mozilla.org/MPL/2.0/.

@tool # To ensure that itest is not run from the editor; see below.

extends Node
class_name GDScriptTestRunner

func _ready():
	# Don't run tests when opened in the editor, unless it's headless mode (-e --headless).
	# When editor tests are run, we skip GDScript suites and benchmarks -- they aren't editor-specific.
	var editor_only_run := false
	if Engine.is_editor_hint():
		if DisplayServer.get_name() == 'headless':
			print("Opened itest in editor in headless mode -> run editor integration tests.")
			editor_only_run = true
		else:
			print("Opened itest in editor in UI mode -> skip integration tests.")
			return

	# Ensure physics is initialized, for tests that require it.
	await get_tree().physics_frame

	var allow_focus := true
	var filters: Array = []
	var unrecognized_args: Array = []
	for arg in OS.get_cmdline_user_args():
		match arg:
			"--disallow-focus":
				allow_focus = false
			_:
				if not arg.begins_with("[") or not arg.ends_with("]"):
					unrecognized_args.push_back(arg)

				var args = arg.lstrip("[").rstrip("]").split(",")
				filters.append_array(args)

	if unrecognized_args:
		push_error("Unrecognized arguments: ", unrecognized_args)
		get_tree().quit(2)
		return

	var rust_runner = IntegrationTests.new()

	var gdscript_suites: Array = [] if editor_only_run else [
		load("res://ManualFfiTests.gd").new(),
		load("res://gen/GenFfiTests.gd").new(),
		load("res://InheritTests.gd").new(),
		load("res://ScriptInstanceTests.gd").new(),
		load("res://SpecialTests.gd").new(),
	]

	var gdscript_focused_run := _has_focused_tests(gdscript_suites)
	var prefix := "focus_test_" if gdscript_focused_run else "test_"

	var gdscript_tests: Array = []
	for suite in gdscript_suites:
		for method in suite.get_method_list():
			if method.name.begins_with(prefix):
				# Always use `await` -- it does nothing on synchronous run_test() methods.
				gdscript_tests.push_back(await suite.run_test(suite, method.name))

	var property_tests = load("res://gen/GenPropertyTests.gd").new()

	# Run benchmarks after all synchronous and asynchronous tests have completed.
	# Skipped in editor-only mode -- benchmarks are not editor-specific.
	var run_benchmarks = func (success: bool):
		if success and not editor_only_run:
			rust_runner.run_all_benchmarks(self)

		var exit_code: int = 0 if success else 1
		get_tree().quit(exit_code)

	rust_runner.run_all_tests(
		gdscript_tests,
		gdscript_suites.size(),
		allow_focus,
		gdscript_focused_run,
		self,
		filters,
		property_tests,
		run_benchmarks
	)


func _has_focused_tests(suites: Array) -> bool:
	for suite in suites:
		for method in suite.get_method_list():
			if method.name.begins_with("focus_test_"):
				return true
	return false


class GDScriptTestCase:
	var suite: RefCounted # not always TestSuite, e.g. InheritTests.
	var method_name: String
	var suite_name: String
	
	func _init(suite: RefCounted, method_name: String):
		self.suite = suite
		self.method_name = method_name
		self.suite_name = _suite_name(suite)

	func run():
		push_error("run unimplemented")
		return false
	
	static func _suite_name(suite: RefCounted) -> String:
		var script: GDScript = suite.get_script()
		return str(script.resource_path.get_file().get_basename(), ".gd")

# Standard test case used for whenever something can be tested by just running a GDScript function.
class GDScriptExecutableTestCase extends GDScriptTestCase:
	func run():
		# This is a no-op if the suite doesn't have this property.
		suite.reset_state()
		var result = suite.call(method_name)
		var ok: bool = (result == true or result == null) and not suite.is_test_failed()
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
