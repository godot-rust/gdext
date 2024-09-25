use core::time;
use std::collections::HashMap;
use std::thread;

use godot::classes::{
    Button, Control, ENetMultiplayerPeer, IControl, LineEdit, MultiplayerApi, RichTextLabel,
};
use godot::global::Error;
use godot::obj::WithBaseField;
use godot::prelude::*;

use crate::scene_manager::SceneManager;
use crate::NetworkId;

const LOCALHOST: &str = "127.0.0.1";
const PORT: i32 = 8910;
#[derive(GodotClass, Clone)]
#[class(init)]
pub struct PlayerData {
    pub name: GString,
}

#[derive(GodotClass)]
#[class(base=Control)]
pub struct MultiplayerController {
    #[export]
    address: GString,
    port: i32,
    #[export]
    game_scene: Gd<PackedScene>,
    player_database: HashMap<NetworkId, PlayerData>,
    number_of_players_loaded: u32,
    multiplayer: OnReady<Gd<MultiplayerApi>>,
    base: Base<Control>,
}

#[godot_api]
impl MultiplayerController {
    // called when a new "peer" gets connected with the server. Both client and server get notified about this
    #[func]
    fn on_peer_connected(&self, network_id: NetworkId) {
        godot_print!("Player connected: {network_id}");
    }

    // called when a new "peer" gets disconnected with the server. Both client and server get notified about this
    #[func]
    fn on_peer_disconnected(&mut self, network_id: NetworkId) {
        godot_print!("Player Disconnected: {network_id}");
        self.player_database.remove(&network_id);
        // TODO: delete player from game when player leaves lobby
    }

    // called only from client to send information to server through send_player_information
    #[func]
    fn on_connected_to_server(&mut self) {
        godot_print!("Connected to Server!");
        // send information to server
        let username = self
            .base()
            .get_node_as::<LineEdit>("UsernameLineEdit")
            .get_text();
        let network_id = self.multiplayer.get_unique_id();
        // server always has peer id of 1
        self.base_mut().rpc_id(
            1,
            "send_player_information".into(),
            &[Variant::from(username), Variant::from(network_id)],
        );
    }

    // called only from clients
    #[func]
    fn on_connection_failed(&self) {
        godot_print!("Couldn't Connect");
    }

    // utility function that converts our player database hashmap to a string
    fn player_database_to_string(&self) -> String {
        let mut string = String::from("");
        for (network_id, data) in self.player_database.iter() {
            let username = &data.name;
            string.push_str(&format!(
                "network_id: {network_id}, username: {username} \n"
            ));
        }
        string
    }

    // this function should first be called by the player connecting to the server
    // and then, the server should call this function on all the other players to propagate the information out
    // this should result in each player having a fully populated player database containing everyone else in the lobby
    #[rpc(any_peer)]
    fn send_player_information(&mut self, name: GString, network_id: NetworkId) {
        // insert new player data with network_id if it doesn't already exist
        self.player_database
            .entry(network_id)
            .or_insert(PlayerData { name });

        // print player information onto multiplayer log
        let mut multiplayer_log = self
            .base_mut()
            .get_node_as::<RichTextLabel>("MultiplayerLog");
        multiplayer_log.set_text(self.player_database_to_string().into());

        if self.multiplayer.is_server() {
            for (id, data) in self.player_database.clone().into_iter() {
                godot_print!("sending player {id} data");
                let username = data.name;
                self.base_mut().rpc(
                    "send_player_information".into(),
                    &[Variant::from(username), Variant::from(id)],
                );
            }
        }
    }

    #[rpc(any_peer, call_local, reliable)]
    fn load_game(&mut self) {
        // start up game scene
        let mut scene = self.game_scene.instantiate_as::<SceneManager>();
        // have to put this into a block to avoid borrowing self as immutable when its already mutable
        {
            let mut base = self.base_mut();
            base.get_tree()
                .unwrap()
                .get_root()
                .unwrap()
                .add_child(scene.clone());
            // hide multiplayer menu
            base.hide();
        }

        // add players to scene
        let mut player_ids = Vec::<NetworkId>::new();
        for (&network_id, data) in &self.player_database {
            scene.bind_mut().add_player(network_id, data.name.clone());
            player_ids.push(network_id);
        }

        if self.multiplayer.is_server() {
            for id in player_ids {
                // don't call rpc on server
                if id == 1 {
                    continue;
                }
                // force other clients to also load the game up
                self.base_mut().rpc_id(id.into(), "load_game".into(), &[]);
            }
        }
    }

