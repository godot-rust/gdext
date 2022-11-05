use godot::engine::{AnimatedSprite2D, Area2D, CollisionShape2D, PhysicsBody2D};
use godot::prelude::*;

use crate::main_scene::Main;

/// The player "class"
#[derive(GodotClass)]
#[godot(base=Area2D)]
pub struct Player {
    speed: f32,
    screen_size: Vector2,
    base: Base<Area2D>,
}

#[godot_api]
impl Player {
    #[godot]
    fn _ready(&mut self) {
        let viewport = self.base.get_viewport_rect();
        self.screen_size = viewport.size;
        self.base.hide();
    }

    #[godot]
    fn _process(&mut self, delta: f32) {
        let animated_sprite = self.base.get_node_as::<AnimatedSprite>("animated_sprite");

        let input = Input::singleton();
        let mut velocity = Vector2::new(0.0, 0.0);

        // Note: exact=false by default, in Rust we have to provide it explicitly
        if Input::is_action_pressed(input, "ui_right", false) {
            velocity.x += 1.0
        }
        if Input::is_action_pressed(input, "ui_left", false) {
            velocity.x -= 1.0
        }
        if Input::is_action_pressed(input, "ui_down", false) {
            velocity.y += 1.0
        }
        if Input::is_action_pressed(input, "ui_up", false) {
            velocity.y -= 1.0
        }

        if velocity.length() > 0.0 {
            velocity = velocity.normalized() * self.speed;

            let animation;

            if velocity.x != 0.0 {
                animation = "right";

                animated_sprite.set_flip_v(false);
                animated_sprite.set_flip_h(velocity.x < 0.0)
            } else {
                animation = "up";

                animated_sprite.set_flip_v(velocity.y > 0.0)
            }

            animated_sprite.play(animation, false);
        } else {
            animated_sprite.stop();
        }

        let change = velocity * delta;
        let position = self.base.global_position() + change;
        let position = Vector2::new(
            position.x.max(0.0).min(self.screen_size.x),
            position.y.max(0.0).min(self.screen_size.y),
        );
        self.base.set_global_position(position);
    }

    #[godot]
    fn on_player_body_entered(&self, _body: Gd<PhysicsBody2D>) {
        self.base.hide();
        self.base.emit_signal("hit", &[]);

        let collision_shape = self
            .base
            .get_node_as::<CollisionShape2D>("collision_shape_2d");

        collision_shape.set_deferred("disabled", true);
    }

    #[godot]
    pub fn start(&self, pos: Vector2) {
        self.base.set_global_position(pos);
        self.base.show();

        let collision_shape = self
            .base
            .get_node_as::<CollisionShape2D>("collision_shape_2d");

        collision_shape.set_disabled(false);
    }
}

#[godot_api]
impl GodotExt for Player {
    fn init(base: Base<Area2D>) -> Self {
        Player {
            speed: 400.0,
            screen_size: Vector2::new(0.0, 0.0),
            base,
        }
    }

    // TODO once signals are supported
    // fn register_player(builder: &ClassBuilder<Self>) {
    //     builder.signal("hit").done()
    // }
}
