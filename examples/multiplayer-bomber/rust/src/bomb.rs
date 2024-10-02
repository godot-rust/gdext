/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use godot::classes::{Area2D, PhysicsRayQueryParameters2D, TileMap};
use godot::prelude::*;

#[derive(GodotClass)]
#[class(init, base=Area2D)]
pub struct Bomb {
    #[var]
    pub from_player: i64,
    base: Base<Area2D>,
}

#[godot_api]
impl Bomb {
    /// Called from the animation.
    #[func]
    fn explode(&self) {
        // Explode only on authority.
        if !self.base().is_multiplayer_authority() {
            return;
        }
        let Some(Some(mut space_state)) = self
            .base()
            .get_world_2d()
            .map(|mut w| w.get_direct_space_state())
        else {
            panic!("couldn't access World2D from Bomb!")
        };
        for mut collider in self.base().get_overlapping_bodies().iter_shared() {
            if !collider.has_method("exploded") {
                continue;
            }
            // check if there is any solid wall between bomb and the object
            let Some(mut query) = PhysicsRayQueryParameters2D::create(
                self.base().get_global_position(),
                collider.get_global_position(),
            ) else {
                panic!("couldn't create PhysicsRayQueryParameters!")
            };
            query.set_hit_from_inside(true);
            let result = space_state.intersect_ray(&query);
            let is_wall_between = result
                .get("collider")
                .map(|c| c.try_to::<Gd<TileMap>>().is_ok())
                .unwrap_or(false);
            if is_wall_between {
                continue;
            }

            collider.rpc("exploded", &[self.from_player.to_variant()]);
        }
    }

    #[func]
    fn done(&mut self) {
        if self.base().is_multiplayer_authority() {
            self.base_mut().queue_free();
        }
    }
}
