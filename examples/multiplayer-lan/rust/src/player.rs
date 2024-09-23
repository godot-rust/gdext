use godot::prelude::*;
use godot::classes::{CharacterBody2D, ICharacterBody2D, ProjectSettings};
use godot::global::{move_toward};

use crate::bullet::Bullet;
#[derive(GodotClass)]
#[class(base=CharacterBody2D)]
struct Player {
    speed : f32,
    jump_velocity : f32,
    gravity : f64,
    #[export]
    bullet_scene : Gd<PackedScene>,
    base: Base<CharacterBody2D>
}

#[godot_api]
impl Player {
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
            base,
        }
    }

    fn physics_process(&mut self, delta: f64) {
        let velocity : &mut Vector2 = &mut self.base_mut().get_velocity();
        // Add the gravity.
		if !self.base().is_on_floor()
        {
			velocity.x += (self.gravity * delta) as f32;
        }
			
		//$GunRotation.look_at(get_viewport().get_mouse_position())
        let input = Input::singleton();
		// Handle Jump.
		if input.is_action_just_pressed("ui_accept".into()) && self.base().is_on_floor()
        {
			velocity.y = self.jump_velocity;
        }
		
        /* 
		syncPos = global_position
		syncRot = rotation_degrees
        */

		if input.is_action_just_pressed("Fire".into())
        {
			self.fire();
        }
		// Get the input direction and handle the movement/deceleration.
		// As good practice, you should replace UI actions with custom gameplay actions.
         
        let direction = input.get_axis("ui_left".into(), "ui_right".into());
		if direction != 0.0
        {
			velocity.x = direction * self.speed;
        }
		else
        {
			velocity.x = move_toward(velocity.x.into(), 0.0, self.speed.into()) as f32;
        }

		self.base_mut().move_and_slide();
    }
}