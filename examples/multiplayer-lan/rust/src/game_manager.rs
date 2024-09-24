use std::collections::HashMap;

use godot::prelude::*;

use crate::{player::Player, NetworkId};

#[derive(GodotClass)]
#[class(init)]
pub struct PlayerData {
    pub name: GString,
    pub network_id: NetworkId,
    pub score: i64,
    pub player_ref: Option<Gd<Player>>,
}

#[godot_api]
impl PlayerData{
    pub fn set_player_ref(&mut self, player_ref : Gd<Player>)
    {
        godot_print!("adding player reference for {0}", self.network_id);
        self.player_ref = Some(player_ref);
    }
}

#[derive(GodotClass)]
#[class(init, base=Object)]
pub struct GameManager {
    pub player_database: HashMap<NetworkId, PlayerData>,
    base: Base<Object>,
}

#[godot_api] 
impl GameManager {

    // Rust only functions
    pub fn singleton() -> Gd<Self>
    {
         godot::classes::Engine::singleton().get_singleton(StringName::from("GameManager")).unwrap().try_cast::<GameManager>().unwrap()
    }

    pub fn get_player_database(&mut self) -> &mut HashMap<NetworkId, PlayerData>
    {
        &mut self.player_database
    }

    // Expose these to GDScript

    // Will panic on the GDScript side if there isnt a network id there
    #[func]
    pub fn get_player_data(&mut self, network_id: NetworkId) -> Dictionary
    {
        let player_data = self.player_database.get(&network_id).unwrap();
        dict![
            "id" : network_id,
            "name" : player_data.name.clone(),
            "score" : player_data.score,
        ]
    }

    #[func]
    pub fn get_list_of_network_ids(&self) -> Array<NetworkId>
    {
        let mut array = Array::<NetworkId>::new();
        for &network_id in self.player_database.keys() {
            array.push(network_id);
        }
        array
    }

    #[func]
    pub fn get_list_of_players(&self) -> Array<Gd<Player>>
    {
        let mut array = Array::<Gd<Player>>::new();
        for data in self.player_database.values() {
            if let Some(player) = &data.player_ref {
                array.push(player);
            }
        }
        array
    }

    #[func]
    pub fn remove_player(&mut self, network_id: NetworkId)
    {
        if let Some((id, mut data)) = self.player_database.remove_entry(&network_id)
        {
            godot_print!("Removing player {id}");
            if let Some(player_ref) = &mut data.player_ref {
                player_ref.queue_free();
            }
        }
    }

    #[func]
    pub fn add_player_data(&mut self, network_id: NetworkId, name: GString, score: i64)
    {
        godot_print!("adding player {network_id}");
        self.player_database.entry(network_id).or_insert(PlayerData{name, network_id, score, player_ref: None});
    }

    /*
    #[func]
    pub fn register_player_reference(&mut self, network_id: NetworkId, player_ref: Option<Gd<Player>>)
    {
        godot_print!("adding player reference for {network_id}");
        self.player_database.entry(network_id).and_modify(|data|  data.player_ref = player_ref);
    }
    */


    #[func]
    pub fn update_score(&mut self, network_id: NetworkId, score : i64)
    {
        self.player_database.entry(network_id).and_modify(|data| data.score = score);
    }

}