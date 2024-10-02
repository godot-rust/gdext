use crate::constants::{ANIMATION_EXPLODE, SIGNAL_SCORE_INCREASED};
use crate::game_state::{GameSingleton, GameState};
use godot::classes::{AnimationPlayer, CharacterBody2D};
use godot::prelude::*;

#[derive(GodotClass)]
#[class(init, base=CharacterBody2D)]
pub struct Rock {
    #[init(node = "AnimationPlayer")]
    animation_player: OnReady<Gd<AnimationPlayer>>,
    base: Base<CharacterBody2D>,
}

#[godot_api]
impl Rock {
    #[rpc(any_peer, call_local)]
    fn exploded(&mut self, by_who: i64) {
        self.animation_player
            .play_ex()
            .name(ANIMATION_EXPLODE)
            .done();
        GameState::singleton().emit_signal(SIGNAL_SCORE_INCREASED, &[by_who.to_variant()]);
    }

    #[rpc(call_local)]
    fn done(&mut self) {
        self.base_mut().queue_free();
    }
}
