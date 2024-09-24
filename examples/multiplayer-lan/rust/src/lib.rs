use player::Player;
use godot::{prelude::*};

type NetworkId = i32;

mod player;
mod bullet;
mod scene_manager;
mod multiplayer_controller;

#[derive(GodotClass, Clone)]
#[class(init)]
pub struct PlayerData {
    pub name: GString,
    pub network_id: NetworkId,
    pub score: i64,
    // reference to the Player Scene that is instantiated
    pub player_ref: Option<Gd<Player>>,
}

#[godot_api]
impl PlayerData{
    pub fn set_player_ref(&mut self, player_ref : Gd<Player>)
    {
        godot_print!("adding player reference for {0}", self.network_id);
        self.player_ref = Some(player_ref);
    }
    pub fn delete_player_ref(&mut self){
        if let Some(player_ref) = &mut self.player_ref {
            player_ref.queue_free();
        }
    }
}

struct MultiplayerLan;


#[gdextension]
unsafe impl ExtensionLibrary for MultiplayerLan { }