use godot::prelude::*;
use godot::classes::{CharacterBody2D, ICharacterBody2D, ProjectSettings};

const SPEED : f32 = 500.0;

#[derive(GodotClass)]
#[class(base=CharacterBody2D)]
pub struct Bullet {
    gravity : f64,
    direction : Vector2,
    base: Base<CharacterBody2D>
}

#[godot_api]
impl ICharacterBody2D for Bullet {
    fn init(base: Base<CharacterBody2D>) -> Self {
        godot_print!("Registering Player"); // Prints to the Godot console
        let gravity : f64 = Result::expect(ProjectSettings::singleton().get_setting("physics/2d/default_gravity".into()).try_to::<f64>(), "default setting in Godot");

        Self {
            gravity,
            direction: Vector2::new(1., 0.),
            base,
        }
    }

    fn ready(&mut self)
    {
        self.direction = self.direction.rotated(self.base().get_rotation());
        let velocity = self.direction * SPEED;
        self.base_mut().set_velocity(velocity);
    }

    fn physics_process(&mut self, delta: f64) {
        // have bullet fall down while flying
        if !self.base().is_on_floor() { 
            self.base_mut().get_velocity().x += (self.gravity * 1. * delta) as f32;
        }   

		self.base_mut().move_and_slide();
    }
}