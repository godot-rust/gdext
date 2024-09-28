use godot::classes::{Node, INode};
use godot::prelude::*;

#[derive(GodotClass)]
#[class(base=Node)]
pub struct PlayerControls {
    #[var(set=set_motion, get=get_motion)]
    #[export]
    motion: Vector2,
    #[export]
    bombing: bool,
    base: Base<Node>,
}

#[godot_api]
impl PlayerControls {
    #[func]
    pub fn set_motion(&mut self, value: Vector2) {
        self.motion = value.clamp(Vector2::new(-1., -1.), Vector2::new(1., 1.));
    }

    #[func]
    pub fn get_motion(&self) -> Vector2 {
        self.motion
    }

    #[func]
    pub fn update(&mut self) {
        let mut m = Vector2::new(0., 0.);
        let input = Input::singleton();
        if input.is_action_pressed("move_left".into()) {
		    m.x += -1.;
        }
        if input.is_action_pressed("move_right".into()){
            m.x += 1.;
        }
        if input.is_action_pressed("move_up".into()){
            m.y += -1.;
        }
        if input.is_action_pressed("move_down".into()){
            m.y += 1.;
        }
        self.motion = m;
        self.bombing = input.is_action_pressed("set_bomb".into());
    }
}

#[godot_api]
impl INode for PlayerControls {
    fn init(base: Base<Node>) -> Self {
        Self {
            motion: Vector2::new(0., 0.),
            bombing: false,
            base,
        }
    }
}
