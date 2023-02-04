# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at https://mozilla.org/MPL/2.0/.

extends Node

func _ready():
	var test_suites: Array = [
		IntegrationTests.new(),
		preload("res://ManualFfiTests.gd").new(),
		preload("res://gen/GenFfiTests.gd").new(),
	]
	
	var tests: Array[_Test] = []
	for suite in test_suites:
		for method in suite.get_method_list():
			var method_name: String = method.name
			if method_name.begins_with("test_"):
				tests.push_back(_Test.new(suite, method_name))
	
	print()
	print_rich("  [b][color=green]Running[/color][/b] test project %s" % [
		ProjectSettings.get_setting("application/config/name", ""),
	])
	print()
	
	var stats: TestStats = TestStats.new()
	stats.start_stopwatch()
	for test in tests:
		printraw("  -- %s ... " % [test.test_name])
		var ok: bool = test.run()
		print_rich("[color=green]ok[/color]" if ok else "[color=red]FAILED[/color]")
		stats.add(ok)
	stats.stop_stopwatch()
	
	print()
	print_rich("test result: %s. %d passed; %d failed; finished in %.2fs" % [
		"[color=green]ok[/color]" if stats.all_passed() else "[color=red]FAILED[/color]",
		stats.num_ok,
		stats.num_failed,
		stats.runtime_seconds(),
	])
	print()
	
	for suite in test_suites:
		suite.free()
	
	var exit_code: int = 0 if stats.all_passed() else 1
	get_tree().quit(exit_code)

class _Test:
	var suite: Object
	var method_name: String
	var test_name: String
	
	func _init(suite: Object, method_name: String):
		self.suite = suite
		self.method_name = method_name
		self.test_name = "%s::%s" % [_suite_name(suite), method_name]
	
	func run():
		# This is a no-op if the suite doesn't have this property.
		suite.set("_assertion_failed", false)
		var result = suite.call(method_name)
		var ok: bool = (
			(result == true or result == null)
			&& !suite.get("_assertion_failed")
		)
		return ok
	
	static func _suite_name(suite: Object) -> String:
		var script := suite.get_script()
		if script:
			# Test suite written in GDScript.
			return script.resource_path.get_file().get_basename()
		else:
			# Test suite written in Rust.
			return suite.get_class()
