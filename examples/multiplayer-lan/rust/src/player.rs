use godot::classes::{
    AnimationPlayer, CharacterBody2D, ICharacterBody2D, Label, MultiplayerApi, MultiplayerPeer, MultiplayerSpawner, MultiplayerSynchronizer
};
use godot::obj::WithBaseField;
use godot::prelude::*;

use crate::bomb_spawner::BombSpawner;
use crate::player_controls::PlayerControls;
use crate::NetworkId;

const MOTION_SPEED: f32 = 90.0;
const BOMB_RATE : f64 = 0.5;

#[derive(GodotClass)]
#[class(base=CharacterBody2D)]
pub struct Player {
    #[export]
    synced_position: Vector2,
    #[export]
    stunned: bool,
    inputs: OnReady<Gd<PlayerControls>>,
    #[var]
    last_bomb_time: f64,
    bomb_spawner: OnReady<Gd<BombSpawner>>,
    #[var]
    current_anim: StringName,
    animation_player: OnReady<Gd<AnimationPlayer>>,
    multiplayer: OnReady<Gd<MultiplayerApi>>,
    network_id: NetworkId,
    base: Base<CharacterBody2D>,
}

#[godot_api]
impl Player {
    #[func]
    fn set_player_name(&mut self, value: GString) {
        self.base_mut().get_node_as::<Label>("label").set_text(value);
    }

    #[rpc(call_local)]
    fn exploded(&mut self, _by_who: NetworkId) {
        if self.stunned {
            return;
        }
        self.stunned = true;
        self.animation_player.play_ex().name("stunned".into());
    }
}

#[godot_api]
impl ICharacterBody2D for Player {
    fn init(base: Base<CharacterBody2D>) -> Self {
        Self {
            synced_position: Vector2::new(0., 0.),
            stunned: false,
            inputs: OnReady::from_base_fn(|base| base.get_node_as::<PlayerControls>("Inputs")),
            last_bomb_time: BOMB_RATE,
            bomb_spawner: OnReady::from_base_fn(|base| base.get_node_as::<BombSpawner>("../../BombSpawner")),
            current_anim: StringName::default(),
            animation_player: OnReady::from_base_fn(|base| base.get_node_as::<AnimationPlayer>("anim")),
            multiplayer: OnReady::from_base_fn(|base| base.get_multiplayer().unwrap()),
            network_id: MultiplayerPeer::TARGET_PEER_SERVER,
            base,
        }
    }

    fn ready(&mut self) {
        self.stunned = false;
        let synced_position = self.synced_position;
        self.base_mut().set_position(synced_position);
        self.network_id = self.base().get_name().to_string().parse::<i32>().unwrap();
        self.base().get_node_as::<MultiplayerSynchronizer>("Inputs/InputsSync").set_multiplayer_authority(self.network_id);
    }

    fn physics_process(&mut self, delta: f64) {
        if self.multiplayer.get_multiplayer_peer() == None || self.multiplayer.get_unique_id() == self.network_id {
            // The client which this player represent will update the controls state, and notify it to everyone.
            self.inputs.bind_mut().update();
        }

        if self.multiplayer.get_multiplayer_peer() == None || self.base().is_multiplayer_authority() {
            // The server updates the position that will be notified to the clients.
            self.synced_position = self.base().get_position();
            // And increase the bomb cooldown spawning one if the client wants to.
            self.last_bomb_time += delta;
            if !self.stunned && self.base().is_multiplayer_authority() && self.inputs.bind().get_bombing() && self.last_bomb_time >= BOMB_RATE {
                self.last_bomb_time = 0.0;
                let data = varray![self.base().get_position(), self.network_id];
                let _ = self.bomb_spawner.spawn_ex().data(Variant::from(data));
                godot_print!("bomibg");
            }
        } else {
            // The client simply updates the position to the last known one.
            let synced_position = self.synced_position;
            self.base_mut().set_position(synced_position);
        }
        
        if !self.stunned {
            let velocity = self.inputs.bind().get_motion() * MOTION_SPEED;
            self.base_mut().set_velocity(velocity);
            self.base_mut().move_and_slide();
        }

        // Also update the animation based on the last known player input state
        let mut new_anim = "standing";

        let motion = self.inputs.bind().get_motion();

        if motion.y < 0. {
            new_anim = "walk_up";
        }
        else if motion.y > 0. {
            new_anim = "walk_down";
        }
        else if motion.x < 0. {
            new_anim = "walk_left";
        }
        else if motion.x > 0. {
            new_anim = "walk_right";
        }

        if self.stunned {
            new_anim = "stunned";
        }

        if new_anim != self.current_anim.to_string() {
            self.current_anim = new_anim.into();
            self.animation_player.play_ex().name(self.current_anim.clone());
        }
    }
}