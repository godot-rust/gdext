mod bomb;
mod bomb_spawner;
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
