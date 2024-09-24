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
    // called when a new "peer" gets connected with the server. Both client and server get notified about this
    #[func]
    fn on_peer_connected(&self, id : i64) {
        godot_print!("Player Connected: {id}");
    }

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
/*
# gets called only on clients
func on_connected_to_server():
	print("Connected to server!")
	# send information to server 
	send_player_information.rpc_id(1, $UsernameLineEdit.text, multiplayer.get_unique_id())

# gets called only on clients
func on_connection_failed():
	print("Connection failed :(")

func host_game():
	peer = ENetMultiplayerPeer.new()
	var error = peer.create_server(port)
	if error != OK:
		print("Cannot host: ", error)
		return
	# TODO: figure out best compression algorithm for packets sent 
	peer.get_host().compress(ENetConnection.COMPRESS_RANGE_CODER)
	
	# set the peer we created as our network peer for the game
	multiplayer.set_multiplayer_peer(peer)
	
	print("Waiting for players...")

# this function should first be called by the player connecting to the server
# and then, the server should call this function on all the other players to propagate the information out
@rpc("any_peer")
func send_player_information(username : String, id : int):
	if !GameManager.players.has(id):
		GameManager.players[id] = {
			"username" : username,
			"id" : id,
			"score" : 0
		}
	
	# server should update all the other players with the new info
	if multiplayer.is_server():
		for peer_id in GameManager.players:
			send_player_information.rpc(GameManager.players[peer_id].username, peer_id)

# make sure that every peer starts the game at the same time
# call local makes it so that it will be called on the person who clicked "start game" as well
@rpc("any_peer", "call_local")
func start_game():
	var scene = load("res://testScene.tscn").instantiate()
	get_tree().root.add_child(scene)
	# hide multiplayer menu
	self.hide()

func _on_host_button_button_down():
	host_game()
	# register host player information
	send_player_information($UsernameLineEdit.text, multiplayer.get_unique_id())

func _on_join_button_button_down():
	peer = ENetMultiplayerPeer.new()
	var error = peer.create_client(address, port)
	# compression has to be same as host
	peer.get_host().compress(ENetConnection.COMPRESS_RANGE_CODER)
	
	# set the peer we created as our network peer for the game
	multiplayer.set_multiplayer_peer(peer)

func _on_start_button_button_down():
	# call as rpc
	start_game.rpc()

func on_packet_recieved(id: int, packet: PackedByteArray):
	print(packet.get_string_from_ascii())
     */
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
            let peer: i32 = args.get(0).unwrap().try_to::<i32>().unwrap();
            Ok(Variant::nil())
        });
        multiplayer.connect("peer_connected".into(), on_peer_connected);
    }

    fn physics_process(&mut self, delta: f64) {

    }
}