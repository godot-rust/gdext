extends Node2D

@export var PlayerScene : PackedScene

# Called when the node enters the scene tree for the first time.
func _ready():
	var index = 0
	for network_id in GameManager.get_list_of_players():
		var currentPlayer = PlayerScene.instantiate()
		currentPlayer.peer_id = network_id
		add_child(currentPlayer)
		for spawn in get_tree().get_nodes_in_group("PlayerSpawnPoint"):
			if spawn.name == str(index):
				currentPlayer.global_position = spawn.global_position
		index += 1
	pass # Replace with function body.


# Called every frame. 'delta' is the elapsed time since the previous frame.
func _process(delta):
	pass
