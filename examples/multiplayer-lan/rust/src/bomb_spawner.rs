use godot::classes::{MultiplayerSpawner, IMultiplayerSpawner};
use godot::prelude::*;

use crate::NetworkId;

#[derive(GodotClass)]
#[class(base=MultiplayerSpawner)]
pub struct BombSpawner {
    base: Base<MultiplayerSpawner>,
}

#[godot_api]
impl BombSpawner {
    #[func]
    fn _spawn_bomb(data : VariantArray) -> Gd<Node> {
        godot_print!("spawn");
        if let Ok(position) = data.get(0).unwrap().try_to::<Vector2>() {
            if let Ok(from_player) = data.get(1).unwrap().try_to::<NetworkId>() {
                if let Ok(bomb_scene) = try_load::<PackedScene>("res://bomb.tscn") {
                    godot_print!("{position} {from_player}");
                    let mut bomb = bomb_scene.instantiate().unwrap();
                    bomb.set("position".into(), &data.get(0).unwrap());
                    bomb.set("from_player".into(), &data.get(1).unwrap());
                }
            }
        }

        Node::new_alloc()
    }
}

#[godot_api]
impl IMultiplayerSpawner for BombSpawner {
    fn init(base: Base<MultiplayerSpawner>) -> Self {
        Self {
            base
        }
    }

    fn enter_tree(&mut self){
        let spawn_function = self.base().callable("_spawn_bomb");
        self.base_mut().set_spawn_function(spawn_function);
        godot_print!("hello");
    }
}
