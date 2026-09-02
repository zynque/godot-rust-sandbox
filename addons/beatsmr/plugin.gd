@tool
extends EditorPlugin


func _enter_tree() -> void:
	var selection := get_editor_interface().get_selection()
	if selection and not selection.selection_changed.is_connected(_on_selection_changed):
		selection.selection_changed.connect(_on_selection_changed)
	_on_selection_changed()


func _exit_tree() -> void:
	var selection := get_editor_interface().get_selection()
	if selection and selection.selection_changed.is_connected(_on_selection_changed):
		selection.selection_changed.disconnect(_on_selection_changed)


func _on_selection_changed() -> void:
	var editor := get_editor_interface()
	if editor == null:
		return

	var root := editor.get_edited_scene_root()
	if root == null:
		return

	var selected_ids := {}
	for node in editor.get_selection().get_selected_nodes():
		selected_ids[node.get_instance_id()] = true

	_apply_selection_state(root, selected_ids)


func _apply_selection_state(node: Node, selected_ids: Dictionary) -> void:
	if node.has_method("set_editor_selected"):
		node.call("set_editor_selected", selected_ids.has(node.get_instance_id()))

	for child in node.get_children():
		if child is Node:
			_apply_selection_state(child, selected_ids)
