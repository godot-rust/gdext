extends Node

func run() -> bool:
	print("Run GDScript tests...")
	var status := test_int()
	print("GDScript tests done (passed=", status, ")")

	return status

func test_int() -> bool:
	var ffi = RustFfi.new()
	var from_rust = ffi.create_int()
	var ok: bool = ffi.accept_int(from_rust)
	
	var from_gdscript = 38821
	var mirrored = ffi.mirror_int(from_gdscript)
	
	return ok && mirrored == from_gdscript
