use std::thread::spawn;

use godot::{classes::RandomNumberGenerator, prelude::*};

use crate::{game_manager::GameManager, player::Player};

#[derive(GodotClass)]
#[class(base=Node2D)]
pub struct SceneManager {
    #[export]
    player_scene: Gd<PackedScene>,
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
    fn respawn_player(&self, &mut player: Gd<Player>)
    {
        // get random spawnpoint
        let spawn_points = self.get_spawn_points();
        let mut random = RandomNumberGenerator::new_gd();
        let spawn = spawn_points.get(random.randi_range(0, spawn_points.len() as i32 - 1) as usize).unwrap();
        player.bind_mut().base_mut().set_global_position(spawn.get_global_position());
    }   
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
        let spawn_points = self.get_spawn_points();
        let mut index = 0;
        for (network_id, data) in game_manager.get_player_database().iter_mut() {
            let mut current_player = self.player_scene.instantiate_as::<Player>();
            // add reference in player database to the instantiated player scene
            data.set_player_ref(current_player.clone());

            // setup player
            {
                let mut binding = current_player.bind_mut();
                binding.set_peer_id(*network_id);
                binding.set_username(data.name.clone());
            }

            // gotta make a new borrow since we're adding current_player as a child
            self.base_mut().add_child(current_player.clone());

            // spawn each player next to each spawn point
            current_player.set_global_position(spawn_points.at(index).get_global_position());

            // set up signal on death
            // TODO: figure out how to do this
            /* 
            current_player.connect("death".into(), Callable::from_fn("on_death", |args: &[&Variant]| {
                let player = *args.first().unwrap().try_to::<Gd<Player>>().unwrap();
                self.respawn_player(player);
                Ok(Variant::nil())
            }));
            */

            index += 1;
            
            if index >= spawn_points.len() {
                index = 0;
            }
        }
    }
}
