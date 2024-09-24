use std::{collections::HashMap};

use godot::{classes::RandomNumberGenerator, prelude::*};

use crate::{player::Player, NetworkId, PlayerData};

#[derive(GodotClass)]
#[class(base=Node2D)]
pub struct SceneManager {
    #[export]
    player_scene: Gd<PackedScene>,
    pub player_database: HashMap<NetworkId, PlayerData>,
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
            player_database: HashMap::new(),
            base,
        }
    }

    fn ready(&mut self) {
        let mut player_vec = Vec::<Gd<Player>>::new();
        // set up players
        for (network_id, data) in self.player_database.iter_mut() {
            let mut player = self.player_scene.instantiate_as::<Player>();
            // add reference in player database to the instantiated player scene
            data.set_player_ref(player.clone());

            // setup player
            {
                let mut binding = player.bind_mut();
                binding.set_network_id(*network_id);
                binding.set_username(data.name.clone());
            }
            
            player_vec.push(player);
        }

        // actually add them into the scene
        let spawn_points = self.get_spawn_points();
        let mut index = 0;
        for mut player in player_vec {
            // gotta make a new borrow since we're adding current_player as a child
            self.base_mut().add_child(player.clone());

            // spawn each player next to each spawn point
            player.set_global_position(spawn_points.at(index).get_global_position());

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
