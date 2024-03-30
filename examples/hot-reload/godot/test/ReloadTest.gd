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


func _ready() -> void:
	print("[GDScript] Start...")

	var r = Reloadable.new()
	var num = r.get_number()
	r.free()

	print("[GDScript] Sanity check: initial number is ", num)
	
	var extensions = GDExtensionManager.get_loaded_extensions()
	if extensions.size() == 1:
		extension_name = extensions[0]
	else:
		fail(str("Must have 1 extension, has: ", extensions))
		return

	udp.bind(1337)
	print("[GDScript] ReloadTest ready to receive...")

	send_udp()


func send_udp():
	# Attempt to bind the UDP socket to any available port for sending.
	# You can specify a port number instead of 0 if you need to bind to a specific port.
	var out_udp = PacketPeerUDP.new()

	# Set the destination address and port for the message
	if out_udp.set_dest_address("127.0.0.1", 1338) != OK:
		fail("Failed to set destination address")
		return

	if out_udp.put_packet("ready".to_utf8_buffer()) != OK:
		fail("Failed to send packet")
		return

	print("[GDScript] Packet sent successfully")
	out_udp.close()


func _exit_tree() -> void:
	print("[GDScript] ReloadTest exit.")
	udp.close()


func _process(delta: float) -> void:
	if udp.get_available_packet_count() == 0:
		return

	var packet = udp.get_packet().get_string_from_ascii()
	print("[GDScript] Received UDP packet [", packet.length(), "]: ", packet)

	if not _hot_reload():
		return

	var r = Reloadable.new()
	var num = r.get_number()
	r.free()

	if num == 777:
		print("[GDScript] Successful hot-reload! Exit...")
		get_tree().quit(0)
	else:
		fail(str("Number was not updated correctly (is ", num, ")"))
		return


func _hot_reload():
	# TODO sometimes fails because .so is not found
	var status = GDExtensionManager.reload_extension(extension_name)
	if status != OK:
		fail(str("Failed to reload extension: ", status))
		return false

	return true


func fail(s: String) -> void:
	print("::error::[GDScript] ", s) # GitHub Action syntax
	get_tree().quit(1)


