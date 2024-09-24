use godot::prelude::*;
use godot::classes::{CharacterBody2D, ICharacterBody2D, MultiplayerSynchronizer, ProjectSettings};
use godot::global::{move_toward};

use crate::bullet::Bullet;
use crate::NetworkId;
#[derive(GodotClass)]
#[class(base=CharacterBody2D)]
pub struct Player {
    speed : f32,
    jump_velocity : f32,
    gravity : f64,
    #[export]
    bullet_scene : Gd<PackedScene>,
    // multiplayer stuff
    multiplayer_synchronizer : OnReady<Gd<MultiplayerSynchronizer>>,
    #[export]
    username: GString,
    #[export]
    peer_id: NetworkId,
    #[export]
    sync_position: Vector2,
    #[export]
    sync_rotation: f32,
    base: Base<CharacterBody2D>
}

#[godot_api]
impl Player {
    #[rpc(any_peer, call_local)]
    fn fire(&self) {
        let mut bullet = self.bullet_scene.instantiate_as::<Bullet>();
	    bullet.set_global_position(self.base().get_node_as::<Node2D>("GunRotation/BulletSpawn").get_global_position());
	    bullet.set_rotation_degrees(self.base().get_node_as::<Node2D>("GunRotation").get_rotation_degrees());
	    self.base().get_tree().unwrap().get_root().unwrap().add_child(bullet);
    }
}

#[godot_api]
impl ICharacterBody2D for Player {
    fn init(base: Base<CharacterBody2D>) -> Self {
        godot_print!("Registering Player"); // Prints to the Godot console
        let gravity : f64 = Result::expect(ProjectSettings::singleton().get_setting("physics/2d/default_gravity".into()).try_to::<f64>(), "default setting in Godot");
        Self {
            speed: 300.0,
            jump_velocity: -400.0,
            gravity,
            bullet_scene: PackedScene::new_gd(),
            multiplayer_synchronizer : OnReady::node("MultiplayerSynchronizer"),
            peer_id: 1,
            username: "Player".into(),
            sync_position: Vector2::new(0., 0.),
            sync_rotation: 0.,
            base,
        }
    }

    fn ready(&mut self) {
        self.base_mut().add_to_group("Player".into());

        /*
            set multiplayer authority of each player to their correct peer id
	        basically, we need to make sure that client with peer 34 (for example) 
            should ONLY control player 34 in everyone else's game simulation
	        and that their data gets replicated everywhere
         */
        let peer_id = self.get_peer_id();
        self.multiplayer_synchronizer.set_multiplayer_authority(peer_id);
        
        // set up networked version of position and rotation
        let position = self.base().get_global_position();
        let rotation = self.base().get_rotation_degrees();
        self.set_sync_position(position);
        self.set_sync_rotation(rotation);
    }

    fn physics_process(&mut self, delta: f64) {
        let mut gun_rotation = self.base().get_node_as::<Node2D>("GunRotation");
        // only allow peer id in charge of this player to control player
        if self.multiplayer_synchronizer.get_multiplayer_authority() == self.base().get_multiplayer().unwrap().get_unique_id() {
            let mut velocity : Vector2 = self.base().get_velocity();
            // Add the gravity.
            if !self.base().is_on_floor()
            {
                velocity.y += (self.gravity * delta) as f32;
            }
            
            let input = Input::singleton();
            // Handle Jump.
            if input.is_action_just_pressed("jump".into()) && self.base().is_on_floor()
            {
                velocity.y = self.jump_velocity;
            }

            // aim gun
            gun_rotation.look_at(self.base().get_viewport().unwrap().get_mouse_position());
            
            self.set_sync_position(self.base().get_global_position());
            self.set_sync_rotation(gun_rotation.get_rotation_degrees());
            

            if input.is_action_just_pressed("Fire".into())
            {
                self.base_mut().rpc( "fire".into(), &[]);
            }
            // Get the input direction and handle the movement/deceleration.
            // As good practice, you should replace UI actions with custom gameplay actions.
            
            let direction = input.get_axis("move_left".into(), "move_right".into());
            if direction != 0.0
            {
                velocity.x = direction * self.speed;
            }
            else
            {
                velocity.x = move_toward(velocity.x.into(), 0.0, self.speed.into()) as f32;
            }

            self.base_mut().set_velocity(velocity);

            self.base_mut().move_and_slide();
        }
        else
        {
            let position = self.get_sync_position();
            let rotation = self.get_sync_rotation();
            self.base_mut().set_global_position(position);
            gun_rotation.set_global_rotation_degrees(rotation);
        }
    }
}