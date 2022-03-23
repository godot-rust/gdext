extends Node3D

const GdClass = preload("res://GdClass.gd")

@onready var rust_test := $RustTest as RustTest

func _init():
	print("[GDScript] _init")

	print("[GDScript] end _init")


func _ready():
	print("[GDScript] _ready")
	print("self._to_string(): ", self)

	var msg := rust_test.test_method(12, "hello from GDScript")
	print(msg)
	#get_tree().quit()
	#return

	var res := rust_test.add(4, 6, Vector2(3, 5))
	print(res)
	var res_vec := rust_test.vec_add(Vector2(1, 2), Vector2(3, 4))
	print(res_vec)

	print_instance_id(self, "self")
	print_instance_id(GdClass.new(), "new")
	print_instance_id($MeshInstance3D, "mesh")
	print_instance_id($WorldEnvironment, "env")
	print_instance_id($DirectionalLight3D, "light")

	print()
	print("----------------------------")
	var obj = rust_test.return_obj()
	print("obj._to_string(): ", obj)
	print("obj._to_string() [explicit]: ", obj._to_string())
	print_instance_id(obj, "entity")
	rust_test.accept_obj(obj)
	print("----------------------------")
	print()

	print("[GDScript] end _ready")
	get_tree().quit()


func print_instance_id(obj, msg=null):
	var full = obj.get_instance_id()
#	var low = full & 0xFFFFFFFF
#	var high = (full / 4294967296) & 0xFFFFFFFF # Not '>> 32' because GDScript disallows shift of negative numbers

#	if msg == null:
#		print("instance id: %x%x" % [high, low])
#	else:
#		print("instance id: %x%x  (%s)" % [high, low, msg])

	if msg == null:
		print("instance id: ", full)
	else:
		print("instance id: ", full, " (", msg, ")")
	print("  _to_string(): ", obj)