use godot::prelude::*;
use godot::classes::{CharacterBody2D, ICharacterBody2D, ProjectSettings};

use crate::NetworkId;

const SPEED : f32 = 500.0;
const LIFETIME : f64 = 2.0;

#[derive(GodotClass)]
#[class(base=CharacterBody2D)]
pub struct Bullet {
    gravity : f64,
    direction : Vector2,
    // who shot the bullet
    #[var]
    pub attacker_id : NetworkId,
    // dont want the bullets to live forever
    time_left : f64,
    base: Base<CharacterBody2D>
}

#[godot_api]
impl ICharacterBody2D for Bullet {
    fn init(base: Base<CharacterBody2D>) -> Self {
        let gravity : f64 = Result::expect(ProjectSettings::singleton().get_setting("physics/2d/default_gravity".into()).try_to::<f64>(), "default setting in Godot");

        Self {
            gravity,
            direction: Vector2::new(1., 0.),
            attacker_id: 1,
            time_left: LIFETIME,
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
        // delete bullet once LIFETIME seconds have passed
        self.time_left -= delta;
        if self.time_left <= 0.0 {
            self.base_mut().queue_free();
        }
        // have bullet fall down while flying
        if !self.base().is_on_floor() { 
            self.base_mut().get_velocity().x += (self.gravity * 1. * delta) as f32;
        }   

		self.base_mut().move_and_slide();
    }
}