use godot::engine::{
    AnimatedSprite2D, Area2D, Area2DVirtual, CollisionShape2D, Engine, PhysicsBody2D,
};
use godot::prelude::*;

#[derive(GodotClass)]
#[class(base=Area2D)]
pub struct Player {
    speed: real,
    screen_size: Vector2,

    #[base]
    base: Base<Area2D>,
}

#[godot_api]
impl Player {
    #[signal]
    fn hit();

    #[func]
    fn on_player_body_entered(&mut self, _body: Gd<PhysicsBody2D>) {
        self.base.hide();
        self.base.emit_signal("hit".into(), &[]);

        let mut collision_shape = self
            .base
            .get_node_as::<CollisionShape2D>("CollisionShape2D");

        collision_shape.set_deferred("disabled".into(), true.to_variant());
    }

    #[func]
    pub fn start(&mut self, pos: Vector2) {
        self.base.set_global_position(pos);
        self.base.show();

        let mut collision_shape = self
            .base
            .get_node_as::<CollisionShape2D>("CollisionShape2D");

        collision_shape.set_disabled(false);
    }
}

#[godot_api]
impl Area2DVirtual for Player {
    fn init(base: Base<Area2D>) -> Self {
        Player {
            speed: 400.0,
            screen_size: Vector2::new(0.0, 0.0),
            base,
        }
    }

    fn ready(&mut self) {
        let viewport = self.base.get_viewport_rect();
        self.screen_size = viewport.size;
        self.base.hide();
    }

    fn process(&mut self, delta: f64) {
        // Don't process if running in editor. This part should be removed when
        // issue is resolved: https://github.com/godot-rust/gdext/issues/70
        if Engine::singleton().is_editor_hint() {
            return;
        }

        let mut animated_sprite = self
            .base
            .get_node_as::<AnimatedSprite2D>("AnimatedSprite2D");

        let mut velocity = Vector2::new(0.0, 0.0);

        // Note: exact=false by default, in Rust we have to provide it explicitly
        let input = Input::singleton();
        if input.is_action_pressed("move_right".into(), false) {
            velocity += Vector2::RIGHT;
        }
        if input.is_action_pressed("move_left".into(), false) {
            velocity += Vector2::LEFT;
        }
        if input.is_action_pressed("move_down".into(), false) {
            velocity += Vector2::DOWN;
        }
        if input.is_action_pressed("move_up".into(), false) {
            velocity += Vector2::UP;
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

            animated_sprite.play(animation.into(), 1.0, false);
        } else {
            animated_sprite.stop();
        }

        let change = velocity * real::from_f64(delta);
        let position = self.base.get_global_position() + change;
        let position = Vector2::new(
            position.x.clamp(0.0, self.screen_size.x),
            position.y.clamp(0.0, self.screen_size.y),
        );
        self.base.set_global_position(position);
    }
}
