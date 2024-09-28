use godot::prelude::*;

type NetworkId = i32;

mod bullet;
mod multiplayer_controller;
mod bomb_spawner;
mod player;
mod player_controls;
mod scene_manager;

struct MultiplayerLan;

#[gdextension]
unsafe impl ExtensionLibrary for MultiplayerLan {}
