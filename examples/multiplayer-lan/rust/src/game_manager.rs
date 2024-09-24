use std::collections::HashMap;

use godot::prelude::*;

use crate::NetworkId;

#[derive(GodotClass)]
#[class(init)]
pub struct PlayerData {
    pub name: GString,
    pub score: i64,
}

#[derive(GodotClass)]
#[class(init, base=Object)]
pub struct GameManager {
    player_data: HashMap<NetworkId, PlayerData>,
    base: Base<Object>,
}

#[godot_api] 
impl GameManager {

    // Rust only functions
    pub fn get_as_singleton() -> Option::<Gd<GameManager>>
    {
        if let Some(game_manager) = godot::classes::Engine::singleton().get_singleton(StringName::from("GameManager")) {
            if let Ok(game_manager) = game_manager.try_cast::<GameManager>() {
                return Some(game_manager);
            }
        }
        None
    }

    pub fn get_player_data(&self) -> &HashMap<NetworkId, PlayerData>
    {
        &self.player_data
    }

    // Expose these to GDScript

    // Will panic on the GDScript side if there isnt a network id there
    #[func]
    fn get_player(&mut self, network_id: NetworkId) -> Dictionary
    {
        let player_data = self.player_data.get(&network_id).unwrap();
        dict![
            "id" : network_id,
            "name" : player_data.name.clone(),
            "score" : player_data.score,
        ]
    }

    #[func]
    fn get_list_of_players(&self) -> Array<NetworkId>
    {
        let mut array = Array::<NetworkId>::new();
        for &network_id in self.player_data.keys() {
            array.push(network_id);
        }
        array
    }

    #[func]
    fn remove_player(&mut self, network_id: NetworkId)
    {
        self.player_data.remove(&network_id);
    }

    #[func]
    fn add_player(&mut self, network_id: NetworkId, name: GString, score: i64)
    {
        self.player_data.entry(network_id).or_insert(PlayerData{name, score});
    }

    #[func]
    fn update_score(&mut self, network_id: NetworkId, score : i64)
    {
        self.player_data.entry(network_id).and_modify(|data| data.score = score);
    }

}