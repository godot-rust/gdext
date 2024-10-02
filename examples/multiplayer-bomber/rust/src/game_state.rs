/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::bomb_spawner::BombArgs;
use crate::constants::{
    SIGNAL_CONNECTED_TO_SERVER, SIGNAL_CONNECTION_FAILED, SIGNAL_CONNECTION_SUCCEEDED,
    SIGNAL_GAME_ENDED, SIGNAL_GAME_ERROR, SIGNAL_GAME_STARTED, SIGNAL_PEER_CONNECTED,
    SIGNAL_PEER_DISCONNECTED, SIGNAL_PLAYER_LIST_CHANGED, SIGNAL_SERVER_DISCONNECTED,
};
use crate::player::Player;
use crate::world::World;
use godot::classes::{ENetMultiplayerPeer, Engine, Marker2D, MultiplayerApi};
use godot::obj::{bounds, Bounds};
use godot::prelude::*;
use std::collections::HashMap;

/// In our example the GameState is being registered both as an [Autoload](https://docs.godotengine.org/en/stable/tutorials/scripting/singletons_autoload.html) and Engine Singleton
/// Autoloads and Engine Singletons works similarly to each other.
/// Engine singleton is anything that inherits an object and is accessible anywhere inside godot's runtime via Engine::singleton().get_singleton(…)
/// Autoload is anything that lives in a scene tree and is registered as a Variant via ScriptServer
/// In real-world usage you might want to declare your singleton as an Object and register it in your [MainLoop](https://docs.godotengine.org/en/stable/classes/class_mainloop.html) or [during initialization of your gdext library](https://godot-rust.github.io/book/recipes/engine-singleton.html).
#[derive(GodotClass)]
#[class(init, base=Node)]
pub struct GameState {
    #[export]
    world_scene: Option<Gd<PackedScene>>,
    #[export]
    player_scene: Option<Gd<PackedScene>>,
    #[init(val = GString::from("The Warrior"))]
    #[var]
    pub(crate) player_name: GString,
    #[var]
    game_board: Option<Gd<World>>,
    peer: Option<Gd<ENetMultiplayerPeer>>,
    players: HashMap<i32, GString>,

    #[init(val = OnReady::manual())]
    multiplayer: OnReady<Gd<MultiplayerApi>>,
    base: Base<Node>,
}

#[godot_api]
impl INode for GameState {
    fn enter_tree(&mut self) {
        Self::register(self.base().clone().upcast());
    }

    fn exit_tree(&mut self) {
        GameSingleton::exit(self);
    }

    fn ready(&mut self) {
        self.multiplayer
            .init(self.base().get_multiplayer().unwrap());

        let player_connected = self.base().callable("player_connected");
        self.multiplayer
            .connect(SIGNAL_PEER_CONNECTED, &player_connected);

        let player_disconnected = self.base().callable("player_disconnected");
        self.multiplayer
            .connect(SIGNAL_PEER_DISCONNECTED, &player_disconnected);

        let connected_ok = self.base().callable("connected_ok");
        self.multiplayer
            .connect(SIGNAL_CONNECTED_TO_SERVER, &connected_ok);

        let connected_fail = self.base().callable("connected_fail");
        self.multiplayer
            .connect(SIGNAL_CONNECTION_FAILED, &connected_fail);

        let server_disconnected = self.base().callable("server_disconnected");
        self.multiplayer
            .connect(SIGNAL_SERVER_DISCONNECTED, &server_disconnected);
    }
}

#[godot_api]
impl GameState {
    /// Default game server port. Can be any number between 1024 and 49151.
    /// Not on the list of registered or common ports as of November 2020:
    /// https://en.wikipedia.org/wiki/List_of_TCP_and_UDP_port_numbers
    #[constant]
    const DEFAULT_PORT: i32 = 10567;

    /// Max number of players
    #[constant]
    const MAX_PEERS: i32 = 12;

    #[signal]
    fn player_list_changed();

    #[signal]
    fn connection_failed();

    #[signal]
    fn connection_succeeded();

    #[signal]
    fn spawn_bomb(args: BombArgs);

    #[signal]
    fn score_increased(for_who: i64);

    #[signal]
    fn game_ended();

    #[signal]
    fn game_started();

    #[signal]
    fn game_error(error_message: GString);

    #[func]
    fn player_connected(&mut self, player_id: i32) {
        let player_name_arg = self.player_name.clone().to_variant();
        self.base_mut()
            .rpc_id(player_id as i64, "register_player", &[player_name_arg]);
    }

    #[func]
    fn player_disconnected(&mut self, player_id: i32) {
        if self.game_board.is_some() && self.multiplayer.is_server() {
            let message = GString::from(format!("Player {player_id} Disconnected"));
            self.base_mut()
                .emit_signal(SIGNAL_GAME_ERROR, &[message.to_variant()]);
            self.end_game();
            return;
        }
        self.unregister_player(player_id);
    }

    #[func]
    fn connected_ok(&mut self) {
        self.base_mut()
            .emit_signal(SIGNAL_CONNECTION_SUCCEEDED, &[]);
    }

