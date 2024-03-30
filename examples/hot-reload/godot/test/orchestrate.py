# Copyright (c) godot-rust; Bromeon and contributors.
# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at https://mozilla.org/MPL/2.0/.

# Python code because it runs outside the engine, and portably sending UDP in bash is cumbersome.

import select
import socket
import sys


def replace_line(file_path: str):
    with open(file_path, 'r') as file:
        lines = file.readlines()

    replaced = 0
    with open(file_path, 'w') as file:
        for line in lines:
            if line.strip() == 'fn get_number(&self) -> i64 { 100 }':
                file.write(line.replace('100', '777'))
                replaced += 1
            else:
                file.write(line)

    if replaced == 0:
        print("[Python]   ERROR: Line not found in file.")
        return False
    else:
        return True


def send_udp():
    msg = bytes("reload", "utf-8")
    udp = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
    udp.sendto(msg, ("localhost", 1337))
    return True


def receive_udp() -> bool:
    timeout = 20
    udp = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
    udp.bind(("localhost", 1338))

    ready = select.select([udp], [], [], 20)
    if ready[0]:
        # If data is ready, receive it (max 1024 bytes)
        data, addr = udp.recvfrom(1024)
        print(f"[Python]   Await ready; received from {addr}: {data}")
        return True
    else:
        # If no data arrives within the timeout, exit with code 1
        print(f"[Python]   ERROR: Await timeout: no packet within {timeout} seconds.")
        return False


# -----------------------------------------------------------------------------------------------------------------------------------------------
# Main

if len(sys.argv) != 2:
    print(f"Usage: {sys.argv[0]} [replace|notify]")
    sys.exit(2)

# Get the command from the command line
command = sys.argv[1]

# Dispatch based on the command
if command == 'await':
    print("[Python]   Await Godot to be ready...")
    ok = receive_udp()
elif command == 'replace':
    print("[Python]   Replace source code...")
    ok = replace_line("../../rust/src/lib.rs")
elif command == 'notify':
    print("[Python]   Notify Godot about change...")
    ok = send_udp()
else:
    print("[Python]   ERROR: Invalid command.")
    sys.exit(2)

if not ok:
    sys.exit(1)

print("[Python]   Done.")
