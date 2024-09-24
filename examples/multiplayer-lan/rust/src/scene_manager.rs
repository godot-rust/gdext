use godot::prelude::*;

use crate::{game_manager::GameManager, player::Player};

#[derive(GodotClass)]
#[class(base=Node2D)]
pub struct SceneManager {
    #[export]
    player_scene: Gd<PackedScene>,
    base: Base<Node2D>,
}

#[godot_api]
impl INode2D for SceneManager {
    fn init(base: Base<Node2D>) -> Self {
        Self {
            player_scene: PackedScene::new_gd(),
            base,
        }
    }

    fn ready(&mut self) {
        let mut binding = GameManager::singleton();
        let mut game_manager = binding.bind_mut();
        let spawn_points = self
            .base()
            .get_tree()
            .unwrap()
            .get_nodes_in_group("PlayerSpawnPoint".into());
        // make sure all the spawn points are Node2D
        let spawn_points = spawn_points
            .iter_shared()
            .map(|spawn_point| spawn_point.cast::<Node2D>())
            .collect::<Vec<Gd<Node2D>>>();
        let mut spawn_iterator = spawn_points.iter();
        for (network_id, data) in game_manager.get_player_database().iter_mut() {
            let current_player = &mut self.player_scene.instantiate_as::<Player>();
            // add reference in player database to the instantiated player scene
            data.set_player_ref(current_player.clone());

            // setup player
            {
                let mut binding = current_player.bind_mut();
                binding.set_peer_id(*network_id);
                binding.set_username(data.name.clone());
            }

            // gotta make a new borrow since we're adding current_player as a child
            self.base_mut().add_child(&mut *current_player);

            // spawn each player next to each spawn point
            if let Some(spawn_point) = spawn_iterator.next() {
                current_player.set_global_position(spawn_point.get_global_position());
            } else {
                // start from beginning if we reached the end
                spawn_iterator = spawn_points.iter();
                current_player
                    .set_global_position(spawn_iterator.next().unwrap().get_global_position());
            }
        }
    }
}
