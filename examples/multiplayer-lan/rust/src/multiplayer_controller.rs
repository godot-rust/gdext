use godot::prelude::*;
use godot::classes::{Control, ENetMultiplayerPeer, IControl};

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
    // called when a new "peer" gets disconnected with the server. Both client and server get notified about this
    #[func]
    fn on_peer_disconnected(&self, id : i64) {
	    godot_print!("Player Disconnected: {id}");
	    //godot::classes::Engine::singleton().get_singleton(StringName::from("GameManager")).players.erase(id);
        for player in self.base().get_tree().unwrap().get_nodes_in_group("players".into()).iter_shared(){
            /*
            if player.peer_id == id {
                player.queue_free();
            }
            */
            todo!();
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
        let on_peer_connected = Callable::from_fn("on_peer_connected", |args: &[&Variant]| {
            let peer_id: i32 = args.get(0).unwrap().try_to::<i32>().unwrap();
            godot_print!("Player Connected: {peer_id}");
            Ok(Variant::nil())
        });
        multiplayer.connect("peer_connected".into(), on_peer_connected);
    }

    fn physics_process(&mut self, delta: f64) {

    }
}