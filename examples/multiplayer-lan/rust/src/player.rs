use godot::obj::WithBaseField;
use godot::prelude::*;
use godot::classes::{Area2D, CharacterBody2D, ICharacterBody2D, MultiplayerSynchronizer, PhysicsBody2D, ProjectSettings};
use godot::global::{move_toward};

use crate::bullet::Bullet;
use crate::NetworkId;

const MAX_HEALTH : i32 = 2;

#[derive(GodotClass)]
#[class(base=CharacterBody2D)]
pub struct Player {
    speed : f32,
    jump_velocity : f32,
    gravity : f64,
    #[var]
    health : i32,
    #[export]
    bullet_scene : Gd<PackedScene>,
    // multiplayer stuff
    multiplayer_synchronizer : OnReady<Gd<MultiplayerSynchronizer>>,
    #[export]
    pub username: GString,
    #[var]
    pub network_id: NetworkId,
    #[export]
    sync_position: Vector2,
    #[export]
    sync_rotation: f32,
    base: Base<CharacterBody2D>
}

#[godot_api]
impl Player {
    #[signal]
    fn death();

    #[rpc(any_peer, call_local)]
    fn fire(&self) {
        let mut bullet = self.bullet_scene.instantiate_as::<Bullet>();
	    bullet.set_global_position(self.base().get_node_as::<Node2D>("GunRotation/BulletSpawn").get_global_position());
	    bullet.set_rotation_degrees(self.base().get_node_as::<Node2D>("GunRotation").get_rotation_degrees());
        bullet.bind_mut().set_network_id(self.network_id);
	    self.base().get_tree().unwrap().get_root().unwrap().add_child(bullet);
    }

    // only the server/host player can call this function
    /*
    #[rpc(call_local)]
    fn set_player_position_from_server(&mut self, position: Vector2)
    {
        self.base_mut().set_global_position(position);
        self.set_sync_position(position);
    }
    */

    // Tried to make this an actual game by having a respawn system and health.
    // TODO: Figure out how to make this work
    #[func]
    fn on_player_body_entered(&mut self, body: Gd<PhysicsBody2D>) {
        if let Ok(mut bullet) = body.try_cast::<Bullet>()
        {
            // don't get hit by your own bullet
            let bullet_id = bullet.bind().get_network_id();
            if bullet_id != self.get_network_id() {
                self.base_mut().rpc("take_damage".into(), &[Variant::from(bullet_id), Variant::from(1)]);
            }
            bullet.queue_free();
        }    
    }
    
    #[rpc(any_peer, call_local)]
    fn take_damage(&mut self, attacker_id: NetworkId, damage: i32) {
        godot_print!("player {0} got hit by player {1}", self.get_network_id(), attacker_id);
        self.health -= damage;
        if self.health <= 0 
        {
            self.base_mut().emit_signal("death".into(), &[]);
            self.base_mut().queue_free();
        }    
    }

    /*
    #[func]
    fn respawn(&mut self, position: Vector2){
        self.health = MAX_HEALTH;
        self.base_mut().set_global_position(position);
        self.sync_position = position;
    }
    */
}

#[godot_api]
impl ICharacterBody2D for Player {
    fn init(base: Base<CharacterBody2D>) -> Self {
        let gravity : f64 = Result::expect(ProjectSettings::singleton().get_setting("physics/2d/default_gravity".into()).try_to::<f64>(), "default setting in Godot");
        Self {
            speed: 300.0,
            jump_velocity: -400.0,
            gravity,
            health: MAX_HEALTH,
            bullet_scene: PackedScene::new_gd(),
            multiplayer_synchronizer : OnReady::node("MultiplayerSynchronizer"),
            network_id: 1,
            username: "Player".into(),
            sync_position: Vector2::new(0., 0.),
            sync_rotation: 0.,
            base,
        }
    }

    fn ready(&mut self) {
        self.base_mut().add_to_group("Player".into());
        // set up signals
        let mut hurt_box = self.base_mut().get_node_as::<Area2D>("HurtBox");
        hurt_box.connect("body_entered".into(), self.base().callable("on_player_body_entered"));
        /*
            set multiplayer authority of each player to their correct peer id
	        basically, we need to make sure that client with peer 34 (for example) 
            should ONLY control player 34 in everyone else's game simulation
	        and that their data gets replicated everywhere
         */
        let network_id = self.get_network_id();
        self.multiplayer_synchronizer.set_multiplayer_authority(network_id);
        //godot_print!("peer id {peer_id}");
        
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
        //godot_print!("{}", self.base_mut().get_global_position());
    }
}