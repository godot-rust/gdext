use godot::prelude::*;

use crate::{game_manager::GameManager, player::Player};

#[derive(GodotClass)]
#[class(base=Node2D)]
pub struct SceneManager {
    #[export]
    player_scene : Gd<PackedScene>,
    base: Base<Node2D>
}

#[godot_api]
impl INode2D for SceneManager {
    fn init(base: Base<Node2D>) -> Self {
        Self {
            player_scene : PackedScene::new_gd(),
            base,
        }
    }
    /*
# Called when the node enters the scene tree for the first time.
func _ready():
	var index = 0
	for i in GameManager.players:
		var current_player = player_scene.instantiate()
		# set the peer id that is going to control this player object
		current_player.peer_id = GameManager.players[i].id
		add_child(current_player)
		var spawn_points := get_tree().get_nodes_in_group("spawn_points")
		for spawn in spawn_points:
			if spawn.name == str(index):
				current_player.global_position = spawn.global_position
		index += 1
		if index >= spawn_points.size():
			index = 0
	pass # Replace with function body.
 */


    fn ready(&mut self)
    {
        if let Some(game_manager) = GameManager::get_as_singleton() {
            let game_manager = game_manager.bind();
            let spawn_points = self.base().get_tree().unwrap().get_nodes_in_group("PlayerSpawnPoint".into());
            // make sure all the spawn points are Node2D
            let spawn_points = spawn_points.iter_shared().map(|spawn_point| spawn_point.cast::<Node2D>()).collect::<Vec<Gd<Node2D>>>();
            let mut spawn_iterator = spawn_points.iter();
            for (&network_id, data) in game_manager.get_player_database() {
                let current_player = &mut self.player_scene.instantiate_as::<Player>();

                // setup player
                // I have no idea why i have to do this. For this specific function, I have to do bind_mut()
                {
                    let mut binding = current_player.bind_mut();
                    binding.set_peer_id(network_id);
                    binding.set_username(data.name.clone());
                }
                
                // gotta make a new borrow since we're adding current_player as a child
                self.base_mut().add_child(&mut *current_player);

                // spawn each player next to each spawn point
                if let Some(spawn_point) = spawn_iterator.next() {
                    current_player.set_global_position(spawn_point.get_global_position());
                }
                else
                {
                    // start from beginning if we reached the end
                    spawn_iterator = spawn_points.iter();
                    current_player.set_global_position(spawn_iterator.next().unwrap().get_global_position());
                }
            }
        }
    }
}