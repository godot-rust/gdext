use godot::prelude::*;
use godot::classes::{Control, ENetMultiplayerPeer, IControl};

use crate::game_manager::GameManager;
use crate::NetworkId;

const LOCALHOST : &str = "127.0.0.1";
const PORT : u32 = 8910;

#[derive(GodotClass)]
#[class(base=Control)]
pub struct MultiplayerController {
    #[export]
    address: GString,
    port: u32,
    peer: Option<Gd<ENetMultiplayerPeer>>,
    base: Base<Control>
}

#[godot_api] 
impl MultiplayerController {
    #[func]
    fn on_peer_connected(&self, network_id : NetworkId) {
	    godot_print!("Player connected: {network_id}");
    }

    // called when a new "peer" gets disconnected with the server. Both client and server get notified about this
    #[func]
    fn on_peer_disconnected(&self, network_id : NetworkId) {
	    godot_print!("Player Disconnected: {network_id}");
	    if let Some(mut game_manager) = GameManager::get_as_singleton() {
            game_manager.bind_mut().remove_player(network_id);
        }
    }
}

#[godot_api]
impl IControl for MultiplayerController {
    fn init(base: Base<Control>) -> Self {
        Self {
            address: LOCALHOST.into(),
            port: PORT,
            peer: Option::<Gd<ENetMultiplayerPeer>>::None,
            base,
        }
    }

    fn ready(&mut self)
    {
        let mut multiplayer = self.base().get_multiplayer().unwrap();

        // currently callable/signal API is really ugly
        multiplayer.connect("peer_connected".into(), self.base().callable("on_peer_connected"));
        multiplayer.connect("peer_disconnected".into(), self.base().callable("on_peer_disconnected"));
        multiplayer.connect("connected_to_server".into(), self.base().callable("on_connected_to_server"));
        multiplayer.connect("connection_failed".into(), self.base().callable("on_connection_failed"));
    }

    fn physics_process(&mut self, delta: f64) {

    }
}