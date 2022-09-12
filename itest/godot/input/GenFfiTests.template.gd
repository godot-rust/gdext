extends Node

func run() -> bool:
	var ffi = GenFfi.new()
	print("[GD] GenFfi constructed: ", ffi.get_instance_id())
	var ok := true#(
	if !test_varcall_IDENT(ffi):
		ok = false
		push_error("  -- FFI test failed: test_varcall_IDENT")
	#)

	print("[GD] GenFfi destructing...")
	return ok

#(
func test_varcall_IDENT(ffi: GenFfi) -> bool:
	var from_rust = ffi.return_IDENT()
	var ok1: bool = ffi.accept_IDENT(from_rust)

	var from_gdscript = VAL
	var mirrored = ffi.mirror_IDENT(from_gdscript)
	var ok2: bool = (mirrored == from_gdscript)

	return ok1 && ok2
#)
