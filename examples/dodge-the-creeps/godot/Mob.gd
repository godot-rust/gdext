# This .gd file is not used, it just serves as a comparison with the godot-rust implementation.

extends RigidBody2D

func _ready():
	$AnimatedSprite2D.playing = true
	var mob_types = $AnimatedSprite2D.frames.get_animation_names()
	$AnimatedSprite2D.animation = mob_types[randi() % mob_types.size()]


func _on_VisibilityNotifier2D_screen_exited():
	queue_free()