    #[func]
    fn server_disconnected(&mut self) {
        self.base_mut()
            .emit_signal(SIGNAL_GAME_ERROR, &["server disconnected".to_variant()]);
    }

    #[rpc(any_peer)]
    fn register_player(&mut self, new_player_name: GString) {
        let id = self.multiplayer.get_remote_sender_id();
        self.players.entry(id).or_insert(new_player_name);
        self.base_mut().emit_signal(SIGNAL_PLAYER_LIST_CHANGED, &[]);
    }

    #[func]
    fn unregister_player(&mut self, player_id: i32) {
        self.players.remove(&player_id);
        self.base_mut().emit_signal(SIGNAL_PLAYER_LIST_CHANGED, &[]);
    }

    #[func]
    pub fn host_game(&mut self, new_player_name: GString) {
        self.player_name = new_player_name;
        let mut peer = ENetMultiplayerPeer::new_gd();
        peer.create_server_ex(Self::DEFAULT_PORT)
            .max_clients(Self::MAX_PEERS)
            .done();
        self.multiplayer.set_multiplayer_peer(&peer);
        self.peer = Some(peer);
        self.players
            .entry(self.multiplayer.get_unique_id())
            .or_insert(self.player_name.clone());
    }

    #[func]
    pub fn join_game(&mut self, address: GString, new_player_name: GString) {
        self.player_name = new_player_name;
        let mut peer = ENetMultiplayerPeer::new_gd();
        peer.create_client(&address, Self::DEFAULT_PORT);
        self.multiplayer.set_multiplayer_peer(&peer);
        self.peer = Some(peer);
        self.players
            .entry(self.multiplayer.get_unique_id())
            .or_insert(self.player_name.clone());
    }

    #[func]
    pub fn begin_game(&mut self) {
        if !self.multiplayer.is_server() {
            panic!("Only server can start a game!")
        }
        self.base_mut().rpc("load_world", &[]);
        let Some(world) = self.game_board.as_mut() else {
            panic!("no game board!")
        };
        for (i, (player_id, player_name)) in self.players.iter().enumerate() {
            let spawn_marker = world
                .bind_mut()
                .spawn_points
                .get_child(i as i32)
                .expect("no spawn point!")
                .cast::<Marker2D>();
            let Some(mut player) = self
                .player_scene
                .as_ref()
                .map(|ps| ps.instantiate_as::<Player>())
            else {
                panic!("Couldn't instantiate player scene!")
            };
            player.bind_mut().synced_position = spawn_marker.get_position();
            player.bind_mut().player_id = *player_id;
            player.set_name(&player_id.to_string());
            world.bind_mut().players.add_child(&player);
            player.bind_mut().label.set_text(player_name);
        }
        self.base_mut().emit_signal(SIGNAL_GAME_STARTED, &[]);
    }

    #[rpc(call_local)]
    fn load_world(&mut self) {
        let Some(mut world) = self
            .world_scene
            .as_ref()
            .map(|w| w.instantiate_as::<World>())
        else {
            panic!("World scene haven't been set!")
        };
        self.base()
            .get_tree()
            .unwrap()
            .get_root()
            .unwrap()
            .add_child(&world);

        for (player_id, player_name) in self.players.iter() {
            world
                .bind_mut()
                .score
                .bind_mut()
                .add_player(*player_id, player_name.clone());
        }
        self.game_board = Some(world);
        self.base().get_tree().unwrap().set_pause(false);
    }

    #[func]
    pub fn end_game(&mut self) {
        if let Some(mut game_board) = self.game_board.take() {
            game_board.queue_free();
        }
        self.base_mut().emit_signal(SIGNAL_GAME_ENDED, &[]);
        self.players.clear();
        if let Some(peer) = self.peer.as_mut() {
            peer.close();
        };
    }

    #[func]
    pub fn get_player_list(&self) -> Array<GString> {
        self.players.values().cloned().collect()
    }
}

impl GameState {
    pub fn get_players(&self) -> &HashMap<i32, GString> {
        &self.players
    }
}

impl GameSingleton for GameState {
    const NAME: &'static str = "GameState";
}

/// A trait that allows us to register given instance as an engine singleton
/// and use it in the same fashion as godot's ones.
pub trait GameSingleton:
    GodotClass + Bounds<Declarer = bounds::DeclUser> + Inherits<Object>
{
    const NAME: &'static str;

    fn singleton() -> Gd<Self> {
        Engine::singleton()
            .get_singleton(Self::NAME)
            .unwrap()
            .cast::<Self>()
    }

    fn register(game_system: Gd<Object>) {
        // in real world usage you might want to use this method to create your autoload.
        // In such case add trait bound 'NewAlloc'
        // and create your game system with `let game_system = Self::new_alloc();`
        Engine::singleton().register_singleton(Self::NAME, &game_system);
    }

    fn exit(&mut self) {
        Engine::singleton().unregister_singleton(Self::NAME);
    }
}
