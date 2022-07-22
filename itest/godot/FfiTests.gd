extends Node

func run() -> bool:
	print("Run GDScript tests...")
	var ok = true
	#ok = ok && test_int()
	ok = ok && test_to_string()
	print("[GD] RustFfi now out of scope.")

	print("GDScript tests done (passed=", ok, ")")

	return ok

func test_int() -> bool:
	var ffi = RustFfi.new()
	print("[GD] RustFfi constructed: ", ffi.get_instance_id())
#	var from_rust = ffi.create_int()
#	var ok: bool = ffi.accept_int(from_rust)
	var ok = true
	
	var from_gdscript = 38821
	var mirrored = ffi.mirror_int(from_gdscript)

	print("[GD] end of method, RustFfi should go out of scope...")
	return ok && mirrored == from_gdscript

func test_to_string() -> bool:
	var ffi = VirtualMethodTest.new()
	var s = str(ffi)

	print("to_string: ", s)
	print("to_string: ", ffi)
	return true