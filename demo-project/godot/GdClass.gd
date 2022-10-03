# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at https://mozilla.org/MPL/2.0/.

# Standalone script, not attached to any nodes

extends RefCounted
class_name GdClass

func _to_string():
	return "Custom string repr"
