use godot::prelude::*;
use player::Player;

type NetworkId = i32;

mod bullet;
mod multiplayer_controller;
mod player;
mod scene_manager;

struct MultiplayerLan;

#[gdextension]
unsafe impl ExtensionLibrary for MultiplayerLan {}
