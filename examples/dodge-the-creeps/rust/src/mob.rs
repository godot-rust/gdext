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
    fn to_str(self) -> GodotString {
        match self {
            MobType::Walk => "walk".into(),
            MobType::Swim => "swim".into(),
            MobType::Fly => "fly".into(),
        }
    }
}

const MOB_TYPES: [MobType; 3] = [MobType::Walk, MobType::Swim, MobType::Fly];

// ----------------------------------------------------------------------------------------------------------------------------------------------

#[derive(GodotClass)]
#[class(base=RigidBody2D)]
pub struct Mob {
    pub min_speed: real,
    pub max_speed: real,

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
        sprite.set_animation(StringName::from(&animation_name));
    }
}
