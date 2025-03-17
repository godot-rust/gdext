# Copyright (c) godot-rust; Bromeon and contributors.
# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at https://mozilla.org/MPL/2.0/.

# See https://docs.godotengine.org/en/stable/tutorials/editor/command_line_tutorial.html#running-a-script.

class_name ReloadOrchestrator
extends SceneTree

const TIMEOUT_S = 30
const TO_GODOT_PORT = 1337
const FROM_GODOT_PORT = 1338

var run_main_loop: bool = false
var elapsed: float = 0.0
var udp := PacketPeerUDP.new()
var args: Array # must be Array for match, not PackedStringArray.

func _initialize():
	args = OS.get_cmdline_user_args()
	print("[GD Orch]   Start ", args)

	var ok := true
	match args:
		["await"]:
			ok = receive_udp()
			run_main_loop = true

		["replace"]:
			print("[GD Orch]   Replace source code...")
			ok = replace_line("../rust/src/lib.rs")

		["notify"]:
			print("[GD Orch]   Notify Godot about change...")
			ok = send_udp()

		_:
			fail("Invalid command-line args")
			ok = false

	if not ok:
		quit(1)
		return


func _finalize():
	udp.close()
	print("[GD Orch]   Stop ", args)

func _process(delta: float) -> bool:
	if not run_main_loop:
		return true
	
	elapsed += delta
	if elapsed > TIMEOUT_S:
		fail(str("Timed out waiting for Godot (", TIMEOUT_S, " seconds)."))
		return true
		
	if udp.get_available_packet_count() == 0:
		return false

	var packet = udp.get_packet().get_string_from_ascii()
	print("[GD Orch]   Received UDP packet [", packet.length(), "]: ", packet)
	return true


func replace_line(file_path: String) -> bool:
	var file = FileAccess.open(file_path, FileAccess.READ)
	if file == null:
		return false
		
	var lines = []
	while not file.eof_reached():
		lines.append(file.get_line())
	file.close()

	var replaced = 0
	file = FileAccess.open(file_path, FileAccess.WRITE)
	for line: String in lines:
		if line.strip_edges() == 'fn get_number(&self) -> i64 { 100 }':
			file.store_line(line.replace("100", "777"))
			replaced += 1
		else:
			file.store_line(line)
	file.close()

	if replaced == 0:
		fail("Line not found in file.")
		return false
	else:
		return true


func receive_udp() -> bool:
	if udp.bind(FROM_GODOT_PORT) != OK:
		fail("Failed to bind UDP")
		return false

	print("[GD Orch]   Waiting for Godot to be ready (UDP)...")
	return true


func send_udp() -> bool:
	var out_udp = PacketPeerUDP.new()
	if out_udp.set_dest_address("127.0.0.1", TO_GODOT_PORT) != OK:
		fail("Failed to set destination address")
		return false

	if out_udp.put_packet("reload".to_utf8_buffer()) != OK:
		fail("Failed to send packet")
		return false

	print("[GD Orch]   Packet sent successfully")
	return true


func fail(s: String) -> void:
	print("::error::[GD Orch]   ", s) # GitHub Action syntax
	quit(1)
