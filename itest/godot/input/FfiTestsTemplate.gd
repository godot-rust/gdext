func test_ffi():
	var ffi = RustFfi.new()
	print("[GD] RustFfi constructed: ", ffi.get_instance_id())
	var ok := true
	#(
	ok = ok && test_varcall_IDENT(ffi)
	#)
	print("[GD] end of method, RustFfi should go out of scope...")


#(
func test_varcall_IDENT(ffi: RustFfi) -> bool:
	var from_rust = ffi.return_IDENT()
	var ok1: bool = ffi.accept_IDENT(from_rust)

	var from_gdscript = VAL
	var mirrored = ffi.mirror_IDENT(from_gdscript)
	var ok2: bool = (mirrored == from_gdscript)

	return ok1 && ok2
#)
