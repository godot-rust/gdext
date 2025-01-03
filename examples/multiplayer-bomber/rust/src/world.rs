/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::constants::SIGNAL_CHILD_EXITING_TREE;
use crate::game_state::{GameSingleton, GameState};
use crate::score::ScoreBoard;
use godot::classes::Label;
use godot::prelude::*;

#[derive(GodotClass)]
#[class(init, base=Node2D)]
pub struct World {
    #[init(node = "Rocks")]
    pub rocks: OnReady<Gd<Node2D>>,
    #[init(node = "Players")]
    pub players: OnReady<Gd<Node2D>>,
    #[init(node = "Winner")]
    pub winner_label: OnReady<Gd<Label>>,
    #[init(node = "Score")]
    pub score: OnReady<Gd<ScoreBoard>>,
    #[init(node = "SpawnPoints")]
    pub spawn_points: OnReady<Gd<Node2D>>,
    base: Base<Node2D>,
}

#[godot_api]
impl INode2D for World {
    fn ready(&mut self) {
        let on_rock_being_removed = self.base().callable("on_rock_being_removed");
        self.rocks
            .connect(SIGNAL_CHILD_EXITING_TREE, &on_rock_being_removed);
        self.winner_label.hide();
    }
}

#[godot_api]
impl World {
    #[func]
    fn on_rock_being_removed(&mut self, _rock: Gd<Node>) {
        if self.rocks.get_child_count() <= 60 {
            self.finish_game();
        }
    }

    #[func]
    fn on_exit_game_pressed(&mut self) {
        self.base_mut().rpc("end_game", &[]);
    }

    #[rpc(any_peer, call_local)]
    #[func(gd_self)]
    fn end_game(_this: Gd<Self>) {
        GameState::singleton().bind_mut().end_game();
    }
}

impl World {
    fn finish_game(&mut self) {
        self.winner_label.show();
        let Some(highest) = self.score.bind_mut().clear_score_and_get_highest() else {
            return;
        };
        self.winner_label
            .set_text(&format! {"THE WINNER IS: \n {}", highest.player_name});
    }
}
