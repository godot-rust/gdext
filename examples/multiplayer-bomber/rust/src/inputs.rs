use godot::prelude::*;

#[derive(GodotClass)]
#[class(init, base=Node)]
pub struct PlayerInputs {
    #[export]
    pub bombing: bool,
    #[export]
    pub(crate) motion: Vector2,
}

impl PlayerInputs {
    pub fn update(&mut self) {
        self.motion = Input::singleton().get_vector(
            "move_left".into(),
            "move_right".into(),
            "move_up".into(),
            "move_down".into(),
        );
        self.bombing = Input::singleton().is_action_pressed("set_bomb".into());
    }
}
