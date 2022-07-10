extends Node

func _ready():
	var rust_tests := IntegrationTests.new()
	var gdscript_tests := $FfiTests
	var status: bool = rust_tests.run() && gdscript_tests.run()

	print()
	var exit_code: int
	if status:
		print(" All tests PASSED.")
		exit_code = 0
	else:
		print(" Tests FAILED.")
		exit_code = 1

	print(" -- exiting.")
	rust_tests.free()
	get_tree().quit(exit_code)
