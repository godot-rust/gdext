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

#[godot_cfg(not(test))]
#[derive(GodotClass)]
#[class(base=RigidBody2D)]
pub struct Mob {
    pub min_speed: f32,
    pub max_speed: f32,

    #[cfg(not(test))]
    #[base]
    base: Base<RigidBody2D>,

    #[cfg(test)]
    pub base: tests::FakeBase,
}

#[godot_cfg(not(test))]
#[godot_api]
impl Mob {
    #[cfg(test)]
    fn new(base: tests::FakeBase) -> Self {
        Mob {
            min_speed: 150.0,
            max_speed: 250.0,
            base,
        }
    }

    #[func]
    fn on_visibility_screen_exited(&mut self) {
        self.base.queue_free();
    }

    #[func]
    fn on_start_game(&mut self) {
        self.base.queue_free();
    }

    fn ready(&mut self) {
        let mut rng = rand::thread_rng();
        let animation_name = MOB_TYPES.choose(&mut rng).unwrap().to_str();

        // let mut sprite = self.get_animated_sprite();
        let mut sprite = self
            .base
            .get_node_as::<AnimatedSprite2D>("AnimatedSprite2D");
        sprite.set_animation(animation_name.as_str().into());
        sprite.set_playing(true);
    }
}

#[godot_cfg(not(test))]
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
        Mob::ready(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    pub struct FakeAnimatedSprite2D {
        pub animate_is_set: bool,
        pub playing: bool,
        pub was_freed: bool,
    }

    impl FakeAnimatedSprite2D {
        pub fn set_animation(&mut self, _animation: &str) {
            self.animate_is_set = true;
        }

        pub fn set_playing(&mut self, playing: bool) {
            self.playing = playing;
        }

        pub fn queue_free(&mut self) {
            self.was_freed = true;
        }
    }

    #[derive(Default)]
    pub struct FakeBase {
        pub animated_sprite_2d: FakeAnimatedSprite2D,
        pub was_freed: bool,
    }

    impl<'a> FakeBase {
        pub fn queue_free(&mut self) {
            self.was_freed = true;
        }

        pub fn get_node_as<T>(& mut self, _name: &str) -> &mut FakeAnimatedSprite2D {
            &mut self.animated_sprite_2d
        }
    }

    #[test]
    fn test_ready() {
        let mut mob = Mob::new(FakeBase::default());
        mob.ready();

        assert!(mob.base.animated_sprite_2d.animate_is_set);
        assert!(mob.base.animated_sprite_2d.playing);
        assert!(!mob.base.animated_sprite_2d.was_freed);
    }
}
