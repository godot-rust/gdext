extends Node

func _ready():
	var tests := IntegrationTests.new()
	var status := tests.run()
	
	print()
	var exit_code: bool
	if status:
		print(" All tests PASSED.")
		exit_code = 0
	else:
		print(" Tests FAILED.")
		exit_code = 1

	print(" -- exiting.")
	get_tree().quit(exit_code)
