use godot::engine::{AnimatedSprite2D, RigidBody2D};
use godot::prelude::*;
use rand::seq::SliceRandom;

#[derive(Copy, Clone)]
enum MobType {
    Walk,
    Swim,
    Fly,
}

impl MobType {
    fn to_str(self) -> String {
        match self {
            MobType::Walk => "walk".to_string(),
            MobType::Swim => "swim".to_string(),
            MobType::Fly => "fly".to_string(),
        }
    }
}

const MOB_TYPES: [MobType; 3] = [MobType::Walk, MobType::Swim, MobType::Fly];

// ----------------------------------------------------------------------------------------------------------------------------------------------

#[derive(GodotClass)]
#[class(base=RigidBody2D)]
pub struct Mob {
    pub min_speed: f32,
    pub max_speed: f32,

    #[base]
    base: Base<RigidBody2D>,
}

#[godot_api]
impl Mob {
    #[func]
    fn on_visibility_screen_exited(&mut self) {
        self.base.queue_free();
    }

    #[func]
    fn on_start_game(&mut self) {
        self.base.queue_free();
    }
}

#[godot_api]
impl GodotExt for Mob {
    fn init(base: Base<RigidBody2D>) -> Self {
        Mob {
            min_speed: 150.0,
            max_speed: 250.0,
            base,
        }
    }

    fn ready(&mut self) {
        let mut rng = rand::thread_rng();
        let animation_name = MOB_TYPES.choose(&mut rng).unwrap().to_str();

        let mut sprite = self
            .base
            .get_node_as::<AnimatedSprite2D>("AnimatedSprite2D");
        sprite.set_animation(animation_name.as_str().into());
        sprite.set_playing(true);
    }
}
