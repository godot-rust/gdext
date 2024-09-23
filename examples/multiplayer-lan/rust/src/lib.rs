use godot::prelude::*;

mod player;
mod bullet;

struct MultiplayerLan;

#[gdextension]
unsafe impl ExtensionLibrary for MultiplayerLan {}