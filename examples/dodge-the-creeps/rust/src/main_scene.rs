use crate::hud::Hud;
use crate::mob;
use crate::player;
use godot::engine::node::InternalMode;
use godot::engine::packed_scene::GenEditState;
use godot::engine::{Marker2D, PathFollow2D, RigidBody2D, Timer};
use godot::prelude::*;
use rand::Rng as _;
use std::f64::consts::PI;

// Deriving GodotClass makes the class available to Godot
#[derive(GodotClass)]
#[class(base=Node)]
pub struct Main {
    mob_scene: Gd<PackedScene>,
    music: Option<Gd<AudioStreamPlayer>>,
    death_sound: Option<Gd<AudioStreamPlayer>>,
    score: i64,
    #[base]
    base: Base<Node>,
}

#[godot_api]
impl Main {
    #[func]
    fn game_over(&mut self) {
        let mut score_timer = self.base.get_node_as::<Timer>("ScoreTimer");
        let mut mob_timer = self.base.get_node_as::<Timer>("MobTimer");

        score_timer.stop();
        mob_timer.stop();

        let mut hud = self.base.get_node_as::<Hud>("Hud");
        hud.bind_mut().show_game_over();

        self.music().stop();
        self.death_sound().play(0.0);
    }

    #[func]
    pub fn new_game(&mut self) {
        let start_position = self.base.get_node_as::<Marker2D>("StartPosition");
        let mut player = self.base.get_node_as::<player::Player>("Player");
        let mut start_timer = self.base.get_node_as::<Timer>("StartTimer");

        self.score = 0;

        player.bind_mut().start(start_position.get_position());
        start_timer.start(0.0);

        let mut hud = self.base.get_node_as::<Hud>("Hud");
        let hud = hud.bind_mut();
        hud.update_score(self.score);
        hud.show_message("Get Ready".into());

        self.music().play(0.0);
    }

    #[func]
    fn on_start_timer_timeout(&self) {
        let mut mob_timer = self.base.get_node_as::<Timer>("MobTimer");
        let mut score_timer = self.base.get_node_as::<Timer>("ScoreTimer");
        mob_timer.start(0.0);
        score_timer.start(0.0);
    }

    #[func]
    fn on_score_timer_timeout(&mut self) {
        self.score += 1;

        let mut hud = self.base.get_node_as::<Hud>("Hud");
        hud.bind_mut().update_score(self.score);
    }

    #[func]
    fn on_mob_timer_timeout(&mut self) {
        let mut mob_spawn_location = self
            .base
            .get_node_as::<PathFollow2D>("MobPath/MobSpawnLocation");

        let mut mob_scene: Gd<RigidBody2D> = instantiate_scene(&self.mob_scene);

        let mut rng = rand::thread_rng();
        let progress = rng.gen_range(u32::MIN..u32::MAX);

        mob_spawn_location.set_progress(progress.into());
        mob_scene.set_position(mob_spawn_location.get_position());

        let mut direction = mob_spawn_location.get_rotation() + PI / 2.0;
        direction += rng.gen_range(-PI / 4.0..PI / 4.0);

        mob_scene.set_rotation(direction);

        self.base.add_child(
            mob_scene.share().upcast(),
            false,
            InternalMode::INTERNAL_MODE_DISABLED,
        );

        let mut mob = mob_scene.cast::<mob::Mob>();
        {
            // Local scope to bind `mob`
            let mut mob = mob.bind_mut();
            let range = rng.gen_range(mob.min_speed..mob.max_speed);

            mob.set_linear_velocity(Vector2::new(range, 0.0));
            let lin_vel = mob.get_linear_velocity().rotated(real::from_f64(direction));
            mob.set_linear_velocity(lin_vel);
        }

        let mut hud = self.base.get_node_as::<Hud>("Hud");
        hud.bind_mut().connect(
            "start_game".into(),
            Callable::from_object_method(mob, "on_start_game"),
            0,
        );
    }

    fn music(&mut self) -> &mut AudioStreamPlayer {
        self.music.as_deref_mut().unwrap()
    }

    fn death_sound(&mut self) -> &mut AudioStreamPlayer {
        self.death_sound.as_deref_mut().unwrap()
    }
}

#[godot_api]
impl NodeVirtual for Main {
    fn init(base: Base<Node>) -> Self {
        Main {
            mob_scene: PackedScene::new(),
            score: 0,
            base,
            music: None,
            death_sound: None,
        }
    }

    fn ready(&mut self) {
        // Note: this is downcast during load() -- completely type-safe thanks to type inference!
        // If the resource does not exist or has an incompatible type, this panics.
        // There is also try_load() if you want to check whether loading succeeded.
        self.mob_scene = load("res://Mob.tscn");
        self.music = Some(self.base.get_node_as("Music"));
        self.death_sound = Some(self.base.get_node_as("DeathSound"));
    }
}

/// Root here is needs to be the same type (or a parent type) of the node that you put in the child
///   scene as the root. For instance Spatial is used for this example.
fn instantiate_scene<Root>(scene: &PackedScene) -> Gd<Root>
where
    Root: GodotClass + Inherits<Node>,
{
    let s = scene
        .instantiate(GenEditState::GEN_EDIT_STATE_DISABLED)
        .expect("scene instantiated");

    s.cast::<Root>()
}
