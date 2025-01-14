/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::constants::{
    FLAG_CONNECT_DEFERRED, SIGNAL_CONNECTION_FAILED, SIGNAL_CONNECTION_SUCCEEDED,
    SIGNAL_GAME_ENDED, SIGNAL_GAME_ERROR, SIGNAL_GAME_STARTED, SIGNAL_PLAYER_LIST_CHANGED,
};
use crate::game_state::{GameSingleton, GameState};
use godot::classes::{
    AcceptDialog, Button, Control, IControl, ItemList, Label, LineEdit, MultiplayerApi, Os, Panel,
};
use godot::prelude::*;

#[derive(GodotClass)]
#[class(init, base=Control)]
pub struct Lobby {
    #[init(node = "Connect/Name")]
    player_name: OnReady<Gd<LineEdit>>,
    #[init(node = "Connect/ErrorLabel")]
    error_label: OnReady<Gd<Label>>,
    #[init(node = "Connect/Host")]
    host_button: OnReady<Gd<Button>>,
    #[init(node = "Connect/Join")]
    join_button: OnReady<Gd<Button>>,
    #[init(node = "Connect/IPAddress")]
    ip_address: OnReady<Gd<LineEdit>>,
    #[init(node = "Connect")]
    connect_panel: OnReady<Gd<Panel>>,
    #[init(node = "Players")]
    players_panel: OnReady<Gd<Panel>>,
    #[init(node = "Players/List")]
    players_list: OnReady<Gd<ItemList>>,
    #[init(node = "Players/Start")]
    start_button: OnReady<Gd<Button>>,
    #[init(node = "ErrorDialog")]
    error_dialog: OnReady<Gd<AcceptDialog>>,
    #[init(val = OnReady::manual())]
    multiplayer: OnReady<Gd<MultiplayerApi>>,
    base: Base<Control>,
}

#[godot_api]
impl IControl for Lobby {
    fn ready(&mut self) {
        self.multiplayer
            .init(self.base().get_multiplayer().unwrap());
        let on_connection_failed = self.base().callable("on_connection_failed");
        GameState::singleton().connect(SIGNAL_CONNECTION_FAILED, &on_connection_failed);
        let on_connection_success = self.base().callable("on_connection_success");
        GameState::singleton().connect(SIGNAL_CONNECTION_SUCCEEDED, &on_connection_success);
        let refresh_lobby = self.base().callable("refresh_lobby");
        GameState::singleton().connect(SIGNAL_PLAYER_LIST_CHANGED, &refresh_lobby);
        let on_game_ended = self.base().callable("on_game_ended");
        GameState::singleton().connect(SIGNAL_GAME_ENDED, &on_game_ended);
        let on_game_error = self.base().callable("on_game_error");
        GameState::singleton().connect(SIGNAL_GAME_ERROR, &on_game_error);
        let on_game_started = self.base().callable("on_game_started");
        GameState::singleton()
            .connect_ex(SIGNAL_GAME_STARTED, &on_game_started)
            .flags(FLAG_CONNECT_DEFERRED)
            .done();
    }
}

#[godot_api]
impl Lobby {
    #[func]
    fn on_host_pressed(&mut self) {
        if self.player_name.get_text().is_empty() {
            self.error_label.set_text("Invalid name!");
            return;
        }

        self.connect_panel.hide();
        self.players_panel.show();
        self.error_label.set_text(&GString::default());
        let player_name = self.player_name.get_text();
        GameState::singleton().bind_mut().host_game(player_name);
        self.refresh_lobby();
    }

    #[func]
    fn on_join_pressed(&mut self) {
        if self.player_name.get_text().is_empty() {
            self.error_label.set_text("Invalid name!");
            return;
        }

        let ip = self.ip_address.get_text();
        self.host_button.set_disabled(true);
        self.join_button.set_disabled(true);
        self.error_label.set_text(&GString::default());

        let player_name = self.player_name.get_text();
        GameState::singleton().bind_mut().join_game(ip, player_name);
    }

    #[func]
    fn on_connection_success(&mut self) {
        self.connect_panel.hide();
        self.players_panel.show();
    }

    #[func]
    fn on_connection_failed(&mut self) {
        self.error_label.set_text("Connection failed.");
        self.host_button.set_disabled(false);
        self.join_button.set_disabled(false);
    }

    #[func]
    fn on_game_ended(&mut self) {
        self.base_mut().show();
        self.connect_panel.show();
        self.players_panel.hide();
        self.host_button.set_disabled(false);
        self.join_button.set_disabled(false);
    }

    #[func]
    fn on_game_error(&mut self, error: GString) {
        self.error_dialog.set_text(&error);
        self.error_dialog.popup_centered();
        self.host_button.set_disabled(false);
        self.join_button.set_disabled(false);
    }

    #[func]
    fn refresh_lobby(&mut self) {
        // add current player at the top of the players list
        self.players_list.clear();
        self.players_list.add_item(&GString::from(
            format! {"{} (You)", GameState::singleton().bind().player_name},
        ));

        let game_state = GameState::singleton();
        let binding = game_state.bind();
        let other_players = binding
            .get_players()
            .iter()
            .filter(|(player_id, _)| **player_id != self.multiplayer.get_unique_id());

        for (_, player) in other_players {
            self.players_list.add_item(player);
        }
        let is_server = self.base().get_multiplayer().unwrap().is_server();
        self.start_button.set_disabled(!is_server);
    }

    #[func]
    fn on_start_pressed(&self) {
        GameState::singleton().bind_mut().begin_game();
    }

    #[func]
    fn on_find_public_ip_pressed(&self) {
        Os::singleton().shell_open("https://icanhazip.com/");
    }

    #[func]
    fn on_game_started(&mut self) {
        self.base_mut().hide();
    }
}
