# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at https://mozilla.org/MPL/2.0/.

class_name TestStats
extends RefCounted

var num_run := 0
var num_ok := 0
var num_failed := 0

var _start_time_usec := 0
var _runtime_usec := 0

func add(ok: bool):
	num_run += 1
	if ok:
		num_ok += 1
	else:
		num_failed += 1

func all_passed() -> bool:
	# Consider 0 tests run as a failure too, because it's probably a problem with the run itself.
	return num_failed == 0 && num_run > 0

func start_stopwatch():
	_start_time_usec = Time.get_ticks_usec()

func stop_stopwatch():
	_runtime_usec += Time.get_ticks_usec() - _start_time_usec

func runtime_seconds() -> float:
	return _runtime_usec * 1.0e-6
