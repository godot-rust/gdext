use crate::constants::SIGNAL_SCORE_INCREASED;
use crate::game_state::{GameSingleton, GameState};
use godot::classes::control::SizeFlags;
use godot::classes::{HBoxContainer, IHBoxContainer, Label};
use godot::global::HorizontalAlignment;
use godot::prelude::*;
use std::collections::HashMap;

pub struct PlayerScoreData {
    label: Gd<Label>,
    current_score: u32,
    pub player_name: GString,
}

impl PlayerScoreData {
    fn new(label: Gd<Label>, player_name: GString) -> Self {
        Self {
            label,
            player_name,
            current_score: 0,
        }
    }
}

#[derive(GodotClass)]
#[class(init, base=HBoxContainer)]
pub struct ScoreBoard {
    player_labels: HashMap<i32, PlayerScoreData>,
    base: Base<HBoxContainer>,
}

#[godot_api]
impl IHBoxContainer for ScoreBoard {
    fn ready(&mut self) {
        let increase_score = self.base().callable("increase_score");
        GameState::singleton().connect(SIGNAL_SCORE_INCREASED, &increase_score);
    }
}

#[godot_api]
impl ScoreBoard {
    #[func]
    fn increase_score(&mut self, for_who: i32) {
        let Some(player_score) = self.player_labels.get_mut(&for_who) else {
            return;
        };
        player_score.current_score += 1;
        player_score
            .label
            .set_text(&format! {"{} \n {}", player_score.player_name, player_score.current_score})
    }
}

impl ScoreBoard {
    pub fn add_player(&mut self, player_id: i32, new_player_name: GString) {
        let mut label = Label::new_alloc();
        label.set_horizontal_alignment(HorizontalAlignment::CENTER);
        label.set_text(&new_player_name);
        label.set_h_size_flags(SizeFlags::EXPAND_FILL);
        self.base_mut().add_child(&label);
        self.player_labels
            .insert(player_id, PlayerScoreData::new(label, new_player_name));
    }

    pub fn clear_score_and_get_highest(&mut self) -> Option<PlayerScoreData> {
        self.player_labels
            .drain()
            .fold(None, |highest, (_player_id, player_score)| {
                if let Some(high) = highest {
                    if high.current_score < player_score.current_score {
                        Some(player_score)
                    } else {
                        Some(high)
                    }
                } else {
                    Some(player_score)
                }
            })
    }
}
