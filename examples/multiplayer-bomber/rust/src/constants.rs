/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

pub const FLAG_CONNECT_DEFERRED: u32 = 1;

pub const SIGNAL_CHILD_EXITING_TREE: &str = "child_exiting_tree";
pub const SIGNAL_SCORE_INCREASED: &str = "score_increased";
pub const SIGNAL_CONNECTION_SUCCEEDED: &str = "connection_succeeded";
pub const SIGNAL_CONNECTION_FAILED: &str = "connection_failed";
pub const SIGNAL_PLAYER_LIST_CHANGED: &str = "player_list_changed";
pub const SIGNAL_GAME_ENDED: &str = "game_ended";
pub const SIGNAL_GAME_ERROR: &str = "game_error";
pub const SIGNAL_GAME_STARTED: &str = "game_started";
pub const SIGNAL_PEER_CONNECTED: &str = "peer_connected";
pub const SIGNAL_PEER_DISCONNECTED: &str = "peer_disconnected";
pub const SIGNAL_CONNECTED_TO_SERVER: &str = "connected_to_server";
pub const SIGNAL_SERVER_DISCONNECTED: &str = "server_disconnected";

pub const SIGNAL_SPAWN_BOMB: &str = "spawn_bomb";

pub const ANIMATION_EXPLODE: &str = "explode";
pub const ANIMATION_STUNNED: &str = "stunned";

pub const ANIMATION_WALK_DOWN: &str = "walk_down";
pub const ANIMATION_WALK_UP: &str = "walk_up";
pub const ANIMATION_WALK_RIGHT: &str = "walk_right";
pub const ANIMATION_WALK_LEFT: &str = "walk_left";
pub const ANIMATION_STANDING: &str = "standing";

pub const ACTION_MOVE_LEFT: &str = "move_left";
pub const ACTION_MOVE_RIGHT: &str = "move_right";
pub const ACTION_MOVE_UP: &str = "move_up";
pub const ACTION_MOVE_DOWN: &str = "move_down";
pub const ACTION_SET_BOMB: &str = "set_bomb";
