# Copyright (c) godot-rust; Bromeon and contributors.
# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at https://mozilla.org/MPL/2.0/.

# Note: This file is not part of the example, but a script used for gdext integration tests.
# You can safely ignore it.

@tool
extends Node

var udp := PacketPeerUDP.new()
var thread := Thread.new()
var extension_name: String

var retained_obj: Reloadable


func _ready() -> void:
	print("[GD Editor] Start...")

	var r = Reloadable.new()
	var num = r.get_number()
	r.free()

	# Test construction from Rust (regression test for https://github.com/godot-rust/gdext/issues/543).
	retained_obj = Reloadable.from_string("Mars")
	var planet = retained_obj.favorite_planet

	print("[GD Editor] Sanity check: initial number is ", num, "; planet is ", planet)
	
	var extensions = GDExtensionManager.get_loaded_extensions()
	if extensions.size() == 1:
		extension_name = extensions[0]
	else:
		fail(str("Must have 1 extension, has: ", extensions))
		return

	udp.bind(1337)
	print("[GD Editor] ReloadTest ready to receive...")

	send_udp()


func send_udp():
	var out_udp = PacketPeerUDP.new()

	if out_udp.set_dest_address("127.0.0.1", 1338) != OK:
		fail("Failed to set destination address")
		return

	if out_udp.put_packet("ready".to_utf8_buffer()) != OK:
		fail("Failed to send packet")
		return

	print("[GD Editor] Packet sent successfully")
	out_udp.close()


func _exit_tree() -> void:
	print("[GD Editor] ReloadTest exit.")
	udp.close()


func _process(delta: float) -> void:
	if udp.get_available_packet_count() == 0:
		return

	var packet = udp.get_packet().get_string_from_ascii()
	print("[GD Editor] Received UDP packet [", packet.length(), "]: ", packet)

	if not _hot_reload():
		return

	var r = Reloadable.new()
	var num = r.get_number()
	r.free()

	# Check if the property has been restored.
	var planet = retained_obj.favorite_planet
	retained_obj.free()

	if num == 777 and planet == "Mars":
		print("[GD Editor] Successful hot-reload! Exit...")
		get_tree().quit(0)
	elif num != 777:
		fail(str("Number was not updated correctly (is ", num, ")"))
		return
	else:
		fail(str("Planet was not restored correctly (is ", planet, ")"))
		return


func _hot_reload():
	var status = GDExtensionManager.reload_extension(extension_name)
	if status != OK:
		fail(str("Failed to reload extension: ", status))
		return false

	return true


func fail(s: String) -> void:
	print("::error::[GD Editor] ", s) # GitHub Action syntax
	get_tree().quit(1)