    // callback from scene_manager, tells the multiplayer_controller that this player has loaded in
    // Every peer will call this when they have loaded the game scene.
    #[rpc(any_peer, call_local, reliable)]
    fn load_in_player(&mut self) {
        // if server, start up game on everyone else's client
        if self.multiplayer.is_server() {
            let network_id = self.multiplayer.get_remote_sender_id();
            // only load in players that are actually in the player database
            if !self.player_database.contains_key(&network_id) {
                return;
            }
            godot_print!("loading in player {network_id}");
            self.number_of_players_loaded += 1;
            // start game once everyone is loaded in
            if self.number_of_players_loaded == self.player_database.len() as u32 {
                let mut game_scene = self
                    .base_mut()
                    .get_tree()
                    .unwrap()
                    .get_root()
                    .unwrap()
                    .get_node_as::<SceneManager>("Game");
                game_scene.bind_mut().start_game();
            }
        }
    }

    // set up server
    #[func]
    fn host_game(&mut self) {
        let mut peer = ENetMultiplayerPeer::new_gd();
        let error = peer.create_server(self.port);
        if error != Error::OK {
            godot_print!("cannot host");
            return;
        }
        peer.get_host()
            .unwrap()
            .compress(godot::classes::enet_connection::CompressionMode::RANGE_CODER);

        self.multiplayer.set_multiplayer_peer(peer);
        godot_print!("Waiting For Players!");
    }

    #[func]
    fn on_host_button_down(&mut self) {
        self.base_mut()
            .get_node_as::<Button>("JoinButton")
            .set_visible(false);
        self.host_game();
        let username = self
            .base()
            .get_node_as::<LineEdit>("UsernameLineEdit")
            .get_text();
        let network_id = self.multiplayer.get_unique_id();
        // in this instance, the host is also playing, so add their information to player_database
        self.send_player_information(username, network_id);
    }

    // if join button is clicked, set up peer as client
    #[func]
    fn on_join_button_down(&mut self) {
        self.base_mut()
            .get_node_as::<Button>("HostButton")
            .set_visible(false);
        let mut peer = ENetMultiplayerPeer::new_gd();
        self.address = self
            .base()
            .get_node_as::<LineEdit>("AddressLineEdit")
            .get_text();
        let error = peer.create_client(self.address.clone(), self.port);
        if error != Error::OK {
            godot_print!("cannot join");
            return;
        }
        peer.get_host()
            .unwrap()
            .compress(godot::classes::enet_connection::CompressionMode::RANGE_CODER);

        self.multiplayer.set_multiplayer_peer(peer);
        godot_print!("Waiting For Server...");
    }

    #[func]
    fn on_start_button_down(&mut self) {
        // have client call server to start up game
        self.base_mut().rpc_id(1, "load_game".into(), &[]);
    }
}

#[godot_api]
impl IControl for MultiplayerController {
    fn init(base: Base<Control>) -> Self {
        Self {
            address: LOCALHOST.into(),
            port: PORT,
            game_scene: PackedScene::new_gd(),
            player_database: HashMap::new(),
            number_of_players_loaded: 0,
            multiplayer: OnReady::from_base_fn(|base| base.get_multiplayer().unwrap()),
            base,
        }
    }

    fn ready(&mut self) {
        // setup ui
        let mut host_button = self.base_mut().get_node_as::<Button>("HostButton");
        host_button.connect(
            "button_down".into(),
            self.base().callable("on_host_button_down"),
        );
        let mut join_button = self.base_mut().get_node_as::<Button>("JoinButton");
        join_button.connect(
            "button_down".into(),
            self.base().callable("on_join_button_down"),
        );
        let mut start_button = self.base_mut().get_node_as::<Button>("StartButton");
        start_button.connect(
            "button_down".into(),
            self.base().callable("on_start_button_down"),
        );

        // make clone to avoid borrowing errors
        let mut multiplayer = self.multiplayer.clone();

        // setup multiplayer signal callbacks
        // currently callable/signal API is really ugly
        multiplayer.connect(
            "peer_connected".into(),
            self.base().callable("on_peer_connected"),
        );
        multiplayer.connect(
            "peer_disconnected".into(),
            self.base().callable("on_peer_disconnected"),
        );
        multiplayer.connect(
            "connected_to_server".into(),
            self.base().callable("on_connected_to_server"),
        );
        multiplayer.connect(
            "connection_failed".into(),
            self.base().callable("on_connection_failed"),
        );

        if godot::classes::Os::singleton()
            .get_cmdline_args()
            .contains(&GString::from("--server"))
        {
            self.host_game();
        }
    }
}
