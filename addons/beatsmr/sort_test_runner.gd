@tool
extends SortTester

@export_tool_button("Run Sort Tests")
var run_sort_tests_action = _run_sort_tests

func _run_sort_tests() -> void:
	run_tests()
