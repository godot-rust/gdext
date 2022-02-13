extends Node3D

@onready var rust_test := $RustTest as RustTest

func _ready():
	var msg := rust_test.test_method(12, "hello from GDScript")
	print(msg)
	var res := rust_test.add(4, 6)
	print_debug(res)
