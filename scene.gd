extends Node2D

@export var look_speed: float = 2.2
@export var move_speed: float = 6.0
@export var stick_deadzone: float = 0.15
@export var max_pitch_degrees: float = 80.0

var camera: Camera3D
var camera_pitch: float = 0.0

# Axis values updated by _input() from InputEventJoypadMotion events.
var _left_x: float = 0.0
var _left_y: float = 0.0
var _right_x: float = 0.0
var _right_y: float = 0.0


func _axis_with_deadzone(value: float) -> float:
	if abs(value) < stick_deadzone:
		return 0.0
	return value


# Called when the node enters the scene tree for the first time.
func _ready() -> void:
	camera = get_node_or_null("Camera3D")
	if camera:
		camera_pitch = camera.rotation.x


func _input(event: InputEvent) -> void:
	if not event is InputEventJoypadMotion:
		return
	var joy: InputEventJoypadMotion = event as InputEventJoypadMotion
	match joy.axis:
		JOY_AXIS_LEFT_X:  _left_x  = joy.axis_value
		JOY_AXIS_LEFT_Y:  _left_y  = joy.axis_value
		JOY_AXIS_RIGHT_X: _right_x = joy.axis_value
		JOY_AXIS_RIGHT_Y: _right_y = joy.axis_value


# Called every frame. 'delta' is the elapsed time since the previous frame.
func _process(delta: float) -> void:
	if camera == null:
		return

	var left_x: float = _axis_with_deadzone(_left_x)
	var left_y: float = _axis_with_deadzone(_left_y)
	var right_x: float = _axis_with_deadzone(_right_x)
	var right_y: float = _axis_with_deadzone(_right_y)

	# Left stick controls camera position in the horizontal plane (no vertical drift from pitch).
	if left_x != 0.0 or left_y != 0.0:
		var forward: Vector3 = -camera.global_transform.basis.z
		var right: Vector3 = camera.global_transform.basis.x
		var flat_forward: Vector3 = Vector3(forward.x, 0.0, forward.z).normalized()
		var flat_right: Vector3 = Vector3(right.x, 0.0, right.z).normalized()
		var move_direction: Vector3 = flat_right * left_x + flat_forward * -left_y
		camera.global_position += move_direction * move_speed * delta

	# Right stick controls camera direction (yaw and pitch).
	if right_x != 0.0:
		camera.global_rotate(Vector3.UP, -right_x * look_speed * delta)

	if right_y != 0.0:
		var pitch_delta: float = -right_y * look_speed * delta
		var next_pitch: float = clamp(camera_pitch + pitch_delta, deg_to_rad(-max_pitch_degrees), deg_to_rad(max_pitch_degrees))
		camera.rotate_object_local(Vector3.RIGHT, next_pitch - camera_pitch)
		camera_pitch = next_pitch
