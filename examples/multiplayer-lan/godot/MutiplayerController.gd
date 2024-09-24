extends Control

@export var Address = "127.0.0.1"
@export var port = 8910
var peer

# Called when the node enters the scene tree for the first time.
func _ready():
	multiplayer.peer_connected.connect(peer_connected)
	multiplayer.peer_disconnected.connect(peer_disconnected)
	multiplayer.connected_to_server.connect(connected_to_server)
	multiplayer.connection_failed.connect(connection_failed)
	if "--server" in OS.get_cmdline_args():
		hostGame()
	pass # Replace with function body.


# Called every frame. 'delta' is the elapsed time since the previous frame.
func _process(delta):
	pass

# this get called on the server and clients
func peer_connected(id):
	print("Player Connected " + str(id))
	
# this get called on the server and clients
func peer_disconnected(id):
	print("Player Disconnected " + str(id))
	GameManager.remove_player(id)
	var players = get_tree().get_nodes_in_group("Player")
	for i in players:
		if i.peer_id == id:
			i.queue_free()
# called only from clients
func connected_to_server():
	print("connected To Sever!")
	SendPlayerInformation.rpc_id(1, $LineEdit.text, multiplayer.get_unique_id())

# called only from clients
func connection_failed():
	print("Couldnt Connect")

@rpc("any_peer")
func SendPlayerInformation(name : String, id : int):
	GameManager.add_player(id, name, 0)
	
	if multiplayer.is_server():
		for network_id in GameManager.get_list_of_players():
			SendPlayerInformation.rpc(GameManager.get_player(network_id).get("name"), network_id)

@rpc("any_peer","call_local")
func StartGame():
	var scene = load("res://testScene.tscn").instantiate()
	get_tree().root.add_child(scene)
	self.hide()
	
func hostGame():
	peer = ENetMultiplayerPeer.new()
	var error = peer.create_server(port, 2)
	if error != OK:
		print("cannot host: " + error)
		return
	peer.get_host().compress(ENetConnection.COMPRESS_RANGE_CODER)
	
	multiplayer.set_multiplayer_peer(peer)
	print("Waiting For Players!")

func _on_host_button_down():
	hostGame()
	SendPlayerInformation($LineEdit.text, multiplayer.get_unique_id())
	pass # Replace with function body.


func _on_join_button_down():
	peer = ENetMultiplayerPeer.new()
	peer.create_client(Address, port)
	peer.get_host().compress(ENetConnection.COMPRESS_RANGE_CODER)
	multiplayer.set_multiplayer_peer(peer)	
	pass # Replace with function body.


func _on_start_game_button_down():
	StartGame.rpc()
	pass # Replace with function body.
