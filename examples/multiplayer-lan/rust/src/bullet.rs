use godot::classes::{CharacterBody2D, ICharacterBody2D, MultiplayerPeer, SceneTreeTimer};
use godot::prelude::*;

use crate::NetworkId;

const SPEED: f32 = 500.0;
const LIFETIME: f64 = 2.0;

#[derive(GodotClass)]
#[class(base=CharacterBody2D)]
pub struct Bullet {
    direction: Vector2,
    // who shot the bullet
    #[var]
    pub network_id: NetworkId,
    // dont want the bullets to live forever
    timer: OnReady<Gd<SceneTreeTimer>>,
    base: Base<CharacterBody2D>,
}

#[godot_api]
impl ICharacterBody2D for Bullet {
    fn init(base: Base<CharacterBody2D>) -> Self {
        Self {
            direction: Vector2::new(1., 0.),
            network_id: MultiplayerPeer::TARGET_PEER_SERVER,
            timer: OnReady::from_base_fn(|base| {
                base.get_tree().unwrap().create_timer(LIFETIME).unwrap()
            }),
            base,
        }
    }

    fn ready(&mut self) {
        self.direction = self.direction.rotated(self.base().get_rotation());
        let velocity = self.direction * SPEED;
        self.base_mut().set_velocity(velocity);
    }

    fn physics_process(&mut self, _delta: f64) {
        // delete bullet once LIFETIME seconds have passed
        if self.timer.get_time_left() <= 0. {
            self.base_mut().queue_free();
        }

        self.base_mut().move_and_slide();
    }
}
