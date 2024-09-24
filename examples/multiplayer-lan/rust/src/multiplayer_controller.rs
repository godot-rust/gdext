use core::time;
use std::collections::HashMap;
use std::thread;

use godot::classes::{Button, Control, ENetMultiplayerPeer, IControl, LineEdit, RichTextLabel};
use godot::global::Error;
use godot::obj::WithBaseField;
use godot::prelude::*;

use crate::scene_manager::SceneManager;
use crate::{NetworkId, PlayerData};

const LOCALHOST: &str = "127.0.0.1";
const PORT: i32 = 8910;

#[derive(GodotClass)]
#[class(base=Control)]
pub struct MultiplayerController {
    #[export]
    address: GString,
    port: i32,
    #[export]
    game_scene: Gd<PackedScene>,
    player_database: HashMap<NetworkId, PlayerData>,
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
        if let Some ((_id, data)) = &mut self.player_database.remove_entry(&network_id) {
            data.delete_player_ref();
        }
    }

    // called only from clients
    #[func]
    fn on_connected_to_server(&mut self) {
        godot_print!("Connected to Server!");
        // send information to server
        let mut multiplayer = self.base().get_multiplayer().unwrap();
        let username = self.base().get_node_as::<LineEdit>("UsernameLineEdit").get_text();
        let network_id = multiplayer.get_unique_id();
        // server always has peer id of 1
        self.base_mut().rpc_id(1, "send_player_information".into(), &[Variant::from(username), Variant::from(network_id)]);
    }

    // called only from clients
    #[func]
    fn on_connection_failed(&self) {
        godot_print!("Couldn't Connect");
    }

    fn player_database_to_string(&self) -> String 
    {
        let mut string = String::from("");
        for (network_id, data) in self.player_database.iter() {
            let username = &data.name;
            let score = data.score;
            string.push_str(&format!("network_id: {network_id}, username: {username}, score: {score} \n"));
        }
        string
    }

    // this function should first be called by the player connecting to the server
    // and then, the server should call this function on all the other players to propagate the information out
    #[rpc(any_peer)]
    fn send_player_information(&mut self, name: GString, network_id: NetworkId) {
        let mut multiplayer = self.base().get_multiplayer().unwrap();
        self.player_database.entry(network_id).or_insert(PlayerData{name, network_id, score: 0, player_ref: None});
        // print player information onto multiplayer log
        let mut multiplayer_log = self
            .base_mut()
            .get_node_as::<RichTextLabel>("MultiplayerLog");
        multiplayer_log.set_text(self.player_database_to_string().into());

        if multiplayer.is_server() {
            for (id, data) in self.player_database.clone().into_iter() {
                godot_print!("sending player {id} data");
                let username = data.name;
                self.base_mut().rpc("send_player_information".into(), &[Variant::from(username), Variant::from(id)]);
            }
        }
    }

    #[rpc(any_peer, call_local)]
    fn start_game(&mut self) {
        // start up game scene
        let scene = self.game_scene.instantiate().unwrap();
        // give game scene our player database
        if let Ok(scene) = &mut scene.clone().try_cast::<SceneManager>() {
            scene.bind_mut().player_database = self.player_database.clone();
        }
        self.base()
            .get_tree()
            .unwrap()
            .get_root()
            .unwrap()
            .add_child(scene);
        // hide multiplayer menu
        self.base_mut().hide();
    }

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
        let mut multiplayer = self.base().get_multiplayer().unwrap();

        multiplayer.set_multiplayer_peer(peer);
        godot_print!("Waiting For Players!");
    }

    #[func]
    fn on_host_button_down(&mut self) {
        self.base_mut()
            .get_node_as::<Button>("JoinButton")
            .set_visible(false);
        self.host_game();
        self.send_player_information(
            self.base().get_node_as::<LineEdit>("UsernameLineEdit").get_text(),
            self.base().get_multiplayer().unwrap().get_unique_id(),
        );
    }

    #[func]
    fn on_join_button_down(&mut self) {
        self.base_mut()
            .get_node_as::<Button>("HostButton")
            .set_visible(false);
        let mut peer = ENetMultiplayerPeer::new_gd();
        let error = peer.create_client(self.address.clone(), self.port);
        if error != Error::OK {
            godot_print!("cannot join");
            return;
        }
        peer.get_host()
            .unwrap()
            .compress(godot::classes::enet_connection::CompressionMode::RANGE_CODER);
        let mut multiplayer = self.base().get_multiplayer().unwrap();

        multiplayer.set_multiplayer_peer(peer);
        godot_print!("Waiting For Server...");
    }

    #[func]
    fn on_start_button_down(&mut self) {
        // https://forum.godotengine.org/t/how-to-fix-trying-to-call-an-rpc-via-a-multiplayer-peer-which-is-not-connected/37037
        // this might fix some weird edge cases
	    // probably just takes a while for the connection to be established?
        thread::sleep(time::Duration::from_secs(1));
        self.base_mut().rpc("start_game".into(), &[]);
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

        let mut multiplayer = self.base().get_multiplayer().unwrap();

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
