use crate::bomb::Bomb;
use crate::game_state::{GameSingleton, GameState};
use godot::classes::{IMultiplayerSpawner, MultiplayerSpawner};
use godot::prelude::*;

#[derive(Debug)]
pub struct BombArgs {
    position: Vector2,
    player_idx: i64,
}

impl BombArgs {
    pub fn new(position: Vector2, player_idx: i64) -> Self {
        Self {
            position,
            player_idx,
        }
    }
}

impl GodotConvert for BombArgs {
    type Via = VariantArray;
}

impl FromGodot for BombArgs {
    fn try_from_godot(via: Self::Via) -> Result<Self, ConvertError> {
        let position = via
            .get(0)
            .ok_or(ConvertError::new("couldn't find position for bomb spawn!"))?
            .try_to::<Vector2>()?;
        let player_idx = via
            .get(1)
            .ok_or(ConvertError::new(
                "couldn't find player idx for bomb spawn!",
            ))?
            .try_to::<i64>()?;
        Ok(Self {
            position,
            player_idx,
        })
    }
}

impl ToGodot for BombArgs {
    type ToVia<'v> = VariantArray where Self: 'v;

    fn to_godot(&self) -> Self::ToVia<'_> {
        varray![self.position.to_variant(), self.player_idx.to_variant()]
    }
}

#[derive(GodotClass)]
#[class(init, base=MultiplayerSpawner)]
pub struct BombSpawner {
    #[export]
    bomb_scene: Option<Gd<PackedScene>>,
    base: Base<MultiplayerSpawner>,
}

#[godot_api]
impl IMultiplayerSpawner for BombSpawner {
    fn ready(&mut self) {
        let spawn_bomb = self.base().callable("spawn_bomb");
        self.base_mut().set_spawn_function(spawn_bomb);
        let spawn = self.base().callable("spawn");
        GameState::singleton().connect("spawn_bomb".into(), spawn);
    }
}

#[godot_api]
impl BombSpawner {
    #[func]
    fn spawn_bomb(&self, args: BombArgs) -> Gd<Bomb> {
        let Some(mut bomb) = self
            .bomb_scene
            .as_ref()
            .map(|scene| scene.instantiate_as::<Bomb>())
        else {
            panic!("couldn't instantiate bomb scene!")
        };
        bomb.set_position(args.position);
        bomb.bind_mut().from_player = args.player_idx;
        bomb
    }
}
