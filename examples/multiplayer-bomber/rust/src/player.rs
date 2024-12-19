use crate::bomb_spawner::BombArgs;
use crate::game_state::{GameSingleton, GameState};
use crate::inputs::PlayerInputs;
use godot::classes::{
    AnimationPlayer, CharacterBody2D, ICharacterBody2D, Label, MultiplayerApi,
    MultiplayerSynchronizer,
};
use godot::prelude::*;
use std::cmp::Ordering;

#[derive(GodotClass)]
#[class(init, base=CharacterBody2D)]
pub struct Player {
    #[init(node = "AnimationPlayer")]
    animation_player: OnReady<Gd<AnimationPlayer>>,
    #[init(node = "Inputs")]
    inputs: OnReady<Gd<PlayerInputs>>,
    #[init(node = "Inputs/InputsSync")]
    inputs_sync: OnReady<Gd<MultiplayerSynchronizer>>,
    #[init(node = "label")]
    pub label: OnReady<Gd<Label>>,

    #[export]
    pub stunned: bool,
    #[export]
    pub synced_position: Vector2,
    #[export]
    pub player_id: i32,
    last_bomb_time: f64,
    #[init(val = OnReady::manual())]
    multiplayer: OnReady<Gd<MultiplayerApi>>,
    base: Base<CharacterBody2D>,
}

#[godot_api]
impl ICharacterBody2D for Player {
    fn ready(&mut self) {
        let pos = self.synced_position;
        self.base_mut().set_position(pos);
        self.multiplayer
            .init(self.base().get_multiplayer().unwrap());
        self.inputs_sync.set_multiplayer_authority(self.player_id);
    }

    fn physics_process(&mut self, delta: f64) {
        if self.multiplayer.get_multiplayer_peer().is_none()
            || self.multiplayer.get_unique_id() == self.player_id
        {
            self.inputs.bind_mut().update();
        }

        if self.multiplayer.get_multiplayer_peer().is_none()
            || self.base().is_multiplayer_authority()
        {
            self.synced_position = self.base().get_position();
            self.last_bomb_time += delta;
            if !self.stunned
                && self.base().is_multiplayer_authority()
                && self.inputs.bind().bombing
                && self.last_bomb_time >= Self::BOMB_RATE
            {
                self.last_bomb_time = 0.0;
                let bomb_args =
                    BombArgs::new(self.base().get_position(), self.player_id as i64).to_variant();
                GameState::singleton().emit_signal("spawn_bomb".into(), &[bomb_args]);
            }
        } else {
            let pos = self.synced_position;
            self.base_mut().set_position(pos);
        }

        if !self.stunned {
            let v = self.inputs.bind_mut().motion * Self::MOTION_SPEED;
            self.base_mut().set_velocity(v);
            self.base_mut().move_and_slide();
        }

        self.update_animation();
    }
}

impl Player {
    const BOMB_RATE: f64 = 0.5;
    const MOTION_SPEED: f32 = 90.0;

    fn get_current_animation(&self) -> Option<StringName> {
        let new = if self.stunned {
            GString::from("stunned")
        } else {
            let motion = self.inputs.bind().motion;
            match (motion.x.partial_cmp(&0.), motion.y.partial_cmp(&0.)) {
                (_, Some(Ordering::Greater)) => GString::from("walk_down"),
                (_, Some(Ordering::Less)) => GString::from("walk_up"),
                (Some(Ordering::Greater), _) => GString::from("walk_right"),
                (Some(Ordering::Less), _) => GString::from("walk_left"),
                _ => GString::from("standing"),
            }
        };

        let current = self.animation_player.get_current_animation();
        let has_animation_changed =
            (new != current) || current.is_empty() || !self.animation_player.is_playing();
        if has_animation_changed {
            return Some(StringName::from(new));
        }
        None
    }

    fn update_animation(&mut self) {
        if let Some(new_anim) = self.get_current_animation() {
            self.animation_player.play_ex().name(new_anim).done();
        }
    }
}

#[godot_api]
impl Player {
    #[rpc(call_local)]
    pub fn exploded(&mut self, _by_who: i64) {
        if self.stunned {
            return;
        }
        self.stunned = true;
        self.animation_player
            .play_ex()
            .name("stunned".into())
            .done();
    }
}
