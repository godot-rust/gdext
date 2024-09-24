use player::Player;
use godot::{prelude::*};

type NetworkId = i32;

mod player;
mod bullet;
mod scene_manager;
mod multiplayer_controller;

struct MultiplayerLan;


#[gdextension]
unsafe impl ExtensionLibrary for MultiplayerLan { }