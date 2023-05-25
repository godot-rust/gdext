# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at https://mozilla.org/MPL/2.0/.

class_name TestSuiteSpecial
extends TestSuite

var errors: Array[String] = []

func print_newline():
	errors.push_back("")

func print_error(s: String):
	errors.push_back(s)

# Run a special test case, generating a hardcoded test-case based on the outcome of the test.
func run_test(suite: Object, method_name: String) -> GDScriptTestRunner.GDScriptHardcodedTestCase:
	var callable: Callable = Callable(suite, method_name)
	
	_assertion_failed = false
	var start_time = Time.get_ticks_usec()
	var result = await callable.call()
	var end_time = Time.get_ticks_usec()

	var test_case := GDScriptTestRunner.GDScriptHardcodedTestCase.new(suite, method_name)
	test_case.execution_time_seconds = float(end_time - start_time) / 1000000.0
	test_case.result = (result or result == null) and not _assertion_failed
	test_case.errors = clear_errors()
	return test_case

func clear_errors() -> Array[String]:
	var old_errors := errors
	errors = []
	return old_errors
