/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::bomb::Bomb;
use crate::constants::SIGNAL_SPAWN_BOMB;
use crate::game_state::{GameSingleton, GameState};
use godot::classes::{IMultiplayerSpawner, MultiplayerSpawner};
use godot::prelude::*;

#[derive(Debug)]
pub struct BombArgs {
    position: Vector2,
    player_id: i64,
}

impl BombArgs {
    pub fn new(position: Vector2, player_id: i64) -> Self {
        Self {
            position,
            player_id,
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
        let player_id = via
            .get(1)
            .ok_or(ConvertError::new("couldn't find player id for bomb spawn!"))?
            .try_to::<i64>()?;
        Ok(Self {
            position,
            player_id,
        })
    }
}

impl ToGodot for BombArgs {
    type ToVia<'v>
        = VariantArray
    where
        Self: 'v;

    fn to_godot(&self) -> Self::ToVia<'_> {
        varray![self.position.to_variant(), self.player_id.to_variant()]
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
        self.base_mut().set_spawn_function(&spawn_bomb);
        let spawn = self.base().callable("spawn");
        GameState::singleton().connect(SIGNAL_SPAWN_BOMB, &spawn);
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
        bomb.bind_mut().from_player = args.player_id;
        bomb
    }
}
