use std::{collections::HashMap};

use godot::{classes::{RandomNumberGenerator, RichTextLabel}, prelude::*};

use crate::{multiplayer_controller::{self, MultiplayerController}, player::Player, NetworkId};

#[derive(GodotClass)]
#[class(base=Node2D)]
pub struct SceneManager {
    #[export]
    player_scene: Gd<PackedScene>,
    pub player_list: HashMap<NetworkId, Gd<Player>>,
    text_log: OnReady<Gd<RichTextLabel>>,
    base: Base<Node2D>,
}

#[godot_api]
impl SceneManager {
    #[func]
    fn get_spawn_points(&self) -> Array<Gd<Node2D>> {
        let spawn_nodes = self
            .base()
            .get_tree()
            .unwrap()
            .get_nodes_in_group("PlayerSpawnPoint".into());
        // make sure all the spawn points are Node2D
        let spawn_points = spawn_nodes
            .iter_shared()
            .map(|spawn_point| spawn_point.cast::<Node2D>())
            .collect::<Array<Gd<Node2D>>>();
        spawn_points
    }

    #[func]
    pub fn add_player(&mut self, network_id: NetworkId, username: GString)
    {
        let mut player = self.player_scene.instantiate_as::<Player>();

        // setup player
        {
            let mut binding = player.bind_mut();
            binding.set_network_id(network_id);
            binding.set_username(username);
        }

        self.player_list.insert(network_id, player.clone());
        self.base_mut().add_child(player.clone());
    }

    #[func]
    fn respawn_player(&self, &mut player: Gd<Player>)
    {
        // get random spawnpoint
        let spawn_points = self.get_spawn_points();
        let mut random = RandomNumberGenerator::new_gd();
        let spawn = spawn_points.get(random.randi_range(0, spawn_points.len() as i32 - 1) as usize).unwrap();
        player.bind_mut().base_mut().set_global_position(spawn.get_global_position());
    }   

    // called only from the server
    #[rpc(authority, call_local)]
    pub fn start_game(&mut self) {
        // actually add players into the scene
        let spawn_points = self.get_spawn_points();
        let mut index = 0;
        for (_, player) in &mut self.player_list {
            // spawn each player next to each spawn point
            let spawn_position = spawn_points.at(index).get_global_position();
            player.set_global_position(spawn_position);
            
            {
                let mut bind = player.bind_mut();
                bind.set_sync_position(spawn_position);

                godot_print!("id {0} position {1}", bind.get_network_id(), bind.get_sync_position());
            }

            index += 1;
            
            if index >= spawn_points.len() {
                index = 0;
            }
        }
    }
}

#[godot_api]
impl INode2D for SceneManager {
    fn init(base: Base<Node2D>) -> Self {
        Self {
            player_scene: PackedScene::new_gd(),
            player_list: HashMap::new(),
            text_log: OnReady::from_base_fn(|base| base.get_node_as::<RichTextLabel>("TextLog")),
            base,
        }
    }

    fn ready(&mut self) {
        let mut multiplayer_controller = self.base_mut().get_tree().unwrap().get_root().unwrap().get_node_as::<MultiplayerController>("MultiplayerController");
        // Tell the server that this peer has loaded.
        multiplayer_controller.rpc_id(1, "load_in_player".into(), &[]);
    }

    
    fn process(&mut self, delta: f64) {
        let mut string = String::from("");
        for (_, player) in self.player_list.iter() {
            let player_bind = player.bind();
            let username = &player_bind.username;
            let position = player.get_global_position();
            let network_id = &player_bind.get_network_id();
            string.push_str(&format!("network_id: {network_id}, username: {username}, position: {position} \n"));
        }
        self.text_log.set_text(GString::from(string));
    }
    

}
