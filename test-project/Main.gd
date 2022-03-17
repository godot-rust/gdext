extends Node3D

@onready var rust_test := $RustTest as RustTest

func _init():
	print("[GDScript] _init")

	print("[GDScript] end _init")


func _ready():
	print("[GDScript] _ready")

	var msg := rust_test.test_method(12, "hello from GDScript")
	print(msg)
	var res := rust_test.add(4, 6, Vector2(3, 5))
	print(res)
	var res_vec := rust_test.vec_add(Vector2(1, 2), Vector2(3, 4))
	print(res_vec)

	print("[GDScript] end _ready")
