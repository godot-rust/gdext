use crate::hud::Hud;
use crate::mob;
use crate::player;
use godot::engine::packed_scene::GenEditState;
use godot::engine::{Marker2D, PathFollow2D, RigidBody2D, Timer};
use godot::prelude::*;
use rand::*;
use std::f64::consts::PI;

#[derive(GodotClass)]
#[godot(base=Node)]
pub struct Main {
    mob: Gd<PackedScene>,
    score: i64,
    base: Base<Node>,
}

#[godot_api]
impl Main {
    #[godot]
    fn game_over(&self) {
        let mut score_timer = self.base.get_node_as::<Timer>("score_timer");
        let mut mob_timer = self.base.get_node_as::<Timer>("mob_timer");

        score_timer.stop();
        mob_timer.stop();

        let hud = self.base.get_node_as::<Hud>("hud");
        hud.map(|x, o| x.show_game_over(&o))
            .ok()
            .unwrap_or_else(|| godot_print!("Unable to get hud"));
    }

    #[godot]
    fn new_game(&mut self) {
        let start_position = self.base.get_node_as::<Marker2D>("start_position");
        let player = self.base.get_node_as::<player::Player>("player");
        let start_timer = self.base.get_node_as::<Timer>("start_timer");

        self.score = 0;

        player
            .map(|x, o| x.start(&o, start_position.position()))
            .ok()
            .unwrap_or_else(|| godot_print!("Unable to get player"));

        start_timer.start(0.0);

        let hud = self.base.get_node_as_instance::<Hud>("hud");
        hud.map(|x, o| {
            x.update_score(&o, self.score);
            x.show_message(&o, "Get Ready".into());
        })
        .ok()
        .unwrap_or_else(|| godot_print!("Unable to get hud"));
    }

    #[godot]
    fn on_start_timer_timeout(&self) {
        let mob_timer = self.base.get_node_as::<Timer>("mob_timer");
        let score_timer = self.base.get_node_as::<Timer>("score_timer");
        mob_timer.start(0.0);
        score_timer.start(0.0);
    }

    #[godot]
    fn on_score_timer_timeout(&mut self) {
        self.score += 1;

        let hud = self.base.get_node_as_instance::<Hud>("hud");
        hud.map(|x, o| x.update_score(&o, self.score))
            .ok()
            .unwrap_or_else(|| godot_print!("Unable to get hud"));
    }

    #[godot]
    fn on_mob_timer_timeout(&self) {
        let mob_spawn_location = self
            .base
            .get_node_as::<PathFollow2D>("mob_path/mob_spawn_locations");

        let mob_scene: Gd<RigidBody2D> = instance_scene(&self.mob);

        let mut rng = rand::thread_rng();
        let offset = rng.gen_range(u32::MIN..u32::MAX);

        mob_spawn_location.set_offset(offset.into());

        let mut direction = mob_spawn_location.rotation() + PI / 2.0;

        mob_scene.set_position(mob_spawn_location.position());

        direction += rng.gen_range(-PI / 4.0..PI / 4.0);
        mob_scene.set_rotation(direction);
        let d = direction as f32;

        let mob_scene = mob_scene.into_shared();
        self.base.add_child(mob_scene, false);

        {
            // Local scope to bind `mob`
            let mob = mob_scene.cast::<mob::Mob>();
            let mob = mob.bind_mut();
            mob.set_linear_velocity(Vector2::new(
                rng.gen_range(mob.min_speed..mob.max_speed),
                0.0,
            ));
            mob.set_linear_velocity(mob.linear_velocity().rotated(d));
        }

        let hud = self.base.get_node_as_instance::<Hud>("hud");

        hud.bind_mut().connect(
            "start_game",
            Callable::from_object_method(mob, "on_start_game"),
            0,
        );
    }
}

#[godot_api]
impl GodotExt for Main {
    fn init(base: Base<Node>) -> Self {
        Main {
            mob: PackedScene::new(),
            score: 0,
            base,
        }
    }
}

/// Root here is needs to be the same type (or a parent type) of the node that you put in the child
///   scene as the root. For instance Spatial is used for this example.
fn instance_scene<Root>(scene: &Gd<PackedScene>) -> Gd<Root>
where
    Root: GodotClass + Inherits<Node>,
{
    let instance = scene.instance(GenEditState::GEN_EDIT_STATE_DISABLED);

    instance.cast::<Root>()
}
