/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::constants::{
    ACTION_MOVE_DOWN, ACTION_MOVE_LEFT, ACTION_MOVE_RIGHT, ACTION_MOVE_UP, ACTION_SET_BOMB,
};
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
            ACTION_MOVE_LEFT,
            ACTION_MOVE_RIGHT,
            ACTION_MOVE_UP,
            ACTION_MOVE_DOWN,
        );
        self.bombing = Input::singleton().is_action_pressed(ACTION_SET_BOMB);
    }
}
