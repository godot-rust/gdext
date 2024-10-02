/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

mod bomb;
mod bomb_spawner;
mod constants;
mod game_state;
mod inputs;
mod lobby;
mod player;
mod rock;
mod score;
mod world;

use godot::prelude::*;

struct MultiplayerExample;

#[gdextension]
unsafe impl ExtensionLibrary for MultiplayerExample {}
